# Wireframe — current state

## Outcome

Deliver a usable local Rust wireframing product for macOS, Windows and Linux. Close visual and behavioral fidelity is a required acceptance target; a generic editor foundation does not close the goal. Follow SOLID principles where they improve the design. The user has authorized continuous goal work and delegates technical approach decisions.

## Repository boundary

Keep this repository strictly focused on the product: implementation, tests, product specifications, user documentation, packaging and required license notices. License original project code under GNU GPL v3.0 only (`GPL-3.0-only`), present from the first commit. Do not add external application research, extracted code, analysis records or origin narratives to this repository or its history.

## Architecture

Rust 2024 with egui/eframe 0.36.2, custom canvas, shared scene/export primitives, versioned JSON documents, atomic storage and bounded undo history. Modules separate model, editor, scene, storage and UI responsibilities.

## Current checkpoint — 2026-10-13

The initial Rust foundation has 24 controls, local projects, pages, notes, properties, selection, movement, eight-handle resizing, grouping, alignment, duplication, locking, undo/redo, recovery and PNG/SVG exports. A dark toolbar, Quick Add, horizontal previews and page thumbnails are implemented. Native macOS checks verified text/save/new/reopen, recovery, PNG export, drag, resize, library insertion, and the unsaved-close dialog. Fourteen automated workflow/input tests passed at the latest canvas checkpoint; rerun after subsequent edits. The GPL license and ignore rules were committed first (87f11c1); the desktop foundation and recursive project model are committed in 0fcfbd5.

The initial behavior-specification gate was met on 2026-10-13. Feature development may resume. The user explicitly authorizes an automatic transition to implementation once the readiness gate below is met; no further approval is needed for that transition. The current implementation is provisional and has not met fidelity acceptance. Preserve it and all user/test projects. The app currently has a saved synthetic gesture document open.

## Next gate

Complete the behavior specification, coverage matrix and representative fixtures before further feature implementation. Establish control/property coverage, editing rules, text syntax, grouping, reusable components, assets, persistence and exports. Each requirement needs an explicit evidence status and an acceptance case. Keep source material and investigation artifacts outside this product repository. Translate established findings into neutral product requirements only.

### Readiness decision

Resume implementation automatically when the major workflows and data relationships are mapped, representative normal and edge cases have reproducible acceptance criteria, and no unresolved question is likely to force a redesign of the document model, rendering, editing or storage. Record the decision and remaining bounded unknowns before resuming. A catalog of feature names, screenshots alone, or passing foundation tests does not satisfy this gate. Further targeted investigation continues alongside implementation whenever a fidelity question emerges. Existing-format import remains deferred; no other major fidelity scope is silently deferred.

## Model requirements established at the research checkpoint

The next design must support recursive ordered children with parent-local coordinates, shared component definitions with per-instance descendant overrides, and per-control typed state and sizing rules. Moving and ungrouping must preserve canvas positions and descendant structure; detaching an instance must retain its effective local values. These requirements are established for representative cases; dependency invalidation and persistence failure cases remain architectural questions. The initial implementation readiness gate is now met; these are requirements for the foundation redesign, not verified product capabilities.

Text/layout requirements now include a rich-text intermediate representation, per-line link geometry, automatic versus fixed dimensions, and alignment-preserving origin changes. Image requirements include project-owned binary assets and nondestructive crop/rotation with stable center and durable resource references. These are established requirements for the next design; product implementation is authorized to resume after the initial gate.

## Known implementation gaps

- Limited control catalog and property states; text markup and typography incomplete.
- Nested groups, reusable components, image assets and alternate versions incomplete.
- PNG uses bundled fonts; SVG text may depend on fonts installed in the viewer.
- Recovery exit cleanup and last-moment shutdown changes need investigation.
- Native macOS package refresh script exists. Windows/Linux runtime checks and release archives remain unverified.

## Evidence

Product-only validation files may live in evidence/ and fixtures/ locally. Keep build outputs in target/ and dist/, temporary logs in work/. These are ignored by Git unless specific distributable examples are intentionally added.

## Fidelity phases

1. Editor workspace: compact dark toolbar, Quick Add, horizontal categorized control previews, left page thumbnails/list, contextual right inspector, plain white canvas, panel visibility shortcuts.
2. Control coverage and appearance: catalog inventory, state-specific properties, text formatting, eight resize handles, layering and snapping behavior.
3. Document workflows: nested groups, reusable symbols and overrides, image assets, alternate versions, page navigation, presentation and export.
4. Persistence and release: autosave/recovery, available platform verification, packaging, and recorded gaps. Existing-format compatibility stays deferred.


Version/storage requirements include stable page identity with independent alternate trees, recoverable promotion, additive merge with new control IDs, and a local working copy separate from debounced user-file synchronization. Dirty state must track revisions and preserve pending work on save failure.

Group resize must respect each descendant control’s axes, minimum sizes and anchoring while crop remains a separate mask. Component instances need sparse override paths across nested definitions, reset-to-inherited behavior, dependency invalidation and bounded recursion. Reopen acceptance must compare live appearance and selection bounds, not just serialized fields.

Output requirements include transparent/opaque 1x/2x PNG, real PDF resource/URI annotations, content or paper sizing, per-page orientation, optional names/numbers/notes and font fallback. Presentation requires target resolution, history and viewport-aware hit regions. Keep output clipping contracts explicit across raster and vector paths.

Catalog requirements include separate stable IDs, display names, aliases and shape variants; typed inherited properties; context-dependent font/theme defaults; sparse explicit overrides; and distinct automatic, fixed and measured dimensions. Leaf resize axes and computed container geometry must remain separate. These requirements are included in the implementation acceptance plan.

Editing requirements distinguish center distribution from equal gaps, preserve endpoint geometry, support toggle selection and full-enclosure marquee, remember duplication displacement, and separate selected from movable controls. Snapping needs independent axes, center/edge/proximity priorities, drag tolerance and zero keyboard attraction. Keep lock behavior explicit per operation.

## Implementation transition — 2026-10-13

The initial behavior and architecture gate is met. The user already authorized automatic progression. Begin the recursive document and typed-property redesign, preserving the provisional application and existing product files. This decision does not establish visual fidelity, complete catalog behavior or cross-platform runtime acceptance. Remaining checks stay assigned in docs/IMPLEMENTATION-PLAN.md. Goal remains active.

Baseline before foundation migration: `cargo test --locked` passes all 14 tests (3 application tests and 11 workflows). This confirms only the provisional behaviors. That baseline preceded the recursive document checkpoint below.

## Recursive document checkpoint

Version-2 data/storage API now supports recursive groups, scoped paths, typed sparse values, automatic/fixed dimensions, component definitions and nested overrides, crop metadata, embedded assets and independent alternate trees. Legacy migration preserves all 24 kinds and interleaved group layering. Missing/cyclic references, malformed data and excessive expansion are bounded. A depth-order regression was found and fixed.

Verification: 23 tests pass (the 14 existing checks plus nine document tests); Clippy passes with warnings denied. Read-only migration checks pass on five saved native test documents, with source bytes unchanged. The desktop editor still uses version 1. No version-2 visual/native behavior is claimed. Next integrate catalog property resolution and shared layout, then the editor, preserving unsupported data until the switch is complete. See docs/DOCUMENT-FORMAT.md.
