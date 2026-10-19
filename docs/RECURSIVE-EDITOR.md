# Recursive editor

The desktop app edits the canonical recursive Project directly. The current writer uses format15; versions1–14 migrate through the shared decoder. See [document format](DOCUMENT-FORMAT.md). The workspace module owns transactions, history, page/context selection, structural clipboard and tree transformations. Rendering receives disposable control projections; saving and recovery always serialize the original recursive project, including properties/resources that the current inspector does not expose.

Groups have their own identity and parent-local children. Enter, double-click or Edit group contents selects a nested editing context. Escape and Exit group move to its parent. Grouping preserves existing paint order, including nonadjacent selections. Ungrouping preserves world coordinates and repairs sibling identity collisions. Removing a cropped group reveals the previously hidden content; crops on descendants remain intact. Other selected controls and locked groups keep their selection. Empty groups can be dissolved without leaving invalid paint-order entries. Undo restores the original crop, hierarchy and selection. Movement, cancellation and undo/redo retain descendants. Resizing uses catalog axes for leaf sizes. Full-enclosure marquee excludes locked nodes; hit testing follows visible leaf order and clips.

Clipboard data includes selected subtrees and referenced definitions/assets. Matching dependencies already present in the target are reused; imported resources receive new IDs. Root node IDs change while scoped descendant/override paths remain stable. Component instances are atomic entries in explicit paint order, with their expanded descendants kept together. Missing page links are cleared when importing clipboard data.

ProjectRepository separates app workflow from disk storage. ProjectStore uses the shared decoder and writes the current validated format atomically. Opening preflights layout before replacing active work. Failed saves leave the document dirty and retain its contents. Recovery uses the same canonical decoder and exclusive ownership model.

Canvas, thumbnails and SVG/PNG exports use a shared scene with inherited clips. Catalog-specific rendering, shaped text and layout now cover the implemented controls; see [canvas coverage](CANVAS-COVERAGE.md) and [rich text](RICH-TEXT.md) for their limits. Opening still validates and preflights layout before replacing the active project. Unknown types or invalid layout fail without replacing visible work.

Native macOS verification used a copy of a saved two-control fixture. The app created two nested group levels, edited one child, moved it ten units, undid/redid, saved, closed and reopened. Re-entering the hierarchy showed the persisted X=338 coordinate; the file retained four leaves inside three group nodes. Screenshots, accessibility snapshots and the saved fixture are retained locally.

The initial recursive-editor checkpoint passed98 tests; that is historical evidence, not the current suite total. New checks cover hierarchy/context round trips, transactional movement/cancel, lock/enclosure selection, nonadjacent grouping, failed edit rollback, clipboard dependencies, resize-axis handling, crop preservation, scoped layering/hit testing and atomic component paint order. Raster crop checks inspect the output dimensions and pixels inside/outside the mask. An injected disk-full failure verifies the app retains dirty state. Five saved version-1 fixtures retain geometry, order, text and source bytes through migration and editor projection. Clippy is clean with warnings denied.

Component deletion, detailed crop/lock/resize policies, typed inspectors and asset/version workflows now have their own implementations and acceptance records. This document retains historical scoped native examples; it does not assert current process liveness or whole-product completion. Remaining behavioral/platform limits are tracked in [release gates](RELEASE-CHECKLIST.md).

Typed text/table properties and six-state choice rendering are integrated; see INSPECTOR-AND-STATES.md for the current field boundary and native evidence.

## Component workspaces

Components now have a sidebar and source workspace. Select one control or group and choose Make component; use the sidebar + to insert another instance, click its name or Edit component source to edit shared content, and Back to wireframe to return. Source edits use existing nested editing, typed properties, clipboard and history. Detach component materializes current effective values into an ordinary group. Validation rolls back cyclic insertion; locked conversion/detach and interleaved group conversion are rejected without changing the document. Local instance editing and reset are now available; see COMPONENTS.md.

Native macOS verification created a component from a form group, inserted a second instance, renamed its source to Contact row and changed its input text to Email address; both instances updated. Detach, Undo and Redo passed. A later source change to Contact email affected only the linked instance. Save, quit and reopen preserved the linked instance and detached group's different values. Evidence: evidence/components/. Three regression tests additionally cover source grouping/movement/clipboard, cycle rollback, locks, paint order, overrides, history and storage. That historical development bundle used the synthetic fixture; this is scoped self-verification, not independent review or current process status.

Local component editing now works in place: double-click/Enter/Edit this instance enters the instance, and further entries reach nested children. Typed property changes are sparse overrides on the owning instance and stable descendant path; source definitions stay unchanged. Reset beside a field resumes inheritance. Reset local changes clears a selected child subtree's overrides; Reset instance overrides clears the whole instance. Source changes propagate through nested definitions while overridden values stay local. Movement, leaf resizing, Auto-size, clipboard copy and Undo/Redo use resolved geometry with stored local patches. Structural changes are directed to the source workspace. Position locks reject direct geometry/destructive changes while allowing supported content edits and Auto-size; see [locking](LOCKING.md).

Native macOS checks changed one of two instances to Personal email, reset it, undid/redid, then edited the source to Shared email. The local instance retained Personal email and the other updated. Whole-instance reset displayed Shared email on both; Undo restored the local value. Save/quit/reopen and re-entry retained Personal email with its highlighted field and Reset. Evidence: evidence/component-overrides/. Four regression tests add deep paths, competing nested inheritance, explicit catalog defaults over nondefault source values, reset/history/storage, geometry/copy/guards and projected text/choice state edits. That historical development bundle used the saved fixture. No Windows/Linux native or independent review claim is made.

Whole instances now resize through recursive geometry overrides without altering definitions. Nested instances inside ordinary groups follow the same transformation. Per-control resize axes preserve fixed dimensions and edge anchoring, including a horizontal-only label at the bottom. Fonts and existing local text/property overrides remain unchanged. Resizing records descendant positions and resizable dimensions explicitly; computed dimensions retain their sizing rules. Simple movement does not create geometry overrides. Reset instance overrides restores inherited geometry and content.

Native macOS resized one of two synthetic instances265×113→530×226, verified its bottom label and unchanged local text/other instance, exercised resize Undo/Redo and whole-instance reset/Undo, then saved/quit/reopened with X50,Y50,W530,H226 intact. Saved definitions match the original fixture. Numeric position/size entry now applies completed text on focus loss/Enter, avoiding destructive intermediate resizing while typing; pointer dragging remains live. Three regression tests cover nested instances/groups, fixed-height anchoring, frozen geometry/source changes, source preservation, reset/history/storage and locked/move behavior. Evidence: evidence/component-resize/. Typography, minimum-size, crop and locked-descendant edge cases remain open; no Windows/Linux native or independent review claim is made.


## Group crop editing

Select one unlocked, nonempty group or component and choose **Crop selection**. The editor displays the complete artwork with descendant crop masks preserved. Move the frame by dragging inside it; resize with any of its eight handles. Group crop handles have a 10-unit minimum, limited by the full artwork size. Drag outside the frame to move the artwork beneath a fixed frame. The frame remains inside the artwork.

The initial region is the existing crop or a rounded 80% of the group's full bounds. Preview changes are transient: Enter, Apply crop or a double-click applies one Undo step; Escape or Cancel crop discards them. Document shortcuts are blocked during preview. Full artwork and background are separate scenes, so a moving preview leaves no stale copy behind. Existing image crop editing uses the same editor and preserves its image-specific sizing rules.

**Remove crop** expands from the visible top-left, clamped to the parent workspace's nonnegative origin; child geometry and descendant crop masks remain unchanged. This differs from Ungroup, which reveals the children at their stored positions. Crop and content translation are one history operation. Editing inside a component writes local overrides; removing an inherited crop writes an explicit clear, while **Reset crop to source** restores the inherited mask and keeps the current visible top-left fixed. It adjusts the stored origin to compensate for the mask offset, without resizing children or changing unrelated local patches. Reset is one reversible operation; repeating it without a crop override makes no change. The source and sibling instances remain unchanged.

Focused checks cover nested crop geometry, relative paint order, isolated previews, component clear/reset, locks, empty containers, atomic history and save/reopen. Pointer tests cover frame movement at 50%, 100% and 200%, minimum size, content movement, Apply, cancellation and command blocking. Native platform acceptance is tracked in SESSION-SPEC.md.

## Resize minimums and locks

Resizing an unlocked group or component transforms all descendants, including children whose direct position/content editing is locked. Their lock flags remain set. Scalable control dimensions stop at 10 units; arrows retain a 1-unit minimum. Fixed and computed dimensions retain their size. Moving a parent without resizing preserves existing smaller stored geometry.

A cropped parent's frame follows the actual union after child size limits apply. Its resulting size can exceed the requested size, and the normalized crop offset can shift the visible corner. The crop mask remains unchanged. Component resizing writes local geometry overrides and preserves its source and other instances. Undo/Redo restore exact document snapshots. Broader nested fixed-axis combinations remain acceptance work.

The inspector and drag handles use the catalog minimum. Selecting an existing subminimum value does not clamp it. Generic drag handles preserve the opposite edge when a minimum is reached, including aspect-preserving resizing.


## Coordinate rounding

Explicit resizing rounds transformed leaf positions and scalable dimensions to whole units; half ties go toward positive infinity. Parent displacement is included before rounding in the original parent coordinate system. Group coordinate anchors remain stable during resizing, and their visible bounds follow the transformed child union. This avoids accumulating a different rounding error at each group level. Component instances use the same geometry policy without changing their source or siblings.

Stored fractional geometry remains valid and survives loading, selection and move-only edits. Fixed and computed axes retain their existing size. Group crop origins and dimensions round separately in derived layout, without rewriting the normalized mask. Leaf/image crops retain their existing policy.

A very small group crop can resolve to zero width or height. Its content stays intact and selectable through Layers or Select All. Zero-size axes have no resize handle or editable size field; Remove crop restores the content at the visible corner, with exact Undo/Redo. Dragging the remaining nonzero axis uses finite scale factors.

Resize previews derive each frame from the transaction-start snapshot. Rounded descendants must not accumulate error as pointer samples arrive; direct and multi-frame movement to the same endpoint produce identical geometry. Release records one history entry, returning to the start records none, and focus loss cancels the preview.

## Fixed axes inside cropped groups

Recursive resizing anchors fixed dimensions against each group's visible crop frame, including descendants outside that frame. For example, enlarging a 100×50 crop of 100×100 artwork to height100 moves a fixed-height label from local Y80 to Y160 and keeps its height20. Using the full artwork as the anchor would incorrectly move it to Y180. Normalized crops remain unchanged, and ordinary groups and component instances use the same rule.

The regression covers direct/nested groups, ordinary/component content and locked/unlocked labels, with exact history and storage round trips. Native macOS additionally verified a nested component with a locked label, unchanged source/sibling, exact Undo/Redo and save/quit/reopen. Broader fixed-axis controls and nested clipping combinations remain acceptance work; the offset-crop checks below extend this checkpoint. This is scoped macOS verification, not native Windows/Linux acceptance.

## Descendant-aware interactive resize limits

Numeric size fields and resize handles use separate width and height limits. A selection takes the largest applicable minimum on each axis across its resolved descendants. Scalable axes use the catalog minimum (normally10, arrow1); fixed axes retain their measured size. Locked children and children outside crop masks still contribute. Local component overrides and the current nested editing context are included. Empty or missing containers retain their fixed frame. These are interactive limits; selecting or loading smaller stored geometry does not rewrite it.

For a nested 100×100 offset crop containing a height20 label, entering height10 stops at20. The label stays at the visible bottom while its left half remains clipped. Enlarging that crop to height200 also retains its corner and fixed label height. Pointer checks cover top/bottom anchors for ordinary groups and component instances; asymmetric limit checks cover all eight handles with and without aspect preservation. Native macOS verified numeric clamping, source/sibling preservation, exact history and reopen. Other nested clipping/hit-target combinations and native Windows/Linux checks remain open.

## Editing context visibility

Entering a group or component reveals its own crop and all ancestor crops. Descendant groups retain their masks until entered. Selection uses the same revealed regions; unrelated controls remain outside the active selection scope. Unrelated artwork dims with nesting depth, down to a 10% floor. Leaving the context restores the applicable masks and emphasis without changing document geometry, crop values or history.

Emphasis accounts for each unrelated control's parent depth. At editing depth three, siblings whose parents are at depths two, one and zero appear at 40%, 20% and approximately 13.3% opacity. Unrelated branches at or below the current depth use their own parent depth to determine fading. Related descendants remain opaque. Component instances follow the same path-depth rule. Regression coverage includes nested sibling levels, deeper unrelated branches, the 10% floor and exact context-cache restoration without document or history changes. Complete controls are composed before applying context opacity, so overlapping border/fill/text/image parts do not darken each other. Intrinsic part transparency remains separate. Canvas meshes and existing textures are reused at their original coordinates; a reusable GPU tile per opacity nesting level uses at most4 MiB of texture storage and covers larger visible regions without image scaling. Siblings reuse the same depth surface; peak allocated depth is retained until the renderer closes. This is a texture bound per depth, not a total process-memory bound. SVG/PNG and PDF use corresponding opacity scopes. Native macOS checks cover three depths, fractional zoom/pan, masks and both tile axes, within one channel value of same-position framebuffer baselines. Saved values and canonical scenes remain unchanged. Older or unavailable GL contexts use an explicitly reported approximation; other drivers/platforms and large-document performance remain unverified.

The main editor uses a transient context-keyed scene. Page thumbnails, export and presentation use the canonical fully cropped scene, even when presentation starts inside a group. Hidden links remain inactive in presentation. Content changes invalidate both scene variants; changing context cannot reuse a scene from another depth or surface. The cached transient scene is bounded to one context.

Regression checks cover ordinary groups and nested components, source and crop preservation, cache changes after local edits, Undo/Redo and serialization. Native macOS verified two nested masks, selecting a revealed button, returning to page scope, and clipped versus visible presentation links with exact saved-file preservation. Whole-group links are described below. Entering an empty container hides only its own placeholder in the transient canvas. The outside placeholder stays selectable, and canonical scenes retain it. Full-path tests cover ordinary groups and empty component instances, insertion/deletion/Undo/Redo, blank raster output and unchanged canonical scenes/document/history. Native macOS verifies ordinary groups at page and nested levels, toolbar Undo and saved-value preservation. Empty-container origin/reflow and styling, broader overlapping masks, native component visibility and native Windows/Linux remain unresolved.

## Group page links

An unlocked group or component can link to a page through its inspector. Its visible rectangular bounds, including empty interior space, own that target. Parent links take precedence over links on descendants; clearing the parent link restores child navigation. Ancestor crop masks restrict the target, and later controls obscure covered regions. The shared scene supplies the same regions to presentation and exports without adding visible artwork.

Nested component edits write local property overrides; the source and sibling instances stay unchanged. Reset local changes restores inherited links. Existing lock, history and storage rules apply. Group links are placed above the group's last descendant in the document's paint order; interleaved noncontiguous groups need broader native acceptance. The inspector currently assigns page targets; broader link authoring remains future work.

Regression checks cover blank-space and child priority, nested masks, later-control occlusion, component overrides/reset/locks, history, serialization and zoomed presentation navigation with back history. Native macOS verified inspector assignment, both hit areas, Undo restoring the child destination, Redo and exact save/quit/reopen. Native Windows/Linux and broader presentation acceptance remain open.


### Nested opacity composition

Native opacity groups remain structured until the complete scene has closed its
opacity and mask scopes. An enclosing mask recursively clips the contained
meshes, including nested groups. At rendering time each inner group is composed
on a separate reusable surface before applying its opacity onto its parent.
This preserves strokes, caption backgrounds, overlaps, texture coordinates and
clipping when an already faded control is dimmed outside the current group.
The renderer restores the caller's framebuffer after drawing.

Headless or unsupported GL rendering preserves the nested content with per-part
fading as an approximation. If allocating an extra depth surface fails, native
rendering likewise approximates inner fading using the existing outer surface;
allocation failure has not been injected during validation. Exact nested GPU
composition is covered at standard/Retina density by a three-level pixel test
with overlaps, a mask around a nested group, inner clipping, tile boundaries and
later drawing. Native editor group-entry acceptance is recorded in the current
release checkpoint; held quick-draw preview and other drivers/platforms remain
separate checks.

## Current editor requirement reconciliation

The editor requirement ledger maps43 requirements across recursive editing, locking, alignment, hidden markup, insertion, quick drawing, canvas scrolling and viewports to172 named tests passing in the retained macOS/Linux logs. All43 inspected implementation/test files match both the broad platform snapshot and the latest export-guard snapshot exactly. Existing native evidence keeps its original case/platform/candidate scope; no new native run or repeated test suite was needed for this audit. Evidence: external editor-requirements-current/requirements.csv, source-reuse.json and test-evidence.json.

Native modified held drag, physical smart zoom, quick-draw held-key/cursor/cadence, broader edge scrolling, paired nonzero crop offset and other driver/accessibility combinations remain explicit. Component/resource lifecycle, control-by-control catalog coverage, panels/toolbars and broader inspector requirements are separate reconciliation work; this ledger does not close their parent gates.
