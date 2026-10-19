# Wireframe — current state

## Outcome and boundaries

Deliver a usable local Rust desktop wireframing app for macOS, Windows and Linux, with the fixed [G01–G17 contract](docs/RELEASE-CHECKLIST.md), four real-world workflows and a final functional/visual comparison. The goal is active and incomplete. Minor documented differences may remain; missing functionality, data loss and material discrepancies stay open. Research must answer a named remaining question; reuse unchanged evidence and rerun affected checks after fixes.

Single Engineer, no delegation or independent review claimed. Windows native testing is held and unverified. Existing-format project import is deferred. GPL-3.0-only has been present since first commit `87f11c1`. External research, reference names/source/artwork and fixtures stay outside product Git. Preserve user files, recovery owners and pending action-time permissions.

## Public product and release requirements — 19 October 2026

- Product history and release tooling were pushed to `git@github.com:samihsoylu/wireframe.git` on `main` through commit `473fcf9`, following explicit authorization. Public documentation leads with the user’s task, installation and limitations. Technical records remain separate.
- Start at version **0.1.0**. Every release must have bullet-point changes, with downloads and known limitations. See [changelog](CHANGELOG.md) and [release template](.github/RELEASE_TEMPLATE.md).
- macOS releases need a **DMG with a visible arrow from Wireframe to Applications**, installation instructions and matching source/licenses. Follow the established SideTray installer approach. No Apple Developer ID or notarization is planned; explain first-launch behavior honestly.
- Windows releases need a **portable EXE** plus source/license ZIP. Automatic Windows updates may come later. Native Windows testing remains held, not assumed passed.
- Linux releases need an architecture-labelled compiled archive and runtime requirements; initial format is TAR.GZ. Only advertise verified architectures/environments.
- **TODO: integrate Sparkle for macOS updates**, with a separate product signing key, HTTPS feed and a real installed-upgrade check. No updater currently exists. [Updater implementation and acceptance tasks](docs/UPDATES.md).
- Pushing source does not mean release acceptance. Do not publish a finished-release claim while material gates remain open.

## Current candidate and completed checks

Runtime commit **95e5365** consolidates editing, resource and presentation work. Local macOS ARM64 and Linux ARM64 review archives are built and verified, each with 1,759 corresponding-source files and 182 valid documentation links. Source/license and dependency checks pass; the GPL text is unchanged from the first commit. The package identities and logs are retained externally in `review-build-20261019`.

The consolidated package scoped checks total **737 distinct macOS** and **734 Linux** tests. All-target warnings-denied lint passes on both; macOS formatting passes. The macOS total includes completed unit suites from an interrupted broad run plus the later 83 affected integration checks. Do not call that interrupted all-target run successful. Unchanged detailed comparison evidence remains scoped to its original checks.

The extracted macOS ARM64 and Linux ARM64 review packages now pass the sampled four-page workflow: double-click full-text selection, edit cancellation, accepted editing, exact saved Undo/Redo and cold reopen with empty history. macOS includes Unicode entry; Linux uses direct key events and the viewer’s explicit Ctrl modifier after a browser shortcut did not reach the app. Both preserve saved bytes across page/Fit changes; Linux also retains its page and zoom through restart. This does not establish physical keyboard/IME, every control state or whole-platform acceptance. Exact snapshots and native reports are in external `review-build-20261019`. The disposable fixtures were restored after app exit. Current docs/installer changes postdate the frozen archives and do not change their runtime.

The latest runtime fix validates page/resource retention before committing pending edits, preserving drafts and Redo on rejected or empty actions. Four regression/history tests cover rejected transitions and successful separate history steps. Default rendering covers all 86 families in both styles: 172 SVGs and 344 PNGs, no export/layout failure or document mutation. Default rendering is not full control-variant or native input acceptance.

The subsequent toolbar correction aligns panel/presentation glyphs and adds filled Library/Comments active states. Nine affected checks, all-target lint, optimized macOS build and native panel/presentation routes pass. Saved four-page data and 231 prior working-copy records are unchanged. Details: [toolbar fidelity](docs/TOOLBAR-FIDELITY.md), external `toolbar-final-icons-current`. This runtime correction postdates the frozen archives and DMG; final packages must include it.

The updater packaging prerequisite now passes: staging preserves links, native macOS ZIPs preserve framework metadata, and DMG identity checks include entry types, targets and permissions. Eight packaging checks pass on Python 3.9/3.13; a real-framework fixture survives ZIP and DMG round trips with exact entries and valid ad-hoc signatures. The standalone DMG SDK mismatch is corrected. External `updater-packaging-current` records the failed first attempt, corrected result and cleanup. Sparkle is still not integrated; no signing key, update feed or public release was created.

The native macOS termination prerequisite now routes direct AppKit quit through the existing draft/save/export gate. Eleven close checks and all-target lint pass. An isolated native-menu trigger preserves Unicode drafts through file-chooser and quit cancellation, saves exactly, reopens with empty history and exits cleanly without changing saved bytes. All 232 prior working-copy records are unchanged; both recorded test processes exited. External `updater-lifecycle-current` records the trigger-only build difference and corrected harness mistakes. This is not actual Sparkle upgrade, OS shutdown or newly verified Windows/Linux behavior. Final packages must include this change.

## Next actions and unresolved requirements

1. Source publication is complete; no release tag or binary release was published. The macOS DMG wrapper is implemented and passes mounted app/source identity, signature, checksum and Finder visual inspection. Portable EXE emission passes a synthetic packaging check; this is not Windows execution. Keep Sparkle clearly planned until implemented and tested. A distinct product app icon remains a pre-release polish task; the current bundle shows the system placeholder.
2. Current package workflow is complete in the scope above. Move to the finite remaining comparison list; do not repeat the same workflow unless runtime changes or a new failure warrants it. Preserve other app/recovery owners.
3. Complete the finite comparison list: ordinary/selected/disabled/narrow/multiline control variants, precise text/contours and named export/input cases. Reuse accepted smooth library-drag and icon-placement observations unless a regression occurs.
4. Retain pending physical Option/Alt-drag, modified icon-label/source-root and offset-crop comparisons. Held cadence, edge scrolling, smart zoom and wider IME/accessibility are not proven by synthetic events.
5. Final native permanent image/library removal remains unverified; action-time permission is pending and earlier disposable page deletion permission does not cover it. Restore/history and cancellation already have scoped evidence.
6. Online image search and local text extraction first-release scope remain unresolved. Direct image URL import does not establish either capability or authorize deferral.
7. Finish named large-export/failure, component/selected-child and native permission/filesystem cases. No whole-platform acceptance from scoped checks; Windows is held, and Intel Mac, other Linux targets/Wayland/physical GPUs remain unverified.
8. Finish current-source package assembly and G17 reconciliation, with explicit proven/open/user-deferred status. No new broad research restart or repeated full suite without a new failure or changed input.

## Evidence navigation

Detailed candidate identities, native screenshots, saved-state comparisons and failure logs live outside product Git. Latest checkpoints: `updater-lifecycle-current`, `updater-packaging-current`, `toolbar-final-icons-current`, `review-build-20261019`, `catalog-panels-requirements-current`, `component-inspector-requirements-current`, `resource-requirements-current`, `export-destination-current`. Earlier checkpoints below retain their original scope. Historical process/window descriptions do not establish current liveness; recheck ownership before UI cleanup.

## Retained acceptance and evidence

The external release-closure reports below retain exact candidates, actions, assertions and limitations. Their scoped evidence remains valid for unchanged code; old executable identities and historical open-window descriptions do not describe the current runtime.

| Evidence checkpoint | Accepted scope or implemented contract |
|---|---|
| `version-route-current` | Unchanged Open Review 2: native page link to remembered alternate, exact-version Back/Forward and canvas Back links, forward-branch truncation, empty drawing history, byte-exact Save/cold reopen and remembered choices. Corrects stale version documentation; no runtime change or whole-gate closure. |
| `native-created-current`; `scenario-fidelity-current` | Native creation of four pages/40 controls, grouping/alignment, five routes, Unicode save/cold reopen, 2× PNG and linked PDF; paired actual-size line breaks/readability/no scenario clipping. Typeface/browser/device/sketch contours remain bounded cosmetic differences. |
| `native-scenarios-current`; `presentation-chrome` | Accessible page-switch draft-loss correction, independent presentation views, navigator hide/restore, keyboard/menu/history and editor return. |
| `native-comments-current` | Drafts/cancel, post/reply/edit, typed callout fields, reactions, recoverable deletion/restoration, drawing-history isolation and cold reopen. |
| `native-resources-current`; `cross-project` | Same-project Copy/Cut/Paste, shared-image import/reuse/Trash/Restore, cross-project discontiguous pages/conflicts, remapped links, exact history and cold reopen. Mixed/permanent cases retain their gaps. |
| `focus-boundaries`; `save-as-modes` | Save As draft acceptance before chooser, Cancel retaining one undoable edit, Unicode copy/cold reopen, automatic writes to both original/new copy and manual original preservation. Source failure/refused-destination protections pass scoped tests. |
| `text-alignment-current`; `structured-text-current`; `label-orientation-current` | Aligned auto-width origins, structured row metadata, keyboard focus after text dismissal/nudge/Undo/Delete and oriented text/render/export behavior. See [interaction acceptance](docs/INTERACTION-ACCEPTANCE.md) for exact native limits. |
| `editor-library-current`; earlier icon/text checkpoints | Fixed canvas origin across library categories, visible thumbnail scrollbar, corrected texture pixel origin and icon bounds. User confirmed smooth held library drag and accepted icon release placement; do not repeat those physical probes without a regression. |
| `rotated-links-current` | Rotated presentation/PDF highlight correction,70 affected tests, native routes and byte-identical PDF export; transient presentation capture and paired pointer overlap remain open. |
| `menu-open-current` | Asynchronous menu Open and source ownership handoff; cancellation/alias/failure guards, 425 application tests, native save/cancel/history/error/same-file/cold workflows. |
| `native-close-current` | Native manual canvas-close draft, Escape/error ownership, failed-save retry, automatic inspector close and exact saved/cold results; 414 application tests. |
| `orphan-import-current` | Native missing-control edit warnings, revised choices/cancel, identity-capture rejection/recovery, exact atomic history and incoming-result cold reopen. |
| `native-transfer-current` | Mixed complete-project decisions, source error/reload guards, comments/alternate routes, cold reopen, exact repeat no-op and native font comparison. Native page drag is covered by the later named checkpoint; held-preview/cadence limits remain. |
| `native-navigation-current` | Native Save/rename focus corrections, lifecycle/history transitions and per-project collapsed thumbnail restart; exact baseline restoration. The later page-drag checkpoint adds sampled ordering acceptance; deeper batch/depth variants remain open. |
| `page-order-current`; `page-clipboard-current`; `asset-trash-current`; subsystem docs | Implemented ordering/focus, clipboard/import and resource-retention contracts with scoped domain/application/failure tests. Native coverage is limited to the rows above and release checklist. |
