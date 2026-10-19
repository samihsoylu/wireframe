# Desktop updates: implementation plan

Status: **optional macOS bridge implemented; release activation pending**. Ordinary builds have no updater or background update checks. Configured development bundles use Sparkle 2.10.0. Windows automatic updating is explicitly later work; manual downloads remain the public distribution plan.

## macOS acceptance tasks

- [x] Integrate Sparkle 2 through a small native bridge, with a user-visible **Check for Updates…** action and a preference for automatic checks. The native menu, missing-feed error, preference persistence and draft retention pass a disposable-bundle check. Installed-upgrade evidence is recorded below.
- [x] Preserve framework symlinks and helper permissions through staging, ZIP extraction and DMG creation. A real-framework packaging fixture passes entry, permission and signature comparisons; this does not establish updater integration.
- [x] Route direct macOS termination through the draft/save gate. Native cancellation, save and cold reopen pass an isolated AppKit-action check. Updater installation must require this gate to be available; Actual Sparkle cancellation, retry and Save/install now pass the isolated upgrade study; OS shutdown remains unverified.
- [x] Pin and verify the framework download for configured product builds and retain its complete license in the installed app. Fresh bundle staging preserves the previous build on failure.
- [ ] Finish verification of the dedicated Wireframe Ed25519 identity. A distinct key and restricted local backup exist outside Git. Keychain-backed backup verification awaits the macOS authorization prompt; it has not passed yet. Never reuse another app’s key.
- [ ] Embed the public key and HTTPS appcast URL in the bundle. Maintain increasing build versions and human-readable release versions, beginning with 0.1.0.
- [ ] Publish signed update archives and bullet-point release notes only after validating the exact release files. The stable bundle identifier is `nl.samihsoylu.wireframe`; no public installer has used the earlier local identifier.
- [x] Verify isolated installed upgrades with a temporary key: reject bad archive signatures and tampered signed feeds; report unavailable networks; retain unsaved Unicode text through cancellation, retry and Save/install; replace the app and relaunch it. This uses loopback HTTP with a study-only feed override, not the production HTTPS service.
- [ ] Reopen the active saved project after update relaunch. The saved bytes survive, but the current app starts a blank workspace; manual reopen succeeds. Respect explicit Open requests and unrelated recovery offers.
- [ ] Verify production HTTPS delivery, read-only-volume behavior and remaining first-launch cases before activating release updates.
- [ ] Verify the entire flow without Apple Developer ID signing or notarization, including first-launch behavior and any helper signing requirements. Sparkle signatures authenticate updates; they do not grant Apple notarization or bypass Gatekeeper.

The bridge loads the bundled framework and uses the standard Sparkle controller on the main thread. Use HTTPS GitHub-hosted release files/appcast. Keep the first installation as a drag-to-Applications DMG. The updater must not be advertised as available until its end-to-end check passes. Automatic checks are opt-in and owned by Sparkle preferences; automatic installation is disabled. Configured bundles require signed feeds, reject signature failures immediately, and verify archives before extraction. Build configuration is documented in [packaging](PACKAGING.md).

Official references: [setup and signing](https://sparkle-project.org/documentation/), [programmatic integration](https://sparkle-project.org/documentation/programmatic-setup/). The DMG layout follows the existing [SideTray installer approach](https://github.com/samihsoylu/side-tray), with product-specific text and files.

The initial bridge study verified the missing-feed error, preference persistence and draft retention. Its menu checkmark was not visually captured. External evidence: `updater-integration-current`.

The installed-upgrade study uses a separate bundle identity and disposable signing key. Two ad-hoc signed DMG upgrades preserve Unicode edits; the second independently observes the new process before any UI query, establishing actual relaunch. Cancel retains the running old build and edit, and retry succeeds. Incorrect archive and feed signatures are rejected, and an unavailable local server reports an error. Installed build 3 matches the staged bundle entry-for-entry, including permissions and links, and passes strict signature verification. The app reopens the saved project correctly when requested manually, but does not yet reopen it automatically after updating.

All 504 pre-existing working-copy files remain byte-identical, the original save preference is restored, and the study app and local server exited. The public product version remains 0.1.0; study versions are disposable fixture metadata. No test release was published. These checks do not establish production HTTPS, downloaded-app Gatekeeper behavior, read-only installation, OS shutdown or Windows acceptance. External evidence: `updater-signing-current`.
