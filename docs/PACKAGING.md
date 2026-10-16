# Native packaging

Run `python3 scripts/package.py` to build a release archive, or add `--debug` for a development archive. The script resolves Cargo’s target directory through its metadata, including `CARGO_TARGET_DIR`. Packages contain the executable, original product source and build scripts, assets, documentation, GPL-3.0-only license and dependency notices. Generate notices first with `python3 scripts/generate_notices.py` when needed. Cross-compilation through a Cargo target triple is not currently supported by this script; package on the intended platform.

## Checked Linux environment

An extracted debug archive runs on Debian 12 aarch64 with X11, Openbox, Mesa llvmpipe and a private D-Bus session. X11 runtime support includes `libxkbcommon-x11-0`; this library is loaded dynamically and is not listed by a basic executable `ldd` check. OpenGL and the desktop portal also need working runtime installations. File dialogs were exercised with `xdg-desktop-portal` and `xdg-desktop-portal-gtk`.

The four-page sample project opens and renders. A held library drag displays a preview before release; no control enters the saved document until release. Sampled pointer movement updates the preview. Insertion, paced text editing, undo/redo, page switching, Save As through the portal, normal close and reopen were exercised. The initial run exposed trailing input loss on focus change and stale Save snapshots. Follow-up input routing fixes pass deterministic tests and native Linux replays for immediate canvas clicks, keyboard/toolbar Save and continued typing after Save. See [text entry](TEXT-ENTRY.md) for the scope and remaining gaps. These checks do not establish broad input reliability or release readiness.

The tested display is virtual X11 with software OpenGL. Physical GPUs, Wayland, x86-64, other distributions, Windows, release archives and broad runtime dependency coverage remain unverified. Screenshot samples establish held preview behavior, not smooth frame cadence. macOS packaging uses an ad-hoc signature; it does not provide Developer ID signing or notarization.
