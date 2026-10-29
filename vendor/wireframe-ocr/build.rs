// SPDX-License-Identifier: GPL-3.0-only
//! Offline, target-specific static native build. All writes stay in Cargo OUT_DIR.
use flate2::read::GzDecoder;
use sha2::{Digest, Sha256};
use std::{
    env, fs,
    path::{Path, PathBuf},
};

fn checksum(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn unpack(name: &str, hash: &str, out: &Path) -> PathBuf {
    let path = Path::new("native").join(format!("{name}.tar.gz"));
    println!("cargo:rerun-if-changed={}", path.display());
    let bytes = fs::read(path).expect("read vendored OCR source");
    assert_eq!(checksum(&bytes), hash, "OCR source checksum mismatch");
    let destination = out.join("source");
    let source = destination.join(name);
    if !source.is_dir() {
        fs::create_dir_all(&destination).expect("create native source directory");
        tar::Archive::new(GzDecoder::new(bytes.as_slice()))
            .unpack(&destination)
            .expect("extract pinned native OCR source");
    }
    source
}

fn configure(source: &Path, destination: &Path, target: &str) -> cmake::Config {
    let mut config = cmake::Config::new(source);
    config
        .out_dir(destination)
        .profile("Release")
        .define("CMAKE_POSITION_INDEPENDENT_CODE", "ON")
        .define("BUILD_SHARED_LIBS", "OFF")
        .define("CMAKE_INSTALL_LIBDIR", "lib");
    if target.contains("apple-darwin") {
        // Native libraries target macOS 11 for ARM64 and std::filesystem.
        // Wireframe itself retains its separately declared macOS 12 minimum.
        config.define("CMAKE_OSX_DEPLOYMENT_TARGET", "11.0");
        // Use the SDK selected by the active compiler, as package.py does. A
        // different CommandLineTools SDK can contain unsupported linker stubs.
        let sdk = env::var("SDKROOT")
            .ok()
            .or_else(|| {
                std::process::Command::new("xcrun")
                    .args(["--sdk", "macosx", "--show-sdk-path"])
                    .output()
                    .ok()
                    .filter(|output| output.status.success())
                    .and_then(|output| String::from_utf8(output.stdout).ok())
            })
            .expect("select the active macOS SDK using SDKROOT or xcrun");
        config.define("CMAKE_OSX_SYSROOT", sdk.trim());
    }
    if target.contains("msvc") {
        let static_crt = env::var("CARGO_CFG_TARGET_FEATURE")
            .unwrap_or_default()
            .split(',')
            .any(|feature| feature == "crt-static");
        config.define("CMAKE_POLICY_DEFAULT_CMP0091", "NEW").define(
            "CMAKE_MSVC_RUNTIME_LIBRARY",
            if static_crt {
                "MultiThreaded"
            } else {
                "MultiThreadedDLL"
            },
        );
    }
    config
}

fn disable_tiff_runtime_probe(source: &Path) {
    // Leptonica's TIFF codec is disabled below. Upstream still executes a TIFF
    // try_run probe unconditionally; build-only targets must never need it.
    // Preserve the original archive; mark this generated build-input change.
    let file = source.join("CMakeLists.txt");
    let text = fs::read_to_string(&file).expect("read Tesseract CMake input");
    let replacement = include_str!("native/no-tiff-runtime-probe.cmake");
    if text.contains(replacement) {
        return;
    }
    let original = "check_leptonica_tiff_support()";
    assert_eq!(
        text.matches(original).count(),
        1,
        "Tesseract TIFF probe patch no longer applies"
    );
    fs::write(file, text.replacen(original, replacement.trim_end(), 1))
        .expect("disable the unnecessary native TIFF runtime probe");
}

fn find_library(root: &Path, names: &[&str]) -> Option<PathBuf> {
    for entry in fs::read_dir(root)
        .expect("read native library build")
        .flatten()
    {
        let path = entry.path();
        if path.is_file() && names.iter().any(|name| path.file_name().unwrap() == *name) {
            return Some(path);
        }
        if path.is_dir()
            && let Some(path) = find_library(&path, names)
        {
            return Some(path);
        }
    }
    None
}

fn link_library(path: &Path) {
    let mut name = path.file_stem().unwrap().to_str().unwrap();
    if path.extension().is_some_and(|extension| extension == "a") {
        name = name.strip_prefix("lib").unwrap_or(name);
    }
    println!(
        "cargo:rustc-link-search=native={}",
        path.parent().unwrap().display()
    );
    println!("cargo:rustc-link-lib=static={name}");
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=native/no-tiff-runtime-probe.cmake");
    println!("cargo:rerun-if-changed=models/eng.traineddata");
    let model = fs::read("models/eng.traineddata").expect("read embedded English OCR model");
    assert_eq!(
        checksum(&model),
        "7d4322bd2a7749724879683fc3912cb542f19906c83bcc1a52132556427170b2",
        "OCR model checksum mismatch"
    );
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo OUT_DIR"));
    let target = env::var("TARGET").expect("Cargo target");
    let lept = unpack(
        "leptonica-1.87.0",
        "fa2b40c5caea96d1bb93a97486262aed8731b69ce25a84a6bf5d25323e33f631",
        &out,
    );
    let tess = unpack(
        "tesseract-5.5.4",
        "a8dbf8b8c8ee427ff6ad1d99116502feaf59440af1adb893caf60a6576826cf4",
        &out,
    );
    disable_tiff_runtime_probe(&tess);
    let mut config = configure(&lept, &out.join("leptonica"), &target);
    config.define("BUILD_PROG", "OFF").define("SW_BUILD", "OFF");
    // Surface failures through Result rather than native diagnostic output.
    config.cflag(if target.contains("msvc") {
        "/DNO_CONSOLE_IO=1"
    } else {
        "-DNO_CONSOLE_IO=1"
    });
    for codec in ["ZLIB", "PNG", "GIF", "JPEG", "TIFF", "WEBP", "OPENJPEG"] {
        config.define(format!("ENABLE_{codec}"), "OFF");
    }
    let leptonica = config.build();
    let mut config = configure(&tess, &out.join("tesseract"), &target);
    // OCR needs no TIFF-backed debug-caption fonts. This upstream guard avoids
    // trying to decode debug assets when all native file codecs are disabled.
    config.cxxflag(if target.contains("msvc") {
        "/DTESSERACT_DISABLE_DEBUG_FONTS=1"
    } else {
        "-DTESSERACT_DISABLE_DEBUG_FONTS=1"
    });
    config
        .define("Leptonica_DIR", leptonica.join("lib/cmake/leptonica"))
        .define("CMAKE_PREFIX_PATH", &leptonica)
        .define("BUILD_TRAINING_TOOLS", "OFF")
        .define("BUILD_TESTS", "OFF")
        .define("DISABLED_LEGACY_ENGINE", "ON")
        .define("GRAPHICS_DISABLED", "ON")
        .define("DISABLE_ARCHIVE", "ON")
        .define("DISABLE_CURL", "ON")
        .define("DISABLE_TIFF", "ON")
        .define("OPENMP_BUILD", "OFF")
        .define("ENABLE_NATIVE", "OFF")
        .define("ENABLE_LTO", "OFF")
        .define("ENABLE_CCACHE", "OFF")
        .define("INSTALL_CONFIGS", "OFF")
        .build_target("libtesseract");
    let tesseract = config.build();
    link_library(
        &find_library(
            &tesseract.join("build"),
            &["libtesseract.a", "tesseract55.lib", "libtesseract55.a"],
        )
        .expect("Tesseract static library missing"),
    );
    link_library(
        &find_library(
            &leptonica.join("lib"),
            &[
                "libleptonica.a",
                "liblept.a",
                "leptonica-1.87.0.lib",
                "libleptonica-1.87.0.a",
                "leptonica.lib",
            ],
        )
        .expect("Leptonica static library missing"),
    );
    if target.contains("apple-darwin") {
        println!("cargo:rustc-link-lib=c++");
    } else if target.contains("linux") || target.contains("windows-gnu") {
        println!("cargo:rustc-link-lib=stdc++");
    }
    if target.contains("windows") {
        println!("cargo:rustc-link-lib=ws2_32");
    }
}
