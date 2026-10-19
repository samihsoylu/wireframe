# Desktop updates: implementation plan

Status: **planned, not implemented**. The app currently has no updater. Sparkle is the selected macOS framework; Windows automatic updating is explicitly later work. Manual downloads remain available for every platform.

## macOS acceptance tasks

- [ ] Integrate Sparkle 2 through a small native bridge, with a user-visible **Check for Updates…** action and a preference for automatic checks.
- [ ] Pin and verify the framework download, retain its license, and preserve framework symlinks and helper permissions in packaging.
- [ ] Give Wireframe its own Ed25519 signing identity. Keep its private key in the macOS Keychain or release secret storage, outside Git; retain a recoverable backup. Never reuse another app’s key.
- [ ] Embed the public key and HTTPS appcast URL in the bundle. Maintain increasing build versions and human-readable release versions, beginning with 0.1.0.
- [ ] Publish signed update archives and bullet-point release notes only after validating the exact release files. Decide the stable bundle identifier before the first public installer; current local bundles use `local.wireframe.desktop`.
- [ ] Verify an actual upgrade between two installed builds, including update refusal for bad signatures, unavailable networks, user cancellation, read-only volumes and unsaved projects. Do not silently discard edits during relaunch.
- [ ] Verify the entire flow without Apple Developer ID signing or notarization, including first-launch behavior and any helper signing requirements. Sparkle signatures authenticate updates; they do not grant Apple notarization or bypass Gatekeeper.

Use the standard Sparkle controller on the main thread and HTTPS GitHub-hosted release files/appcast. Keep the first installation as a drag-to-Applications DMG. The updater must not be advertised as available until its end-to-end check passes.

Official references: [setup and signing](https://sparkle-project.org/documentation/), [programmatic integration](https://sparkle-project.org/documentation/programmatic-setup/). The DMG layout follows the existing [SideTray installer approach](https://github.com/samihsoylu/side-tray), with product-specific text and files.
