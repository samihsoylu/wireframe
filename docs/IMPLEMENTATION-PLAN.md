# Implementation and acceptance plan

The product will provide a complete local wireframing workflow on macOS, Windows and Linux. The document/editor foundation and substantial workflow implementation now exist. The milestones below retain the required scope; the [release checklist](RELEASE-CHECKLIST.md) records actual completion gates and remaining work. Original code remains GPL-3.0-only; preserve license notices for redistributed dependencies and independently sourced assets.

## Milestone 1: document foundation

Introduce a versioned document representation with ordered recursive nodes, parent-local transforms, stable identity and scoped node paths. Keep automatic/fixed dimensions separate from measured layout. Resolve effective typed properties from catalog defaults, project context, explicit values and instance overrides.

Represent component definitions separately from instances. Instance overrides address descendant paths and can be reset; materializing an instance preserves effective values. Validate missing/cyclic references with bounded recursion. Keep image bytes in project-owned assets, with nondestructive crop and rotation. A page retains its identity across independent alternate content versions.

Migrate existing version-1 files without losing controls, groups, order, names or explicit values. Loading invalid/unsupported content must leave the active document intact. Test nested movement/ungrouping, override propagation/reset/materialization, independent alternates, validation and round-trip migration before integrating the new model into the editor.

## Milestone 2: shared layout and rendering

Build rich-text and structured row/cell/tree representations. Preserve source text while deriving layout, link areas and hit regions. Compute requested, measured, user and clip bounds distinctly. Use original deterministic sketch geometry and redistributable fonts/icons, with licensed provenance recorded where required.

The canvas and exports consume one scene containing paths, text, images, clip stacks and link annotations. Verify representative text, state controls, tables, images and nested components visually before scaling to all control families. Cache derived layout by content/context revision and invalidate dependent instances.

## Milestone 3: catalog and editing

Cover all required control families, typed inspector properties, names/aliases/variants and library insertion. Implement nested editing contexts, toggle/full-enclosure selection, explicit lock policies, center/edge/proximity snapping, per-axis resizing and remembered duplication offsets. Center distribution and equal spacing are distinct commands. Use transactions so previews, text edits and compound operations have predictable cancel/undo/redo behavior.

Compare representative normal and edge cases for each family. No generic placeholder renderer counts as a completed control. Include keyboard use and readable accessible UI labels.

## Milestone 4: workflows and storage

Complete pages/hierarchies, components, assets, alternate promotion/merge, presentation navigation/history and target-aware hit regions. Export transparent/opaque 1x/2x images and PDF with actual text/link annotations, paper/content sizing and optional notes.

Maintain a recoverable working copy separately from the user's saved file. Track revisions across debounced saves; preserve pending changes on errors or shutdown. Verify failures, close/reopen, recovery and cancellation with synthetic documents.

## Milestone 5: usable release

Package and validate native macOS, Windows and Linux builds. Verify real input/dialog/font behavior on each platform; portability claims and cross-compilation alone are insufficient. Check offline usability, startup, large-document responsiveness, license notices and user-facing recovery behavior. Record unavailable checks honestly. Existing-format import remains a later capability.

## Acceptance discipline

Each milestone requires focused automated behavior checks and native/visual checks where relevant. Retain unresolved bounds/clipping, lock-edge, presentation-hit, typography and platform discrepancies until they are resolved explicitly. Update current state and commit completed product changes. Initial readiness is not product completion.
