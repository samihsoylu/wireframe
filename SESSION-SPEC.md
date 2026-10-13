# Wireframe — current state

## Goal and authority

Build a complete, usable offline Rust wireframing product for macOS, Windows and Linux, with close visual and behavioral fidelity to the established specification. The overall goal remains active. The initial investigation/readiness gate was met on 2026-10-13, and the user authorized automatic implementation progression. Continue targeted investigation when fidelity questions arise; do not treat a partial control inventory or passing foundation tests as product completion.

Work directly as Engineer. Preserve user files and explicit deferrals. Use synthetic fixtures for native experiments. Keep reference-specific names, extracted implementation/assets and investigation narratives outside this repository and its Git history. Original code is GPL-3.0-only from first commit 87f11c1; preserve dependency notices. Existing-format import remains deferred.

## Current checkpoint — 2026-10-14

The live desktop app now owns a version-2 recursive Project through the workspace editor. It opens version-1 files by read-only migration, saves version 2, and uses the same project storage for recovery. Nested groups have real parent-local children, editable contexts, movement, resizing, grouping/ungrouping, duplication and undo/redo. Enter/double-click/Edit group contents enters a group; Escape/Exit group returns to its parent. Full-enclosure marquee and visible leaf ordering drive selection.

Canvas, thumbnails and SVG/PNG exports now consume a shared positioned scene for basic rich-text controls and tables. It retains glyph positions, real bold/italic faces, inline colors/sizes, underlines/strikes, choices, sort indicators, cell alignment, clipping and visible link regions. Presentation follows inline page links. Fonts are independently distributed Comic Neue files with OFL notices and pinned provenance. Other controls still use the initial primitive renderer; this is not complete visual fidelity.

Native macOS evidence: migrated an isolated two-control fixture; grouped, duplicated and nested it; entered two levels; moved one child X=328→338; undid to 328; redid; saved; closed and reopened; re-entered both levels and confirmed X=338. The saved file contains an outer group, two child groups and four controls. Evidence is local under evidence/recursive-native-*. Source fixtures were preserved.

Additional native evidence: opened the styled-text/table specimen, inspected canvas and thumbnails, followed Continue to its linked page in presentation, and exported PNG through the native Save dialog. The native PNG and example-generated PNG have the same SHA-256. Evidence is local under evidence/text-scene/. The table now measures 171×81; typography, clean-style font selection and exact sizing remain acceptance work.

The text/table inspector now reads effective typed properties from the recursive node and writes sparse values through the catalog. It exposes implemented text styles/colors/alignment, six choice states, and table row/header/selection/line options, with per-property Reset and Auto-size. Locked controls reject edits. Project font defaults are shared with layout; unknown properties survive. New checkbox/radio insertion uses automatic catalog defaults. Choice marks, disabled outlines/labels/links, 20-unit label padding and state-preserving projection are implemented. Other families still need full typed/rendered state coverage.

Native property evidence: inserted a checkbox; selected Indeterminate and disabled; undid/redid; reset and undid reset; saved, closed and reopened. Confirmed the same state (historical measurement before the line-metric correction: 73.69×25). Selected and disabled also worked, and its menu closed after selection. Restored the saved state with Undo. Local evidence: evidence/property-inspector/; saved input: evidence/text-scene/Native-choice-properties.wireframe. The default checkbox's typography/measurement remains an open fidelity item.

Verification: 70 tests pass (five app, eleven legacy workflows, ten document, eleven catalog/layout, nine text, eleven recursive workspace, eight scene, five property/state). Clippy passes with warnings denied. Prior five-fixture read-only migration checks retain source bytes. The macOS development bundle is refreshed. Windows/Linux runtime checks remain unperformed.

Text metrics now separate font shaping from canvas line boxes: maximum font size per row, 20% interline spacing, common baseline, integer measured width and rounded total height. Measurement, shared drawing and link regions consume the same metrics. Native inspector verified 74×23 at 13 points and 120×34 at 24 points. Multiline canvas/thumbnail inspection and presentation inline navigation passed. Evidence: evidence/text-metrics/. Exact glyph widths, fallback and clean-style fonts remain open.

Recovery follow-up: restarting after the previous synthetic state/undo checks offered a stale recovery copy with Selected-and-disabled while the saved fixture retained Indeterminate-and-disabled. Restored it separately and saved Native-font24.wireframe without changing original evidence. Determine whether clean undo/exit leaves an older recovery snapshot; do not claim recovery correctness from the current checks.

## Next work and unresolved acceptance

1. Expand representative text/table visual comparisons and refine typography, states, padding, clean/sketch styles and source-specific formatting. Current shared rendering covers eleven basic text controls plus tables; other control families still need it. Maintain source text and shared canvas/export geometry.
2. Extend typed inspector and rendering beyond the initial text/table subset, including button/input states, automatic size/anchor rules, text/macros/icons and licensed fonts. The catalog inventories 86 families; this is not 86 completed renderers.
3. Complete component creation/source editing/overrides/reset/detach UI and component resizing. Existing instances can be rendered, moved and copied with dependencies; full component editing is not yet exposed. Group resizing containing instances remains incomplete. Cropped-group ungrouping requires layout-aware handling; it currently reports the limitation.
4. Add asset import/crop/rotation UI, alternate versions, page hierarchy, richer snapping/alignment/distribution and presentation history/target behavior. Verify shutdown/recovery and debounced saves through real failures.
5. Finish exports including PDF/links/fonts and 1x/2x behavior, then package and run native macOS/Windows/Linux acceptance. No cross-platform runtime or full visual fidelity claim is established.

Large-document profiling and revision-based cache invalidation remain open. Current history is bounded by snapshot count/estimated serialized bytes; current layout caching compares project snapshots. Native checks use synthetic documents and are scoped evidence, not independent review.

## Durable specifications

- docs/IMPLEMENTATION-PLAN.md: complete five-milestone scope and acceptance discipline.
- docs/DOCUMENT-FORMAT.md: recursive data, scoped identity, expansion, migration and validation.
- docs/CATALOG-AND-LAYOUT.md: property defaults, schemas, search, sizing and remaining gaps.
- docs/RICH-TEXT.md: source-preserving text/structured parsing and font geometry.
- docs/INSPECTOR-AND-STATES.md: typed property edits, choice rendering and native checks.
- docs/RECURSIVE-EDITOR.md: current live editing/storage boundary and verification.

Keep product-only fixtures/evidence local unless explicitly selected for distribution. Update this checkpoint and commit completed product work. No unattended supervision beyond active goal execution is implied.
