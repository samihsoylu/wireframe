# Wireframe — current state

## Goal and authority

Build a complete, usable offline Rust wireframing product for macOS, Windows and Linux, with close visual and behavioral fidelity to the established specification. The overall goal remains active. The initial investigation/readiness gate was met on 2026-10-13, and the user authorized automatic implementation progression. Continue targeted investigation when fidelity questions arise; do not treat a partial control inventory or passing foundation tests as product completion.

Work directly as Engineer. Preserve user files and explicit deferrals. Use synthetic fixtures for native experiments. Keep reference-specific names, extracted implementation/assets and investigation narratives outside this repository and its Git history. Original code is GPL-3.0-only from first commit 87f11c1; preserve dependency notices. Existing-format import remains deferred.

## Current checkpoint — 2026-10-15

Table geometry now uses filled sort arrows, one-unit Clean and two-unit Sketch separators, inside frames, and correctly sized/aligned checkbox and radio symbols. The prior row-contrast, selection and explicit-color behavior remains intact. Two new regressions inspect actual PNG grid coverage, frame bounds, filled arrow interiors and symbol geometry; existing rendering/palette regressions cover history, clipping, links and row colors. See docs/RICH-TEXT.md.

All 579 tests pass in the final all-target run. Focused tests, all-target Clippy, formatting, diff/exclusion, first-commit GPL and bundle-signature checks pass. The original four integrated scenarios retain 48 controls and five links with all eight PNGs unchanged. A separate five-page variant has 44 controls and five links, including a table-based tablet order page and a table-detail page. Both variant project files are unchanged by the renderer correction.

Both five-page PDFs retain searchable text, five internal links each and notices; all ten rendered pages were inspected. Table-detail PDF/PNG grid samples match at 72 dpi, with mean pixel errors 0.767 Clean and 0.974 Sketch. Exact raster parity is not accepted, and this result does not resolve every previous PDF fixture. All four local native/export comparisons improve; regular Sketch contours, font and device differences remain.

Native macOS inspection covers both table styles at 80%, Undo back to Clean and byte-identical resave, plus the tablet order variant at 75%. The signed debug bundle is running, UUID 86064058-44B3-376F-914F-6C3210FA937F, showing the Clean order page with no selection. Self-review only. Native Windows/Linux and full release acceptance remain open. The reported canvas softness at 100% is retained for the next investigation; no cause or fix is claimed.

The embedded four-role drawing font, style-aware default labels/paragraphs and rounded notched-phone contour remain accepted. Existing missing-font documents retain Comic Neue. PDF export retains background immutable snapshots, atomic output, page/alternate selection, paper sizing, decoration, cancellation retention, Preview navigation and preserving an existing destination on failure. Remaining PDF typography, native close-during-export and sustained responsiveness checks remain open. Table scrollbar/border properties, project selection-color customization, irregular outlines, glyph shapes and device details remain; the overall goal is active.

## Existing capabilities and limits

The 86-entry catalog, typed inspector, shared canvas/PNG/SVG rendering, recursive groups/components, rich text and project fonts, image transforms, page navigation, history and offline project storage are implemented. Catalog availability does not establish full visual or behavioral acceptance.

Save, Save As, automatic/manual named saving, New/Open/close and local recovery use the ordered background session. Retained working copies and destination archives are available through Recover work. Native checks cover automatic saving, manual Quit/restart recovery, Save As, missing-file close refusal, explicit recreation and immediate automatic-Quit flush. Shutdown cannot cancel an application exit after a save failure; recovery depends on durable local work. Filesystem stalls, final external-write races, path-alias limitations and archive cleanup UI remain unresolved. Forced-crash and power-loss acceptance are not established.

## Next work and unresolved acceptance

Keep the four integrated scenarios as repeatable acceptance fixtures. Keep the five-page table-based order variant alongside the original labels/fields version for integrated table acceptance. Continue control-specific palette/contrast, typography and constrained-wrapping comparisons while preserving distinct regular, bold, italic and bold-italic roles. Refine device geometry and drawing style details against the established specification. Continue canvas/export and native interaction checks without treating catalog coverage as complete fidelity.

1. Investigate reported canvas softness at 100% zoom after current table work. Check physical-pixel placement and texture filtering while preserving text shaping; Retina scaling is already present and the cause is unproven. Continue targeted shared-interaction investigation: native held library-drag cadence, display-pixel alignment and library autoscroll. Six screen points is the current onset threshold, not an exact timing acceptance result. Do not repeat completed preview foundations or claim native held verification from post-release captures.
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
