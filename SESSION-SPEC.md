# Wireframe — current state

## Goal and authority

Build a complete, usable offline Rust wireframing product for macOS, Windows and Linux, with close visual and behavioral fidelity to the established specification. The overall goal remains active. The initial investigation/readiness gate was met on 2026-10-13, and the user authorized automatic implementation progression. Continue targeted investigation when fidelity questions arise; do not treat a partial control inventory or passing foundation tests as product completion.

Work directly as Engineer. Preserve user files and explicit deferrals. Use synthetic fixtures for native experiments. Keep reference-specific names, extracted implementation/assets and investigation narratives outside this repository and its Git history. Original code is GPL-3.0-only from first commit 87f11c1; preserve dependency notices. Existing-format import remains deferred.

## Current checkpoint — 2026-10-15

Canvas text now aligns its already-antialiased raster texture to physical pixels. Fractional texture origins previously caused an extra smoothing pass. The correction changes final display placement by at most half a physical pixel per axis; shaping, layout, caret/hit geometry, raster density, filtering, font bytes and exports remain unchanged. Panning reuses cached rasters. See docs/PROJECT-FONTS.md.

Two new regressions cover 30 simulated display-density/zoom combinations, one-to-one texel coverage, physical-grid alignment, bounded translation and cache reuse. Focused text/editing-layout tests, all-target Clippy, formatting, diff/exclusion, first-commit GPL and signed-bundle checks pass. All 581 tests pass in the completed all-target run.

A controlled native comparison draws the previous and corrected paint paths in one window at display density 2. Both use identical shaped glyphs, color and scale. Text at 75%, 100% and 120% visibly improves; increased edge contrast and less than 0.3% change in total ink corroborate reduced smoothing. This is a scoped native self-check. Separate product before/after windows differed in size and were excluded from quantitative comparison. Other font/capture differences and additional display hardware remain unverified.

All ten five-page scenario PNGs and four comparable SVGs are byte-identical. Both project documents and native resave are unchanged. Existing PDF tests remain in the suite; PDFs were not regenerated for this display-only change. The table geometry/palette, four-role drawing font and device corrections from prior checkpoints remain intact. Full cross-platform and complete-fidelity acceptance are still open.

The signed debug bundle is running, UUID A34D81A2-CE31-37A7-BD9F-098746B6FFE3, showing the saved Clean tablet order page fitted at 89%. Native 100% was also inspected; the temporary comparison window is closed. The reported extra canvas-text filtering has a demonstrated correction, without a claim that every possible source of softness is resolved.

Table scrollbar/border properties, selection-color customization, irregular outlines, glyph/device differences, held library-drag cadence, broader component combinations, PDF typography, native close-during-export, sustained responsiveness, failure/recovery and native Windows/Linux release acceptance remain open. The overall goal is active.

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
