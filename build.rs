// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors

fn main() {
    println!("cargo:rerun-if-changed=assets/app-icon/windows.rc");
    println!("cargo:rerun-if-changed=assets/app-icon/Wireframe.ico");
    // Build scripts run on the host; select resources using the actual target.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        embed_resource::compile_for(
            "assets/app-icon/windows.rc",
            ["wireframe"],
            embed_resource::ParamsIncludeDirs(["assets/app-icon"]),
        )
        .manifest_required()
        .expect("Windows application icon resource must compile and link");
    }
}
