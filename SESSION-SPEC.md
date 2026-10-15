# Wireframe — current state

## Goal and authority

Build a complete, usable offline Rust wireframing product for macOS, Windows and Linux, with close visual and behavioral fidelity to the established specification. The overall goal remains active. The initial investigation/readiness gate was met on 2026-10-13, and the user authorized automatic implementation progression. Continue targeted investigation when fidelity questions arise; do not treat a partial control inventory or passing foundation tests as product completion.

Work directly as Engineer. Preserve user files and explicit deferrals. Use synthetic fixtures for native experiments. Keep reference-specific names, extracted implementation/assets and investigation narratives outside this repository and its Git history. Original code is GPL-3.0-only from first commit 87f11c1; preserve dependency notices. Existing-format import remains deferred.

## Current checkpoint — 2026-10-15

Nested group and component resizing now rounds leaf geometry after including parent displacement. Derived group crop origins and sizes round separately, including zero-size crops. Stored fractional geometry remains intact on load and movement. Fixed/computed axes, the shared 10-unit scalable minimum and 1-unit arrow exception remain in place. Definitions, sibling instances and exact transactional history are preserved.

Each absolute resize preview restarts from its transaction-start snapshot. This prevents descendant rounding from accumulating as the pointer moves. Direct, intermediate and detour paths now produce identical documents; returning to the starting point creates no history entry, and focus loss cancels exactly. Zero crop axes do not expose resize handles or enabled size fields; Remove crop retains the visible corner and restores the full content.

Five added regressions cover nested coordinate boundaries, half ties, persisted fractions, zero crop layout/UI and multi-frame resize behavior. All 554 tests pass in the final full run; all-target Clippy, formatting, diff and packaging checks pass. Native macOS verifies nested numeric geometry, rounded minimum crop bounds, zero crop removal and exact history on the interim build. The final build refresh verifies pointer width200→213, unchanged source/sibling, exact Undo/Redo and byte-identical save after process restart. Final bundle UUID E3D0023B-D1DD-3EF6-9315-3AC709828229; canonical executable identity and signature verify. Self-review only; no independent review.

The app is running a saved Geometry rounding fixture, Sketch 100%, Library hidden, first instance selected at(100,100,213,90). Numeric native checks before the final pointer fix retain their earlier build identity. No build or test jobs remain. Source/exclusion and first-commit license checks pass. Group crop reset/gestures, library insertion previews and save/decision gesture boundaries remain in place. Native held smoothness/onset/autoscroll and broader nested edge cases remain open. The PDF page-layout and annotation contract is now specified in docs/PDF-EXPORT.md. A primary-font backend probe preserves positioned text and real links; general fallback-font identity, scene conversion and integrated export UI remain outstanding. Next implement that export path with rendered-file acceptance; the overall goal remains active.

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
