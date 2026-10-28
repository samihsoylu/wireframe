# Package Wireframe

Version 0.1.0 is the first planned release. Packaging creates local artifacts; it does not publish a release or claim native acceptance. For installation, see the [user guide](INSTALL.md).

## Build packages

```sh
python3 scripts/generate_notices.py
python3 scripts/package.py
```

The default is an optimized native build. Add `--debug` for a development build or `--no-archive` to refresh only the local bundle. The script resolves Cargo’s target directory, including `CARGO_TARGET_DIR`. Use `--target` explicitly for Windows cross-compilation; implicit `CARGO_BUILD_TARGET` is rejected.

| Platform | Output | Contents |
|---|---|---|
| macOS | Architecture-specific `.dmg` and companion `.zip` | App, Applications shortcut, arrow/instructions, corresponding source and licenses. |
| Windows | Architecture-specific portable `.exe` and companion `.zip` | Executable; ZIP includes executable, corresponding source, assets and licenses. |
| Linux | Architecture-specific `.tar.gz` | Executable, corresponding source, assets and licenses. |

Every artifact gets a SHA-256 sidecar. Publish the Windows companion ZIP alongside the EXE; do not distribute the EXE alone without corresponding source and notices. Architecture labels describe the actual build target, not a promise that other architectures were tested.

macOS packaging needs Xcode Command Line Tools, Python 3 and the pinned Python dependencies in `scripts/dmg-requirements.txt`, installed in the ignored `.tooling/dmg` environment. It uses `hdiutil` and writes Finder layout metadata directly; it does not automate Finder or alter user settings. The background is drawn at 2× resolution. The app is ad-hoc signed, without Developer ID signing or notarization. The output DMG is never overwritten implicitly: move an earlier local artifact aside before rebuilding the same filename. The packager checks this before replacing the app or companion ZIP. Archive creation is atomic: a failed attempt leaves an earlier archive intact. The standalone DMG wrapper explicitly selects the active compiler’s matching macOS SDK.

To wrap an already verified app and its matching source package without recompiling:

```sh
sh scripts/package-dmg.sh /path/Wireframe.app dist/Wireframe-0.1.0-darwin-arm64.dmg /path/Wireframe-0.1.0-darwin-arm64.zip
```

The wrapper checks app file bytes, entry types, link targets and permissions against the ZIP and requires corresponding source and license entries. It preserves the app signature and keeps the source ZIP inside **Source and licenses** on the image. This identity check relies on the archive’s prior source/build verification; it does not itself prove a reproducible build. Staging preserves links. macOS ZIPs use `ditto`, including framework directory aliases, helper executable permissions and resource metadata. Verification refuses broken, absolute or out-of-bundle app links. Linux TAR.GZ retains link metadata; Windows ZIPs reject links rather than silently replacing them with target contents.

Windows targets accepted by the packager are `x86_64-pc-windows-gnu`, `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc`. Install the matching Rust target and compiler/linker first. Cross-compilation does not close the held native Windows acceptance requirement.

## Optional macOS updater build

Ordinary builds have no updater menu, framework or automatic network checks. Once the signing/feed setup and installed-upgrade acceptance are complete, pass `--updater-config /path/public-update-config.json` to `scripts/package.py`. The JSON accepts exactly three string fields: `feed_url` (HTTPS), `public_key` (the product's 32-byte Ed25519 public key encoded as base64), and `build` (a positive, monotonically increasing integer). Never put a private key in this file.

The stable bundle identifier is `nl.samihsoylu.wireframe`. Display version remains 0.1.0; updater builds use the separate integer for `CFBundleVersion`. The packager verifies the pinned Sparkle 2.10.0 archive, extracts it afresh, preserves framework links and copies its complete license into the app. It stages and signs a fresh bundle before replacing the previous generated bundle. A failed download, copy or signature check retains the previous bundle. A subsequent ordinary build removes stale updater files by replacing the bundle completely.

Configured builds use Sparkle's standard Check for Updates action and an opt-in automatic-check preference in the application menu. Automatic installation is disabled. Signed feeds are mandatory with immediate signature-failure rejection; archives are verified before extraction. The bridge starts only when the native draft/save termination gate is available. This packaging option is for development acceptance until the remaining [update checks](UPDATES.md) pass; no production feed or signing identity has been published.

## Packaging checks

Run `python3 scripts/test_sparkle.py` for public configuration validation, corrupted framework-cache refusal and previous-bundle preservation. Run `python3 scripts/test_packaging.py` for archive round trips, altered-entry rejection and protection of existing artifacts. Native ZIP extraction checks require macOS. These checks do not build or execute a Windows application.

Eight tests pass on macOS with Python 3.9 and 3.13. A separate fixture containing Sparkle 2.10.0 preserves all 191 app entries, including nine links and six executable regular files, through native ZIP extraction and mounted DMG comparison; strict ad-hoc signature checks pass after both. The companion archive contains 1,774 exact source files from that checkpoint. This was a packaging fixture only: no updater was linked, launched or installed. The earlier framework-free review archive also passes the revised verifier. External evidence: `updater-packaging-current`.

## Release procedure

1. Finish the applicable [release gates](RELEASE-CHECKLIST.md). Keep held and unverified platforms explicit.
2. Build from a recorded commit and clean source tree. Check that the version is correct; the first release is `0.1.0`, tagged `v0.1.0`.
3. Verify archives, app signatures, matching source, notices, architecture and actual native workflows. Open the DMG and inspect the arrow, app, Applications shortcut and text. Test installation on a disposable destination without replacing user copies.
4. Prepare release notes from [CHANGELOG.md](../CHANGELOG.md) using the [release template](../.github/RELEASE_TEMPLATE.md). **Every release must contain bullet-point changes**, downloadable files, tested systems and known limitations.
5. Attach the DMG, Windows EXE and companion ZIP, appropriate Linux archive(s), and a combined `SHA256SUMS`. Do not label an untested executable as verified or silently omit the Windows hold.
6. Publish only after the candidate is ready. Update the README’s availability statement with the actual downloads. Integrate signed appcast publication after the [Sparkle tasks](UPDATES.md) are implemented and accepted.

## Current verification scope

Current private macOS ARM64 ZIP/DMG and Linux ARM64 TAR use clean frozen
source/runtime `97e7a99` (source-root navigation correction). All 559 macOS
application checks (three ignored), 551 Linux checks, both lint/format/optimized
builds and source/archive checks pass. Each package contains 1,800 exact source
files and 266 valid links, reader assets/licenses/notices and valid platform modes.
Strict Mac signature, 50 mounted/extracted app entries, companion ZIP and Linux
safety/dependencies pass; the verification image is detached. Extracted Mac
cold/source navigation/Save/empty history and Linux full source navigation/saved
Group history/cold routes pass. All study apps/infrastructure are stopped,
original components/limits/context and all 546 host files/settings are exact.
Private `delivery/reference-source-root/review/REPORT.md` retains hashes, scope and
excluded setup/menu attempts. Prior vendor/packaging checks retain unchanged
inputs; no new uninterrupted full matrix or publication. Other platform/native
and installation/updater gates remain open; Windows held.

The following archives retain their predecessor identity.

Retained private macOS ARM64 ZIP/DMG and Debian ARM64 TAR.GZ use frozen clean
source/runtime `521d988` (rapid slider/input ownership correction). Both application
targets (559 macOS passes/three ignored, 551 Linux passes), 51 egui library tests
per platform, all-target warnings-denied lint, root/vendor formatting and optimized
builds pass. Each archive contains 1,800 exact source files and 266 valid links;
reader assets/licenses/notices are byte-exact with valid platform modes. Linux
normalizes `licenses/manifest.json` from 0664 to 0644 without changing its bytes.
Strict macOS signature, all 50 mounted/extracted app entries, companion ZIP and
Applications link pass; the verification image is detached. Linux archive safety
and dependency resolution pass. Both current extracted platforms now pass their
named native rapid click/key, canvas ownership, saved drag Undo/Redo and cold
Save/empty-history routes. The macOS drag uses the previously authorized guarded
helper on a new named owned fixture; input exclusions and exact full-state
comparisons are retained in `delivery/rapid-slider-input/native-macos/REPORT.md`.
The same macOS package also passes the named approximately 220-point Arrow
inspector click/key, wrapping Unicode/page-link caption, stable live width, saved
Undo/Redo and cold Save/empty-history sequence. Cold width returns to 300 points;
no persistence acceptance (`delivery/macos-inspector-width/REPORT.md`).
Old idle/current macOS studies quit, original save/viewport settings and all 546
prior data files are exact. Owned Linux infrastructure is stopped with original
components/limits/context. Broader native/physical/platform gates remain open. External `appearance-finishing-current/delivery/rapid-slider-input/REPORT.md`
pins identities, failed/passed checks and scope. Source is frozen at the clean
correction commit; later evidence-only docs do not alter those artifacts. No new
full 220-target run, downloaded/installed-launch or publication acceptance.

Predecessor private macOS ARM64 ZIP/DMG and Debian ARM64 TAR.GZ use frozen source/runtime `f944e37` (Trash arrow-focus correction). Each contains 1,800 exact corresponding-source files and 266 valid documentation links. Reader-copy links are repaired during staging while corresponding-source bytes remain exact. All 368 normal/build dependencies have preserved notices; reader licenses/assets and all checksum sidecars match. Source/artifact hashes, failed/passed checks and scope are retained in external `appearance-finishing-current/delivery/overflow-trash/review/REPORT.md`. Later evidence-only documentation commits do not alter these artifacts.

Full application targets pass: 557 macOS tests (three existing ignored) and 549 Linux tests (none ignored). Unchanged library/integration/example/vendor targets retain completed evidence; assembled 220-target matrices total 1,800 macOS passes/four ignored and 1,784 Linux passes/one ignored. These combine retained and refreshed targets, not uninterrupted full-suite runs. Both warnings-denied all-target lint, formatting and optimized builds pass. Linux temporarily adds official rustfmt only inside the owned container, then restores its exact original component set. Seven unchanged-input Python configuration/packaging/updater checks on each platform and five extra non-Windows archive-integrity checks retain their original scope; actual current artifacts are checked directly. No Windows-specific check is repeated.

Current macOS strict ad-hoc signature, read-only mounted DMG identity for all 50 app entries, companion ZIP and Applications link pass; this verification image is detached. The complete extracted app matches the corrected native study in all entry types, modes, links and bytes, so its minimum-window 46-card focus/scroll/footer Restore and exact saved Undo/Redo evidence applies to that exact app without another native run. Linux archive source/mode/safety and dependency resolution pass. Its extracted package also passes native keyboard scrolling, last-page-only Restore with the surviving page current, exact saved Undo/Redo and cold Save/empty history. The complete saved partial file matches macOS bytes. A further current Linux native local-project/library Keep Both replay verifies dependent Landing/Contact review, exact cancellation/import/history/cold Save and repeat no-op. Its whole result matches retained macOS bytes after six canonical-path identity substitutions, and both source files/ownership remain unchanged. Private `appearance-finishing-current/delivery/linux-library-conflict/REPORT.md` records this unchanged-package scope. Current extracted Linux native foreign-page Copy/Paste also passes unchanged repeat/no empty action, changed conflict Cancel/Keep Both, exact saved Undo/Redo and cold Save/empty history. All nine complete files match predeclared retained macOS output after two canonical-origin substitutions; source files stay exact. Its separate `delivery/linux-page-clipboard/REPORT.md` records editable-open ownership, CLI setup and remote-input limits; no artifact or runtime changes. Linux uses isolated X11/Mesa and scaled noVNC; minimum-window observation is screenshot/clamp based. A dead VNC server during native resizing is restarted while the same product process remains live. All accepted scopes and exclusions are retained; no physical-display/IME/screen-reader equivalence is inferred.

Predecessor browser/error/scenario and disposable-installation evidence keeps its named unchanged-source/artifact scope. No installed user app was replaced. The prior `1c959c9` DMG has an actual Chrome localhost HTTP download/quarantine/native Finder-opening check with matching bytes, but first launch is blocked at Gatekeeper. Its exact pre-UI app/server are stopped and all 546 prior product files exact; `syspolicyd` holds that old image pending user prompt dismissal/normal ejection. The user handoff and quarantined download/image remain intact. No warning was bypassed, protection weakened or quarantine removed. See external `appearance-finishing-current/delivery/browser-download/REPORT.md`. This is not current `f944e37` downloaded launch, public HTTPS, fresh-machine or successful first-launch acceptance. Protected installation retains its separate system-authorization boundary.

Current Linux apps and viewer/forward/container/VM are stopped, isolated evidence archived and original limits/tool components/global context preserved; all 546 prior macOS data files remain exact. Separate old Gatekeeper image cleanup is pending. Final permanent image/library removal still awaits action-time approval after cancellation checks. Ordinary packages do not activate the updater; no public release, production feed/key or new feature deferral. Windows testing stays held. Intel/minimum macOS, downloaded/installed launch, protected installation, other Linux architectures/distributions/Wayland/physical GPUs, native IME and screen-reader acceptance remain open. See [current work](../SESSION-SPEC.md) and [release gates](RELEASE-CHECKLIST.md).

The same extracted `f944e37` Linux package now also passes the sampled narrow
Arrow inspector sequence: fractional opacity, wrapped caption/stable page link,
pixel-exact panel width and saved caption Undo/Redo/cold Save with empty history.
It returns to default inspector width after restart; width persistence is not
accepted. Initial stale desktop-lock and remote zero-duration-click failures,
measurement corrections and the accepted 100 ms input scope are retained in
external `appearance-finishing-current/delivery/linux-inspector-width/REPORT.md`.
No runtime/artifact change or broad physical/accessibility acceptance; apps and
owned environment are stopped with original limits/context and host data exact.

## Application artwork

The original application icon lives in `assets/app-icon`, with editable vector sources and generated PNG, ICNS and ICO files. macOS bundles declare and include the ICNS; the desktop viewport uses the PNG. Ordinary builds use the committed outputs. See the [artwork regeneration instructions](../assets/app-icon/README.md). Native Finder appearance and app startup pass on macOS; Windows executable-resource embedding and Linux native appearance remain open.
