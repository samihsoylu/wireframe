# Wireframe — current state

## Goal and authority

Build a complete, usable offline Rust wireframing product for macOS, Windows and Linux, with close visual and behavioral fidelity to the established specification. The overall goal remains active. The initial investigation/readiness gate was met on 2026-10-13, and the user authorized automatic implementation progression. Continue targeted investigation when fidelity questions arise; do not treat a partial control inventory or passing foundation tests as product completion.

Work directly as Engineer. Preserve user files and explicit deferrals. Use synthetic fixtures for native experiments. Keep reference-specific names, extracted implementation/assets and investigation narratives outside this repository and its Git history. Original code is GPL-3.0-only from first commit 87f11c1; preserve dependency notices. Existing-format import remains deferred.

## Current checkpoint — 2026-10-15

Tables now resolve automatic text and link contrast per row, retain dark ink on light-blue selection, follow style rule tones and use transparent choice marks. Out-of-range selections do not color empty filler rows. Explicit text/persisted link colors, inline markup, source content, history and storage remain intact. Four new palette regressions cover scene colors and actual raster choice visibility, selection/header offsets, contrast boundaries, explicit overrides and round trips. See docs/RICH-TEXT.md.

All 577 tests pass in the final all-target run. Focused table tests, all-target Clippy, formatting, diff/exclusion, first-commit GPL and bundle-signature checks pass. The four integrated scenarios retain 48 controls and five links; both project files and all eight PNGs remain byte-identical to the preceding checkpoint. Two one-page table PDFs retain searchable text, 12 links each and notices; both rendered pages were inspected. Palette samples match PNG, but 72-dpi grid-line raster coverage differs and fails the stricter pixel-error diagnostic. Exact PDF/PNG raster parity is not accepted.

Native macOS inspection covers Clean and Sketch tables at 89%, style Undo and byte-identical resave. The signed debug bundle is running, UUID 883EE01E-67A6-388E-B429-D34C9D60E4F2, showing Clean tables with no selection. Exact native screenshot color matching remains unaccepted. Self-review only. Native Windows/Linux and full release acceptance remain open.

The embedded four-role drawing font, style-aware default labels/paragraphs and rounded notched-phone contour remain accepted. Existing missing-font documents retain Comic Neue. PDF export retains background immutable snapshots, atomic output, page/alternate selection, paper sizing, decoration, cancellation retention, Preview navigation and preserving an existing destination on failure. Remaining PDF typography, grid-line rasterization, native close-during-export and sustained responsiveness checks remain open. Table geometry, project selection-color customization, glyph shapes, device details and drawing style differences remain; the overall goal is active.

## Existing capabilities and limits

The 86-entry catalog, typed inspector, shared canvas/PNG/SVG rendering, recursive groups/components, rich text and project fonts, image transforms, page navigation, history and offline project storage are implemented. Catalog availability does not establish full visual or behavioral acceptance.

Save, Save As, automatic/manual named saving, New/Open/close and local recovery use the ordered background session. Retained working copies and destination archives are available through Recover work. Native checks cover automatic saving, manual Quit/restart recovery, Save As, missing-file close refusal, explicit recreation and immediate automatic-Quit flush. Shutdown cannot cancel an application exit after a save failure; recovery depends on durable local work. Filesystem stalls, final external-write races, path-alias limitations and archive cleanup UI remain unresolved. Forced-crash and power-loss acceptance are not established.

## Next work and unresolved acceptance

Keep the four integrated scenarios as repeatable acceptance fixtures. Add a table-based order-page variant when broadening integrated table acceptance; preserve the existing labels/fields version. Continue control-specific palette/contrast, typography and constrained-wrapping comparisons while preserving distinct regular, bold, italic and bold-italic roles. Refine device geometry and drawing style details against the established specification. Continue canvas/export and native interaction checks without treating catalog coverage as complete fidelity.

1. Continue targeted shared-interaction investigation: native held library-drag cadence, display-pixel alignment and library autoscroll. Six screen points is the current onset threshold, not an exact timing acceptance result. Do not repeat completed preview foundations or claim native held verification from post-release captures.
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
