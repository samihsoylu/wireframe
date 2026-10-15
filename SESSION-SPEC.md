# Wireframe — current state

## Goal and authority

Build a complete, usable offline Rust wireframing product for macOS, Windows and Linux, with close visual and behavioral fidelity to the established specification. The overall goal remains active. The initial investigation/readiness gate was met on 2026-10-13, and the user authorized automatic implementation progression. Continue targeted investigation when fidelity questions arise; do not treat a partial control inventory or passing foundation tests as product completion.

Work directly as Engineer. Preserve user files and explicit deferrals. Use synthetic fixtures for native experiments. Keep reference-specific names, extracted implementation/assets and investigation narratives outside this repository and its Git history. Original code is GPL-3.0-only from first commit 87f11c1; preserve dependency notices. Existing-format import remains deferred.

## Current checkpoint — 2026-10-15

Control resizing now uses a shared minimum of 10 units on scalable axes, with a 1-unit arrow exception. This applies to the catalog policy, inspector and generic drag handles. Locked descendants still transform with an unlocked parent. A cropped parent uses the actual child union after minimum clamping, so its final size can exceed the requested size and its visible corner can shift. Fixed/computed axes, normalized crops, component sources and sibling instances remain intact.

Three added regressions cover the minimum/axis policy, cropped group and component transformations, exact history and persistence, and move-only preservation of small stored geometry. Existing rendering tests still exercise one-pixel stored geometry through a dedicated fixture helper. Selecting an old small control does not clamp it; explicit resizing applies the current minimum. All 549 tests pass. All-target Clippy with warnings denied, formatting, diff checks and native macOS checks pass. No independent review is claimed.

Native macOS verifies requested crop width 10 resolves to 14.44 at x 207.22 with its trailing child kept 10 wide; exact Undo/Redo; unchanged definition/sibling; and byte-identical save after process restart. A separate native probe verifies selecting a saved 1×1 rectangle preserves it, explicit width 5 applies 10×10, Undo restores 1×1, and an arrow accepts 5×1. The final signed development bundle UUID is D5B70D54-6D32-3EAC-A6C7-156661B1BC8A. Canonical executable identity and signature verify. No build or test jobs remain. The app is running the saved Cropped resize fixture, Sketch 100%, Library hidden, first instance selected at (207.22,148,14.44,24).

Group crop reset anchoring, preview/gestures, library activation, full-size transient insertion previews, save/decision gesture boundaries and cropped Ungroup remain in place. Coordinate quantization during nested resizing remains open: the current model retains fractions. Native held smoothness, exact onset and library autoscroll still require stronger observation. Continue the remaining nested geometry cases and export acceptance; the overall goal remains active.

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
