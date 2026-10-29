# Wireframe local OCR

This crate recognizes printed English text from PNG, JPEG and the first frame of
GIF images. It statically links Tesseract 5.5.4 and Leptonica 1.87.0 and embeds the
official `tessdata_fast` English model. Recognition uses the in-memory C API: no
runtime model download, program installation, cache, temporary image, cloud
request or API key is needed.

`recognize(bytes, cancel)` returns recognized text and an engine confidence
estimate. Empty text is a successful result. Input is limited to 32 MiB and
16 megapixels; transparency is flattened onto white. Recognition has a 30-second
engine deadline and cancellation callback. Decode and initialization are bounded
but synchronous, so applications should call this from a worker thread and set
the cancellation flag when its preview closes. No partial result is returned
after cancellation.

The English model is intended for horizontal printed text. Small, blurred,
rotated or handwritten text and other languages may need correction. Recognition
does not reproduce page layout or turn screenshots into editable wireframes.

Build requirements are Cargo/Rust 1.95+, CMake 3.22+ and a C/C++17 compiler
(Apple Clang on macOS, GCC/Clang on Linux, MSVC on Windows). GNU Make/Ninja or
the platform's CMake generator must be installed. Use native builds for each
target. Supported source paths cover macOS ARM64/x86-64, Linux x86-64 and Windows
x86-64 MSVC; a target build is still required to establish platform acceptance.
Windows `+crt-static` is propagated to native libraries. No Windows runtime
execution is implied. An explicit CMake executable can be supplied with `CMAKE`.

The build verifies SHA-256 for the vendored archives and model before use, then
extracts and builds only under Cargo `OUT_DIR`. Native image codecs, curl,
archive reading, OpenMP, GUI/debug viewer, legacy OCR, tests and training tools
are disabled. Rust decodes images into RGB pixels before passing them to the
engine. `ENABLE_NATIVE=OFF` prevents building a processor-specific executable.
The build needs no network once Cargo dependencies are available. On macOS,
set `SDKROOT` to `xcrun --sdk macosx --show-sdk-path` when the command-line SDK
and active Xcode compiler differ; the existing app packager already does this.

Pinned source URLs, versions, sizes and hashes are in
[PROVENANCE.json](PROVENANCE.json). The source archives are unchanged and retain
their complete upstream copyright/license notices. In generated Cargo OUT_DIR
source, Wireframe replaces the unconditional `check_leptonica_tiff_support()`
call with [known disabled-TIFF results](native/no-tiff-runtime-probe.cmake),
with a prominent modification comment. This avoids CMake `try_run` execution
on build-only targets; TIFF is explicitly disabled in both libraries. No OCR
source or model is modified. Wireframe's wrapper and
build script are GPL-3.0-only; native dependencies retain their own terms:

| Component | Terms | Included notices |
| --- | --- | --- |
| Tesseract 5.5.4 | Apache-2.0 | [LICENSE](native/tesseract-LICENSE), [authors](native/tesseract-AUTHORS), complete pinned source archive |
| Leptonica 1.87.0 | BSD-2-Clause | [license](native/leptonica-LICENSE.txt), per-file notices in complete pinned source archive |
| Official tessdata_fast English model | Apache-2.0 | [LICENSE](models/LICENSE), [upstream README](models/README.md) explicitly licenses all repository data |

The actual model is `models/eng.traineddata` from official repository revision
`87416418657359cb625c412a48b6e1d6d41c29bd`. The upstream license statement applies
to the data itself; it is not inferred from the OCR engine's code license.

Run real-inference checks with:

```sh
CARGO_BUILD_JOBS=2 cargo test --manifest-path vendor/wireframe-ocr/Cargo.toml
```

The synthetic screenshot fixture is rasterized from Wireframe's bundled OFL
Comic Neue font. Regenerate it with `tests/make_fixtures.py` using Pillow 11.3.0;
Pillow is a fixture-authoring tool only, not a build/runtime dependency.
