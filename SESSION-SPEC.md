# Wireframe — current state

## Goal and authority

Build a complete, usable offline Rust wireframing product for macOS, Windows and Linux, with close visual and behavioral fidelity to the established specification. The overall goal remains active. The initial investigation/readiness gate was met on 2026-10-13, and the user authorized automatic implementation progression. Continue targeted investigation when fidelity questions arise; do not treat a partial control inventory or passing foundation tests as product completion.

Work directly as Engineer. Preserve user files and explicit deferrals. Use synthetic fixtures for native experiments. Keep reference-specific names, extracted implementation/assets and investigation narratives outside this repository and its Git history. Original code is GPL-3.0-only from first commit 87f11c1; preserve dependency notices. Existing-format import remains deferred.

## Current checkpoint — 2026-10-15

Table Border, Scrollbar and Scroll position now control the optional frame and track layout. Header rows keep the track below the first row; borderless tables remove top padding. Automatic width adds 16 units while fixed columns reserve 14. The last sorted header cell can extend above the track. Defaults remain framed without a scrollbar. See docs/RICH-TEXT.md.

All 584 tests pass. The final desktop field filter was corrected after native inspection exposed the hidden controls; all 80 desktop tests were rerun against that correction, including expanded inspector and fractional-edit coverage. Core/library/integration code remained unchanged during that rerun. Three new regressions cover both styles, frame/header combinations, thumb positions, PNG frame removal, autosizing, inspector metadata, history and round trips. All-target Clippy, formatting, product boundary, first-commit GPL and signed-bundle checks pass. Ten existing scenario PNGs and four project documents remain byte-identical.

Native macOS checks verify all three inspector options, scrollbar and border toggle/undo, position 0.75/one-step-undo to 0.5, Sketch style/undo and byte-identical final save. Signed debug bundle UUID EA4B554B-9666-36B1-8F5D-A290EC328B6D is running on the Clean six-table page at 93% Fit with no selection. These are scoped self-checks, not independent review or complete visual acceptance.

The shared scrollbar artwork remains approximate: Clean uses a square thumb and extra arrow separator lines. Resolve that styling through broader family acceptance. No new PDF or native Windows/Linux acceptance is claimed.

The prior canvas text correction remains intact: final raster placement aligns to physical pixels, preventing an extra filtering pass while preserving shaping, layout and exports. Controlled native evidence supports that scoped fix; other font/display differences remain open.

Remaining scope includes shared scrollbar artwork, selection-color customization, irregular outlines, glyph/device differences, held library-drag cadence, broader component combinations, PDF typography, native close-during-export, sustained responsiveness, failure/recovery and native Windows/Linux release acceptance. The overall goal remains active.

## Existing capabilities and limits

The 86-entry catalog, typed inspector, shared canvas/PNG/SVG rendering, recursive groups/components, rich text and project fonts, image transforms, page navigation, history and offline project storage are implemented. Catalog availability does not establish full visual or behavioral acceptance.

Save, Save As, automatic/manual named saving, New/Open/close and local recovery use the ordered background session. Retained working copies and destination archives are available through Recover work. Native checks cover automatic saving, manual Quit/restart recovery, Save As, missing-file close refusal, explicit recreation and immediate automatic-Quit flush. Shutdown cannot cancel an application exit after a save failure; recovery depends on durable local work. Filesystem stalls, final external-write races, path-alias limitations and archive cleanup UI remain unresolved. Forced-crash and power-loss acceptance are not established.

## Next work and unresolved acceptance

Keep the four integrated scenarios as repeatable acceptance fixtures. Keep the five-page table-based order variant alongside the original labels/fields version for integrated table acceptance. Continue control-specific palette/contrast, typography and constrained-wrapping comparisons while preserving distinct regular, bold, italic and bold-italic roles. Refine device geometry and drawing style details against the established specification. Continue canvas/export and native interaction checks without treating catalog coverage as complete fidelity.

1. Broaden display-hardware acceptance for canvas pixel alignment; retain the demonstrated extra-filtering correction and distinguish remaining font/capture differences. Continue targeted shared-interaction investigation: native held library-drag cadence, display-pixel alignment and library autoscroll. Six screen points is the current onset threshold, not an exact timing acceptance result. Do not repeat completed preview foundations or claim native held verification from post-release captures.
2. Complete remaining group/component crop, lock and resize combinations, clipping, snapping and alignment behavior. Cropped-group ungrouping and group crop creation/adjustment/removal UI have scoped native acceptance; broader nested component combinations remain. Alternate versions, page hierarchy and presentation target/history behavior need broader acceptance.
3. Refine typography, state styling and geometric/visual edge cases against the established specification. Keep original artwork and fonts properly licensed. Legacy shape/rule coverage, image metadata and placeholder/filter behavior remain open where identified in family documents.
4. Complete PDF typography and remaining native failure/performance acceptance, image scale/options and broader font handling. Preserve shared canvas/export geometry.
5. Package and run native macOS, Windows and Linux acceptance and release checks. Large-document profiling, revision-based cache invalidation, forced failures and recovery remain outstanding. No full cross-platform or complete-fidelity claim.

## Durable specifications

- docs/IMPLEMENTATION-PLAN.md: full scope and acceptance discipline.
- docs/LIBRARY-INSERTION.md and docs/CANVAS-SCROLLING.md: pointer behavior and current verification limits.
- docs/SAVING.md, docs/PROJECT-SESSION.md, docs/WORKING-COPIES.md and docs/RECOVERY.md: desktop persistence and failure boundaries.
- docs/DOCUMENT-FORMAT.md and docs/RECURSIVE-EDITOR.md: recursive data and live editing/storage boundaries.
- docs/CATALOG-AND-LAYOUT.md and control-family documents: rendering and property requirements.
- docs/RICH-TEXT.md, docs/PROJECT-FONTS.md and docs/TEXT-ENTRY.md: text and fonts.
- docs/PDF-EXPORT.md: page selection, sizing, notes, text/link fidelity, architecture boundaries and scoped integrated acceptance.
- docs/IMAGES.md and docs/COMPONENTS.md: image and component behavior and remaining gaps.

Keep product-only fixtures/evidence local unless selected for distribution. Update this checkpoint and commit completed product work. No unattended supervision beyond active goal execution is implied. The overall goal remains active.
