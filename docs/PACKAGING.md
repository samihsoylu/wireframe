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

Current private macOS ARM64 ZIP/DMG and Debian ARM64 TAR.GZ use runtime/frozen source `1c959c9` (save-error presentation correction). Each contains 1,800 exact corresponding-source files and 266 valid documentation links. Reader-copy links are repaired during staging while corresponding-source bytes remain exact. Notices cover all 368 normal/build dependencies. Artifact identities, failed/passed checks and scoped native evidence are retained in external `appearance-finishing-current/delivery/save-error-correction/REPORT.md`. Later documentation-only commits do not change these frozen artifacts.

Current full application targets pass: 555 macOS tests (three ignored) and 547 Linux tests (none ignored). Unchanged library/integration/example/vendor targets retain their completed evidence; assembled 220-target matrices total 1,798 macOS passes/four ignored and 1,782 Linux passes/one ignored. These combine retained and refreshed targets, not uninterrupted successful full-suite runs. Warnings-denied all-target lint, formatting and optimized builds pass on both. Linux initially lacked cargo-fmt; official rustfmt was temporarily added only in the owned container, its check passed and original tool components were restored exactly. Seven unchanged-input Python configuration/packaging/updater checks on both platforms and five non-Windows archive-integrity checks retain their verified scope. No Windows-specific check was repeated.

The current macOS strict ad-hoc signature, read-only mounted DMG identity for all 50 app entries, companion ZIP and Applications link pass. Linux archive source/mode/safety and dependency resolution pass. Both exact extracted apps verify error-only save-failure presentation, layered Escape cancellation and complete-project rescue/retry after a failed close/Save. Current macOS additionally passes three-type page/image/library Restore, byte-exact one-step saved history/cold Save and minimum-window typed keyboard/partial Restore. Retained predecessor keyboard/import/history/four-scene/cold evidence keeps its unchanged scope; no complete keyboard/IME/accessibility or cross-platform visual equivalence is inferred. Linux uses isolated X11/Mesa software rendering and scaled noVNC capture. Its owned environment is stopped with original limits/context.

The preceding `a8e1dcb` disposable installation remains verified at its original identity: app copied from a read-only DMG to an owned Applications folder, exact entries/signature/process path, fixture open/render/save/exit and all 546 prior data files preserved. No installed user app was replaced. Protected-installation acceptance remains limited by the separately recorded system-authorization boundary; no privilege grant is inferred.

The current `1c959c9` DMG now also has an actual Chrome download/quarantine/native Finder-opening check from a localhost-only HTTP origin. Download bytes match the verified artifact, Finder opens the quarantined read-only image and its mounted app signature/hash pass. **First launch is blocked:** macOS creates a translocated pre-UI process, but system assessment rejects the ad-hoc signature and OS logs report a Gatekeeper prompt awaiting response. Native app control times out; no product fixture/open/save is accepted in that context. The exact owned pre-UI process and server are stopped, browser/Finder study windows closed and all 546 prior product data files exact. Normal ejection is currently denied by `syspolicyd`, which holds the image. The user has been asked to dismiss the prompt with Cancel/Done; the owned image/download remain intact and quarantined pending cleanup. No security warning was bypassed, protection weakened, quarantine removed or installed app replaced. External `appearance-finishing-current/delivery/browser-download/REPORT.md` records the boundary and cleanup state. This is not public HTTPS, fresh-machine or successful first-launch acceptance.

Final permanent image/library removal still awaits action-time approval after successful cancellation checks. Ordinary packages do not activate the updater; no public release, production feed/key or new feature deferral. Windows native testing remains held. Intel/minimum macOS, complete Gatekeeper/installed-launch acceptance, protected installation, other Linux architectures/distributions, Wayland, physical Linux GPUs, native IME and screen-reader acceptance remain open. See [current work](../SESSION-SPEC.md) and [release gates](RELEASE-CHECKLIST.md).

## Application artwork

The original application icon lives in `assets/app-icon`, with editable vector sources and generated PNG, ICNS and ICO files. macOS bundles declare and include the ICNS; the desktop viewport uses the PNG. Ordinary builds use the committed outputs. See the [artwork regeneration instructions](../assets/app-icon/README.md). Native Finder appearance and app startup pass on macOS; Windows executable-resource embedding and Linux native appearance remain open.
