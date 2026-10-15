# Wireframe — current state

## Goal and authority

Build a complete, usable offline Rust wireframing product for macOS, Windows and Linux, with close visual and behavioral fidelity to the established specification. The overall goal remains active. The initial investigation/readiness gate was met on 2026-10-13, and the user authorized automatic implementation progression. Continue targeted investigation when fidelity questions arise; do not treat a partial control inventory or passing foundation tests as product completion.

Work directly as Engineer. Preserve user files and explicit deferrals. Use synthetic fixtures for native experiments. Keep reference-specific names, extracted implementation/assets and investigation narratives outside this repository and its Git history. Original code is GPL-3.0-only from first commit 87f11c1; preserve dependency notices. Existing-format import remains deferred.

## Current checkpoint — 2026-10-15

File > Export PDF now exports all/current/chosen pages and existing alternates in navigator order. Content-sized and six standard paper sizes support independent names, numbers, notes and link hints. Large artwork shrinks without enlarging small artwork. A background worker captures an immutable snapshot and writes atomically; cancellation retains dialog options, errors preserve prior output, and closing waits for active export completion.

All 567 tests pass. Planner/worker coverage includes alternate destinations, excluded links, empty pages, overflowing notes, snapshot ownership and destination preservation. Shared whole-control links now respect specialized links and occlusion; paper hints appear once per original region. A PDF text-transform correction preserves rendered pixels while restoring header/note line separation in the tested reader. Clippy, formatting, diff, product exclusions and first-commit GPL checks pass. Self-review only.

Native macOS checks cover export, save-panel cancellation, current-page output and A4 output. The final native A4 PDF is byte-identical to the inspected planner output, reopens in Preview and follows an internal link to page two. A fresh signed debug bundle is running, UUID E9FD4E0F-8627-3F4E-B5AC-D2239613508A. A native oversized-notes failure preserves the prior destination byte for byte; application tests also verify failure cancels deferred quit. No build/test jobs remain. Native close-during-export checks, sustained editing/performance, header typography, mixed-script extraction, native Windows/Linux and full release acceptance remain open. See docs/PDF-EXPORT.md for evidence scope. The overall goal remains active.

## Existing capabilities and limits

The 86-entry catalog, typed inspector, shared canvas/PNG/SVG rendering, recursive groups/components, rich text and project fonts, image transforms, page navigation, history and offline project storage are implemented. Catalog availability does not establish full visual or behavioral acceptance.

Save, Save As, automatic/manual named saving, New/Open/close and local recovery use the ordered background session. Retained working copies and destination archives are available through Recover work. Native checks cover automatic saving, manual Quit/restart recovery, Save As, missing-file close refusal, explicit recreation and immediate automatic-Quit flush. Shutdown cannot cancel an application exit after a save failure; recovery depends on durable local work. Filesystem stalls, final external-write races, path-alias limitations and archive cleanup UI remain unresolved. Forced-crash and power-loss acceptance are not established.

## Next work and unresolved acceptance

Next integrated fixture set: a landing page with two buttons, a linked desktop contact page, an iPhone contact page and a tablet order page. Check complete layouts, spacing/alignment, text wrapping, control states, navigation and canvas/PDF consistency at fixed viewport sizes. Retain original product fixtures for repeatable acceptance.

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
