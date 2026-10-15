# Wireframe — current state

## Goal and authority

Build a complete, usable offline Rust wireframing product for macOS, Windows and Linux, with close visual and behavioral fidelity to the established specification. The overall goal remains active. The initial investigation/readiness gate was met on 2026-10-13, and the user authorized automatic implementation progression. Continue targeted investigation when fidelity questions arise; do not treat a partial control inventory or passing foundation tests as product completion.

Work directly as Engineer. Preserve user files and explicit deferrals. Use synthetic fixtures for native experiments. Keep reference-specific names, extracted implementation/assets and investigation narratives outside this repository and its Git history. Original code is GPL-3.0-only from first commit 87f11c1; preserve dependency notices. Existing-format import remains deferred.

## Current checkpoint — 2026-10-15

Positioned text now retains its actual primary or native fallback face, collection index and non-default variation coordinates. Detached collection members use index zero. Bitmap glyphs and unsupported/unavailable faces preserve the exact existing ink with an explicit reason. Font serialization is deferred until export requests bytes; cache accounting reserves the eventual serialized copy. Live spans share large fallback faces through bounded weak references.

Three added regressions cover source identity and live sharing, native variations/lazy storage, and detached font checksums/alignment/bounds. All 557 tests pass in the final full run; all-target Clippy, formatting and diff checks pass. Six rendered backend probes preserve the previous canvas ink, glyph arrays and layout exactly before PDF encoding. Their PDF vector fonts are embedded; rendered bounds differ by at most one raster pixel. Latin extraction is canonically equivalent and Chinese exact. Mixed-direction extraction order differs from source, and bitmap emoji is visible but not searchable. Variable coordinates pass native tests but have not yet been rendered through the PDF backend. Self-review only.

This is an export foundation change. PDF export remains unavailable in the app; no new dependency, package or native app-flow acceptance is claimed. The running signed bundle remains the earlier coordinate-rounding build, UUID E3D0023B-D1DD-3EF6-9315-3AC709828229; its open scratch document has additional trial controls. Its 554-test and native pointer/history/restart acceptance remain historical evidence. Nested resize rounding, transaction-start previews, zero crop handling and exact Undo/Redo remain in place. Product license/exclusion checks pass. No build or test jobs remain at this checkpoint.

Next implement Scene conversion, page selection/layout, annotations and background atomic PDF output under docs/PDF-EXPORT.md, resolving extraction gaps with real rendered files and readers. General font/synthetic-style fidelity, large-document profiling and native Windows/Linux acceptance remain open. The overall goal remains active.

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
- docs/PDF-EXPORT.md: page selection, sizing, notes, text/link fidelity, architecture boundaries and pending integrated acceptance.
- docs/IMAGES.md and docs/COMPONENTS.md: image and component behavior and remaining gaps.

Keep product-only fixtures/evidence local unless selected for distribution. Update this checkpoint and commit completed product work. No unattended supervision beyond active goal execution is implied. The overall goal remains active.
