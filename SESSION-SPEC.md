# Wireframe — current state

## Goal and authority

Build a complete, usable offline Rust wireframing product for macOS, Windows and Linux, with close visual and behavioral fidelity to the established specification. The overall goal remains active. The initial investigation/readiness gate was met on 2026-10-13, and the user authorized automatic implementation progression. Continue targeted investigation when fidelity questions arise; do not treat a partial control inventory or passing foundation tests as product completion.

Work directly as Engineer. Preserve user files and explicit deferrals. Use synthetic fixtures for native experiments. Keep reference-specific names, extracted implementation/assets and investigation narratives outside this repository and its Git history. Original code is GPL-3.0-only from first commit 87f11c1; preserve dependency notices. Existing-format import remains deferred.

## Current checkpoint — 2026-10-15

Library drag activation now samples the grab point when movement enters the image area. Label-only drags do not insert; label-to-image movement can start a drag. Fast input that skips intermediate source samples retains a fallback to the original image-area press. The chosen anchor remains fixed while held and through release. Full-size transient preview, edge-clamped placement and one-step history remain in place. See docs/LIBRARY-INSERTION.md.

Save and decision/recovery boundaries end active pointer gestures at the last visible canvas edit. A held library preview cannot insert after Save captures its snapshot, and move/resize edge scrolling cannot alter that snapshot afterward. Canvas input is disabled during those boundaries. See docs/CANVAS-SCROLLING.md and docs/SAVING.md.

Three new regressions failed before correction and pass afterward. All 12 library-drag tests and all 70 app tests pass, including a fourth new canvas-move/save-boundary test. This covers 528 current tests across runs; the most recent full all-target run was 523, followed by focused additions. Final all-target Clippy with warnings denied, formatting, diff checks and self-review pass. Native macOS verifies label rejection, image-drop position, one-step Undo/Redo, outside-canvas cancellation and saving the one-control fixture. Held native smoothness and exact onset timing remain unverified; frame-level tests cover held preview behavior at 50/100/200% zoom. No independent review is claimed.

The signed development bundle has UUID 867E3147-311A-375A-8AEB-9179740E6168. Canonical executable identity and ad-hoc signature verify. The new process runs the saved one-control drag fixture, Sketch style at 100%, Buttons library visible and Circle button selected. No background builds or test jobs remain after this checkpoint. Prior completed work is in Git history and the relevant specifications below.

## Existing capabilities and limits

The 86-entry catalog, typed inspector, shared canvas/PNG/SVG rendering, recursive groups/components, rich text and project fonts, image transforms, page navigation, history and offline project storage are implemented. Catalog availability does not establish full visual or behavioral acceptance.

Save, Save As, automatic/manual named saving, New/Open/close and local recovery use the ordered background session. Retained working copies and destination archives are available through Recover work. Native checks cover automatic saving, manual Quit/restart recovery, Save As, missing-file close refusal, explicit recreation and immediate automatic-Quit flush. Shutdown cannot cancel an application exit after a save failure; recovery depends on durable local work. Filesystem stalls, final external-write races, path-alias limitations and archive cleanup UI remain unresolved. Forced-crash and power-loss acceptance are not established.

## Next work and unresolved acceptance

1. Continue targeted shared-interaction investigation: native held library-drag cadence, display-pixel alignment and library autoscroll. Six screen points is the current onset threshold, not an exact timing acceptance result. Do not repeat completed preview foundations or claim native held verification from post-release captures.
2. Complete remaining group/component crop, lock and resize combinations, clipping, snapping and alignment behavior. Cropped-group ungrouping still has a documented limitation. Alternate versions, page hierarchy and presentation target/history behavior need broader acceptance.
3. Refine typography, state styling and geometric/visual edge cases against the established specification. Keep original artwork and fonts properly licensed. Legacy shape/rule coverage, image metadata and placeholder/filter behavior remain open where identified in family documents.
4. Complete PDF export, image scale/options, links, notes and font handling. Preserve shared canvas/export geometry.
5. Package and run native macOS, Windows and Linux acceptance and release checks. Large-document profiling, revision-based cache invalidation, forced failures and recovery remain outstanding. No full cross-platform or complete-fidelity claim.

## Durable specifications

- docs/IMPLEMENTATION-PLAN.md: full scope and acceptance discipline.
- docs/LIBRARY-INSERTION.md and docs/CANVAS-SCROLLING.md: pointer behavior and current verification limits.
- docs/SAVING.md, docs/PROJECT-SESSION.md, docs/WORKING-COPIES.md and docs/RECOVERY.md: desktop persistence and failure boundaries.
- docs/DOCUMENT-FORMAT.md and docs/RECURSIVE-EDITOR.md: recursive data and live editing/storage boundaries.
- docs/CATALOG-AND-LAYOUT.md and control-family documents: rendering and property requirements.
- docs/RICH-TEXT.md, docs/PROJECT-FONTS.md and docs/TEXT-ENTRY.md: text and fonts.
- docs/IMAGES.md and docs/COMPONENTS.md: image and component behavior and remaining gaps.

Keep product-only fixtures/evidence local unless selected for distribution. Update this checkpoint and commit completed product work. No unattended supervision beyond active goal execution is implied. The overall goal remains active.
