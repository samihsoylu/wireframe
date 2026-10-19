# Desktop updates: implementation plan

Status: **optional macOS bridge implemented; release activation pending**. Ordinary builds have no updater or background update checks. Configured development bundles use Sparkle 2.10.0. Windows automatic updating is explicitly later work; manual downloads remain the public distribution plan.

## macOS acceptance tasks

- [x] Integrate Sparkle 2 through a small native bridge, with a user-visible **Check for Updates…** action and a preference for automatic checks. The native menu, missing-feed error, preference persistence and draft retention pass a disposable-bundle check. This does not establish installed upgrades.
- [x] Preserve framework symlinks and helper permissions through staging, ZIP extraction and DMG creation. A real-framework packaging fixture passes entry, permission and signature comparisons; this does not establish updater integration.
- [x] Route direct macOS termination through the draft/save gate. Native cancellation, save and cold reopen pass an isolated AppKit-action check. Updater installation must require this gate to be available; actual Sparkle cancellation/resume and OS shutdown remain unverified.
- [x] Pin and verify the framework download for configured product builds and retain its complete license in the installed app. Fresh bundle staging preserves the previous build on failure.
- [ ] Give Wireframe its own Ed25519 signing identity. Keep its private key in the macOS Keychain or release secret storage, outside Git; retain a recoverable backup. Never reuse another app’s key.
- [ ] Embed the public key and HTTPS appcast URL in the bundle. Maintain increasing build versions and human-readable release versions, beginning with 0.1.0.
- [ ] Publish signed update archives and bullet-point release notes only after validating the exact release files. The stable bundle identifier is `nl.samihsoylu.wireframe`; no public installer has used the earlier local identifier.
- [ ] Verify an actual upgrade between two installed builds, including update refusal for bad signatures, unavailable networks, user cancellation, read-only volumes and unsaved projects. Do not silently discard edits during relaunch.
- [ ] Verify the entire flow without Apple Developer ID signing or notarization, including first-launch behavior and any helper signing requirements. Sparkle signatures authenticate updates; they do not grant Apple notarization or bypass Gatekeeper.

The bridge loads the bundled framework and uses the standard Sparkle controller on the main thread. Use HTTPS GitHub-hosted release files/appcast. Keep the first installation as a drag-to-Applications DMG. The updater must not be advertised as available until its end-to-end check passes. Automatic checks are opt-in and owned by Sparkle preferences; automatic installation is disabled. Build configuration is documented in [packaging](PACKAGING.md).

Official references: [setup and signing](https://sparkle-project.org/documentation/), [programmatic integration](https://sparkle-project.org/documentation/programmatic-setup/). The DMG layout follows the existing [SideTray installer approach](https://github.com/samihsoylu/side-tray), with product-specific text and files.

Current native evidence is scoped to a separate study bundle identity, non-release public test key and a deliberately missing HTTPS feed. It proves truthful error handling and preserving an open Unicode text draft, not archive validation or replacement. Both study processes exited, automatic checks were restored off, and 501 pre-existing working-copy files were unchanged. The menu checkmark was not visually captured; preference persistence was checked on disk and through the subsequent toggle after restart. External evidence: `updater-integration-current`.
