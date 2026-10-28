# Package Wireframe

[Version 0.1.0-beta.2](https://github.com/samihsoylu/wireframe/releases/tag/v0.1.0-beta.2) is published as a GitHub prerelease from frozen source `ccca24b929c42474c2d3e85cf3e47c39368ccbd7`. Its Mac ARM64 DMG/ZIP and unsigned Windows x86-64 EXE/ZIP have the scoped checks below; Windows runtime testing remains held and the EXE is **UNTESTED on Windows**. Packaging commands create local artifacts and do not publish or establish native acceptance. For installation, see the [user guide](INSTALL.md).

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

The stable bundle identifier is `nl.samihsoylu.wireframe`. Beta 2 has package/release version `0.1.0-beta.2`, Mac display version `0.1.0` and bundle build `2`; updater builds use the separate increasing integer for `CFBundleVersion`. The packager verifies the pinned Sparkle 2.10.0 archive, extracts it afresh, preserves framework links and copies its complete license into the app. It stages and signs a fresh bundle before replacing the previous generated bundle. A failed download, copy or signature check retains the previous bundle. A subsequent ordinary build removes stale updater files by replacing the bundle completely.

Configured builds use Sparkle's standard Check for Updates action and an opt-in automatic-check preference in the application menu. Automatic installation is disabled. Signed feeds are mandatory with immediate signature-failure rejection; archives are verified before extraction. The bridge starts only when the native draft/save termination gate is available. This packaging option is for development acceptance until the remaining [update checks](UPDATES.md) pass; no production feed or signing identity has been published. Both betas have no updater keys/framework and use manual downloads. Dedicated Keychain-backed identity verification and production HTTPS/first-launch checks remain pending; do not bypass the authorization prompt using an exported backup. The user declined Apple Developer membership; Developer ID signing/notarization remain absent.

## Packaging checks

Run `python3 scripts/test_sparkle.py` for public configuration validation, corrupted framework-cache refusal and previous-bundle preservation. Run `python3 scripts/test_packaging.py` for archive round trips, altered-entry rejection and protection of existing artifacts. Native ZIP extraction checks require macOS. These checks do not build or execute a Windows application.

Eight tests pass on macOS with Python 3.9 and 3.13. A separate fixture containing Sparkle 2.10.0 preserves all 191 app entries, including nine links and six executable regular files, through native ZIP extraction and mounted DMG comparison; strict ad-hoc signature checks pass after both. The companion archive contains 1,774 exact source files from that checkpoint. This was a packaging fixture only: no updater was linked, launched or installed. The earlier framework-free review archive also passes the revised verifier. External evidence: `updater-packaging-current`.

## Release procedure

1. Finish the applicable [release gates](RELEASE-CHECKLIST.md). Record any explicitly authorized prerelease exception and keep every remaining gate and platform limit open. The beta publication authorization supersedes earlier publication and Windows-build holds only for these prereleases; it did not authorize Windows runtime tests or stable-release acceptance.
2. Build from a recorded commit and clean source tree. Beta 2 is `0.1.0-beta.2`, tag `v0.1.0-beta.2`, exact source `ccca24b`. Preserve both immutable beta tags and their accepted artifacts. Later documentation is not repackaged into either. Stable `0.1.0` remains a separate unaccepted release.
3. Verify archives, app signatures, matching source, notices, architecture and actual native workflows. Open the DMG and inspect the arrow, app, Applications shortcut and text. Test installation on a disposable destination without replacing user copies.
4. Prepare release notes from [CHANGELOG.md](../CHANGELOG.md) using the [release template](../.github/RELEASE_TEMPLATE.md). **Every release must contain bullet-point changes**, downloadable files, tested systems and known limitations.
5. Attach architecture-labelled DMG/EXE and companion source/license ZIPs, appropriate Linux archives as available, and combined `SHA256SUMS`. Beta 2 has Mac ARM64 and Windows x86-64 packages plus `demo.wireframe` and `DEMO-LICENSES.md`; it includes no Linux release artifact. Do not label a build-only executable as runtime-verified or omit the Windows hold.
6. Publish only within the authorized scope after artifact checks. Independently download public files and compare accepted bytes, sizes and checksums; confirm the tag resolves to the candidate. Update user-facing availability without implying older archive documentation changed. Activate signed appcast delivery only after the remaining [Sparkle tasks](UPDATES.md) and security handoffs pass; ordinary beta publication does not activate it.

## Current verification scope

**Published beta 2 — `ccca24b`, `v0.1.0-beta.2`:** [the prerelease](https://github.com/samihsoylu/wireframe/releases/tag/v0.1.0-beta.2) was published on 28 October 2026 at 23:34:17 UTC. All seven unauthenticated public downloads equal the accepted local bytes, sizes and SHA-256; tag and release body match. Private `delivery/beta-0.1.0-beta.2/REPORT.md` and `public-download-verification.json` record delivery.

- **Mac validation/native:** 571 application tests (three existing ignored), five row-selection domain tests, warnings-denied all-target lint, formatting and optimized build pass. The mixed Selection correction has failing-before proof, three UI-event tests/17 cases and independent review. Nine native complete saved comparisons cover mixed/common feedback, actual pending Redo replay after popup dismissal, deliberate choice, one-step Undo/Redo and cold common state. Native popup option AX flags remain unobserved; visual feedback and separate egui/AccessKit tests support their respective scopes. Processes exit0; all546original host files/settings and50bundle entries/signature exact.
- **Mac packages:** ZIP8405safeuniqueCRC entries,1808packaged source files/1810frozen tracked files,761readerasset/license files,101reader documents/554local links and395notices pass independent checks. Frozen modes match the established copy policy: top-level notices normalize0664→0644; licenses/manifest retains frozen0664 versus Git0644. All50app entries equal the native study. Strict signature, numeric0.1.0/build2 and no updater pass. Read-only DMG app/companion/notices/install text/Applications alias and actual Finder arrow view pass; normal detach0. No installation/downloaded-first-launch acceptance.
- **Windows build/static:** exact-SHA build-only run40785930550 succeeds. Standalone/archived EXE and2675safeuniqueCRC entries,1808source files,395notices, PE32+/AMD64/GUI/import checks pass. **No Windows execution, runtime testing or icon-resource acceptance.** Evidence: private `delivery/beta-windows/ccca24b-run-40785930550/SESSION.md`.

Beta 2 refreshes only affected source/native/package evidence. Earlier styles/demo/drag and broader matrices retain their named source scopes; they were not all replayed. Beta 1 and its known mixed-caption observation below remain immutable historical evidence. All broader limits following that checkpoint still apply.

### Retained beta 1 verification

**Published beta 1 — `ca4d48e`, `v0.1.0-beta.1`:** [the prerelease](https://github.com/samihsoylu/wireframe/releases/tag/v0.1.0-beta.1) was published on 28 October 2026 at 22:36:50 UTC. All seven assets (Mac ARM64 DMG/ZIP, Windows x86-64 EXE/ZIP, demo, demo license notes and `SHA256SUMS`) have unauthenticated public downloads equal to the accepted local bytes, sizes and SHA-256. The tag resolves to the exact frozen source. Documentation follow-ups are separate from those archives. Private `delivery/beta-0.1.0-beta.1/public-download-verification.json` records each identity.

- **Mac source/archive/notices:** 8,400 ZIP entries pass CRC and safe/unique-path checks. All 1,807 packaged source paths/bytes match the 1,809-file frozen manifest, excluding only `.gitignore` and `.gitattributes` under the existing package contract. Extraction preserves archived modes. The frozen manifest has no modes: historical filesystem-mode equality is not claimed. Against Git modes, `licenses/manifest.json` is archived `0664` rather than `0644`, matching its checked source filesystem. `THIRD_PARTY_NOTICES.md` is archived `0644` rather than the later source filesystem `0664`, consistent with fresh-file copying and Git mode. These distinctions were recorded without changing source or archive permissions. All 761 reader asset/license files, 101 relocated reader Markdown documents, 549 local reader links and 395 dependency notice records/files pass their recorded checks.
- **Mac app/native/DMG:** the full 50-entry app tree equals the native study in bytes, entry types, modes and links; strict ad-hoc signature and arm64 identity pass. Mac short version is `0.1.0`, build `1`; the minimum-OS plist value `12.0` is metadata, not minimum-OS test acceptance. Scoped native style/Undo/Redo, shared-inspector ordinary two-control drag/history, fresh four-page example, eight both-style scene reviews, headline edit/history/cold Save and five presentation routes pass. The read-only DMG carries the exact app, companion ZIP, license/notices and install instructions; native Finder arrow/caption/layout and serialized positions pass. Normal detach succeeds. Both owned app processes exit normally and all 546 original host data/settings files are exact.
- **Windows build/static only:** the pinned MSVC x86-64 build-only job succeeds at the same source. The unsigned PE32+ AMD64 GUI image has no certificate table. Its standalone EXE equals the EXE in the 2,674-entry ZIP; CRC/safe-path/no-symlink, all 1,807 corresponding-source files and all 395 dependency notices pass. Local inspection and CI reports agree after normalizing case-tied DLL-name ordering; independent `file`/`objdump` checks confirm the image/import identity. **No application execution, Windows runtime/native tests or icon-resource acceptance occurred.** This is the explicitly authorized **UNTESTED on Windows** beta exception, not Windows platform acceptance.

Exact Mac evidence is in private `delivery/beta-0.1.0-beta.1/REPORT.md`, `macos-verification/REPORT.md`, `macos-verification/verification.json` and `dmg-verification.json`; Windows build/static evidence is in `delivery/beta-windows/ca4d48e-run-40777955451`. Final Mac application/lint/format/optimized-build checks pass in their named scope; older broader matrices are not claimed as repeated. Mac source-mode distinctions, excluded native attempts and cleanup guards remain recorded there.

Downloaded first launch, installation/protected destinations, other Mac architectures/versions, wider input/accessibility/failure variants, Windows runtime and production updater activation remain unverified/held. No Apple Developer membership, Developer ID signing or notarization was added. Updates are manual. All 249 retained requirement keys keep their classifications and limits. Q5/Q7/Q8 and prior permission/security handoffs remain open in their recorded scope; existing-format import is the only explicit feature deferral. The beta does not close the original V01–V05/Q3–Q8 goal or authorize a stable release/main merge.

### Retained private verification checkpoints

The records below preserve their original source/artifact/check identities. “Current,” “unpublished” and platform-hold wording in these records describes that historical checkpoint; it is not the published beta-1 disposition above. There is no new beta-1 Linux package or native Linux replay.

Retained private macOS ARM64 ZIP/DMG and Linux AArch64 TAR use frozen source `98b1b20`, runtime `66aee81` (move release/cancellation and plain interruption feedback), and unchanged packaging `15003e0`. Each archive verifies 1,802 exact corresponding-source files/modes, 761 reader asset/license files and 547 valid local reader-file targets. Mac strict signature, 50 app entries/17 immutable Mach-O sections and readonly DMG companion/notices/serialized Finder layout pass; the own verification image detaches normally. Linux safe TAR extraction, ELF64 AArch64/0755 identity and dynamic dependencies pass.

Affected runtime validation passes on Mac (21 related domain/562 application checks, three ignored) and Linux (21 related/554 application), both warnings-denied all-target lint and optimized builds, and shared formatting. These are named affected-target matrices; previous broader matrices retain their original inputs. The extracted Mac app equals all50entries of its native study and reuses five whole-file ordinary drag/history/cold assertions. The exact executed Linux package passes the same five whole-file states, with all3,644entries unchanged after execution. Focus-loss cancellation/Option/forced fallback remain deterministic production-order test evidence; no new native held variants are claimed.

All study apps/infrastructure are stopped, original limits/components/context/config and all546host files/settings are exact. Previous three65d0292archive hashes are unchanged. Private `delivery/move-interruption-current/REPORT.md` and `delivery/move-interruption-review/REPORT.md` retain source identities, artifact hashes, exclusions and cleanup. Earlier Image exports and Debian12/X11/Openbox icon acceptance retain65d0292scope. Downloaded first launch, protected installation, production updater activation and broader native/platform/desktop gates remain open; Windows testing/resource work wholly held, no publication/new deferral. The old Gatekeeper path is absent at the final read; no security action or completion of that prior handoff is claimed.

Retained private macOS ARM64 ZIP/DMG and Linux ARM64 TAR use clean frozen
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
