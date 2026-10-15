# Wireframe — current state

## Goal and authority

Build a complete, usable offline Rust wireframing product for macOS, Windows and Linux, with close visual and behavioral fidelity to the established specification. The overall goal remains active. The initial investigation/readiness gate was met on 2026-10-13, and the user authorized automatic implementation progression. Continue targeted investigation when fidelity questions arise; do not treat a partial control inventory or passing foundation tests as product completion.

Work directly as Engineer. Preserve user files and explicit deferrals. Use synthetic fixtures for native experiments. Keep reference-specific names, extracted implementation/assets and investigation narratives outside this repository and its Git history. Original code is GPL-3.0-only from first commit 87f11c1; preserve dependency notices. Existing-format import remains deferred.

## Current checkpoint — 2026-10-15

Group and component crop editing is available through Crop selection. The full-content preview preserves descendant masks and paint order. Drag inside to move the frame, outside to move content behind a fixed frame, and use handles to resize. Apply records one Undo step; Cancel discards the preview. Remove crop expands from the visible top-left. Component-local removal and crop-only reset preserve source data and unrelated overrides. See docs/RECURSIVE-EDITOR.md.

Native testing exposed a missed drag when press/move and move/release arrived in two frames. A regression reproduced the failure; the editor now retains the initial press immediately. The same gesture handling serves image cropping. Background rendering omits the target during preview, preventing a stale image at the original position.

Six group tests, five crop UI tests and all 76 app tests pass; affected image/component/ungroup checks also pass. The current inventory is 544 tests across runs, not a new full 544-test run. All-target Clippy with warnings denied, formatting, diff checks and self-review pass. Native macOS verifies frame movement, content movement, resize preview, cancellation, visible-position-preserving removal, Undo/Redo and byte-identical save after process restart. Component crop overrides are covered by tests; broader native combinations remain open. No independent review is claimed.

Final signed development bundle UUID B198B288-B58E-3ECE-8362-8BFA0E073C9E. Canonical executable identity and signature verify. The app runs the saved Crop editing fixture, Sketch at 100%, Library hidden, one group selected with visible rectangle (195,155,100,100). No background builds or tests remain. Completed history remains in Git and the specifications. The overall goal remains active.

Library activation, full-size transient previews, save/decision gesture boundaries and cropped Ungroup remain in place. Native held smoothness, exact onset and library autoscroll still require stronger observation. Broader group/component crop, resize and lock combinations and export completion are next.

## Existing capabilities and limits

The 86-entry catalog, typed inspector, shared canvas/PNG/SVG rendering, recursive groups/components, rich text and project fonts, image transforms, page navigation, history and offline project storage are implemented. Catalog availability does not establish full visual or behavioral acceptance.

Save, Save As, automatic/manual named saving, New/Open/close and local recovery use the ordered background session. Retained working copies and destination archives are available through Recover work. Native checks cover automatic saving, manual Quit/restart recovery, Save As, missing-file close refusal, explicit recreation and immediate automatic-Quit flush. Shutdown cannot cancel an application exit after a save failure; recovery depends on durable local work. Filesystem stalls, final external-write races, path-alias limitations and archive cleanup UI remain unresolved. Forced-crash and power-loss acceptance are not established.

## Next work and unresolved acceptance

1. Continue targeted shared-interaction investigation: native held library-drag cadence, display-pixel alignment and library autoscroll. Six screen points is the current onset threshold, not an exact timing acceptance result. Do not repeat completed preview foundations or claim native held verification from post-release captures.
2. Complete remaining group/component crop, lock and resize combinations, clipping, snapping and alignment behavior. Cropped-group ungrouping and group crop creation/adjustment/removal UI have scoped native acceptance; broader nested component combinations remain. Alternate versions, page hierarchy and presentation target/history behavior need broader acceptance.
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
