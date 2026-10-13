# Wireframe — current state

## Goal and authority

Build a complete, usable offline Rust wireframing product for macOS, Windows and Linux, with close visual and behavioral fidelity to the established specification. The overall goal remains active. The initial investigation/readiness gate was met on 2026-10-13, and the user authorized automatic implementation progression. Continue targeted investigation when fidelity questions arise; do not treat a partial control inventory or passing foundation tests as product completion.

Work directly as Engineer. Preserve user files and explicit deferrals. Use synthetic fixtures for native experiments. Keep reference-specific names, extracted implementation/assets and investigation narratives outside this repository and its Git history. Original code is GPL-3.0-only from first commit 87f11c1; preserve dependency notices. Existing-format import remains deferred.

## Current checkpoint — 2026-10-14

The live desktop app now owns a version-2 recursive Project through the workspace editor. It opens version-1 files by read-only migration, saves version 2, and uses the same project storage for recovery. Nested groups have real parent-local children, editable contexts, movement, resizing, grouping/ungrouping, duplication and undo/redo. Enter/double-click/Edit group contents enters a group; Escape/Exit group returns to its parent. Full-enclosure marquee and visible leaf ordering drive selection.

Canvas, thumbnails and SVG/PNG exports now consume a shared positioned scene for basic rich-text controls and tables. It retains glyph positions, real bold/italic faces, inline colors/sizes, underlines/strikes, choices, sort indicators, cell alignment, clipping and visible link regions. Presentation follows inline page links. Fonts are independently distributed Comic Neue files with OFL notices and pinned provenance. Other controls still use the initial primitive renderer; this is not complete visual fidelity.

Native macOS evidence: migrated an isolated two-control fixture; grouped, duplicated and nested it; entered two levels; moved one child X=328→338; undid to 328; redid; saved; closed and reopened; re-entered both levels and confirmed X=338. The saved file contains an outer group, two child groups and four controls. Evidence is local under evidence/recursive-native-*. Source fixtures were preserved.

Additional native evidence: opened the styled-text/table specimen, inspected canvas and thumbnails, followed Continue to its linked page in presentation, and exported PNG through the native Save dialog. The native PNG and example-generated PNG have the same SHA-256. Evidence is local under evidence/text-scene/. The table measures 169.78125×83; typography, clean-style font selection and exact sizing remain acceptance work.

Verification: 63 tests pass (five app, eleven legacy workflows, ten document, eleven catalog/layout, eight text, eleven recursive workspace, seven scene). Clippy passes with warnings denied. Prior five-fixture read-only migration checks retain source bytes. The macOS development bundle is refreshed. Windows/Linux runtime checks remain unperformed.

## Next work and unresolved acceptance

1. Expand representative text/table visual comparisons and refine typography, states, padding, clean/sketch styles and source-specific formatting. Current shared rendering covers eleven basic text controls plus tables; other control families still need it. Maintain source text and shared canvas/export geometry.
2. Complete typed inspector and all control-family rendering, automatic size/anchor rules, states, text/macros/icons and licensed fonts. The catalog inventories 86 families; this is not 86 completed renderers.
3. Complete component creation/source editing/overrides/reset/detach UI and component resizing. Existing instances can be rendered, moved and copied with dependencies; full component editing is not yet exposed. Group resizing containing instances remains incomplete. Cropped-group ungrouping requires layout-aware handling; it currently reports the limitation.
4. Add asset import/crop/rotation UI, alternate versions, page hierarchy, richer snapping/alignment/distribution and presentation history/target behavior. Verify shutdown/recovery and debounced saves through real failures.
5. Finish exports including PDF/links/fonts and 1x/2x behavior, then package and run native macOS/Windows/Linux acceptance. No cross-platform runtime or full visual fidelity claim is established.

Large-document profiling and revision-based cache invalidation remain open. Current history is bounded by snapshot count/estimated serialized bytes; current layout caching compares project snapshots. Native checks use synthetic documents and are scoped evidence, not independent review.

## Durable specifications

- docs/IMPLEMENTATION-PLAN.md: complete five-milestone scope and acceptance discipline.
- docs/DOCUMENT-FORMAT.md: recursive data, scoped identity, expansion, migration and validation.
- docs/CATALOG-AND-LAYOUT.md: property defaults, schemas, search, sizing and remaining gaps.
- docs/RICH-TEXT.md: source-preserving text/structured parsing and font geometry.
- docs/RECURSIVE-EDITOR.md: current live editing/storage boundary and verification.

Keep product-only fixtures/evidence local unless explicitly selected for distribution. Update this checkpoint and commit completed product work. No unattended supervision beyond active goal execution is implied.
