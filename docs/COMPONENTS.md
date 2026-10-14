# Reusable components

Select one control or group and choose **Make component** in the inspector. The Components sidebar lists definitions. Click a name to edit its shared source or + to insert an instance. **Back to wireframe** returns to the page. Shared source edits update instances throughout the project.

Double-click an instance, press Enter on it, or choose **Edit this instance** to work on local children. Enter nested groups the same way. The toolbar identifies local editing; Escape or Exit component level returns one level. Text, appearance, movement and supported size edits affect only that instance. The inspector marks locally changed fields and offers **Reset** to inherit the source again.

**Reset local changes** clears all local changes on a selected child or group. **Reset instance overrides**, available on a selected instance outside its contents, restores the whole instance. Reset leaves the source intact and can be undone. **Detach component** converts an instance to an ordinary group with its current effective values.

Source definitions retain control structure. Add/delete/group/reorder operations inside an instance report that the source must be edited. Copying a local child preserves its current effective values; leave the instance before pasting new content. Locked instances or source children cannot be locally edited.

## Data and verification

The editor locates the first canonical instance crossed by the canvas context. Every descendant edit is stored relative to that instance, even through further nested component boundaries. Read-only resolved nodes supply effective properties and geometry; they never replace canonical definitions. Individual properties compare against inherited component values, including nested overrides, rather than global defaults. Reset removes the local patch field and resumes inheritance. Unrelated and obsolete stored paths survive ordinary edits.

Canvas, thumbnails and exports use the same resolved layout. History captures context and selection as well as the document. Tests cover deep paths, two levels of competing inheritance, explicit defaults over nondefault inherited values, per-property/subtree/whole-instance reset, geometry, copy, locked edits, structural guards and storage. Native macOS checks cover local text, shared propagation, reset/history and reopening. See SESSION-SPEC.md for current totals and evidence scope.

Some crop/lock/resize combinations, large-project performance and Windows/Linux native acceptance remain open. Existing-format import is deferred.

## Resizing

Whole instances now resize through recursive geometry overrides without altering definitions. Nested instances inside ordinary groups follow the same transformation. Per-control resize axes preserve fixed dimensions and edge anchoring, including a horizontal-only label at the bottom. Fonts and existing local text/property overrides remain unchanged. Resizing records descendant positions and resizable dimensions explicitly; computed dimensions retain their sizing rules. Simple movement does not create geometry overrides. Reset instance overrides restores inherited geometry and content.

Native macOS resized one of two synthetic instances265×113→530×226, verified its bottom label and unchanged local text/other instance, exercised resize Undo/Redo and whole-instance reset/Undo, then saved/quit/reopened with X50,Y50,W530,H226 intact. Saved definitions match the original fixture. Numeric position/size entry now applies completed text on focus loss/Enter, avoiding destructive intermediate resizing while typing; pointer dragging remains live. Three regression tests cover nested instances/groups, fixed-height anchoring, frozen geometry/source changes, source preservation, reset/history/storage and locked/move behavior. Evidence: evidence/component-resize/. Typography, minimum-size, crop and locked-descendant edge cases remain open; no Windows/Linux native or independent review claim is made.

## Deleting a source

Open a component source, clear the selection, and choose **Delete component source** in its inspector. Existing instances remain as150×150 missing-component placeholders. Their references and local changes are preserved in the saved file. Undo restores the source during the current session; reopening does not retain Undo history. Missing instances can move, copy or be deleted, but their size is fixed and source editing/reset/detach is unavailable. A parent with missing descendants cannot be detached. Copying missing references assigns fresh missing IDs to prevent unrelated destination components from being substituted.

Native macOS checks cover deletion, Undo restoring resized/local content, Redo, saving and reopening. Four regression tests additionally cover nested references, alternates, crop preservation, context recovery, shared SVG/PNG rendering, allocator/clipboard collisions and no-op Ungroup. See SESSION-SPEC.md for scoped evidence and remaining acceptance.

## Empty components and groups

Removing every source child keeps the definition and shows an **Empty component** frame on its instances. Add new source content to populate them again. Local changes for removed child IDs remain stored but do not attach to fresh children. Empty instances can be detached into ordinary empty groups. Empty groups remain selectable and editable; enter them and add a first child through the library or Quick Add. Empty fallback dimensions are150×150. Position, lock state and stored crop data remain intact; inherited ancestor clipping still applies.

Empty/populated transitions preserve explicit layer position. Older files may omit empty groups in their paint order; they load unchanged and newly visible empty groups draw last. New edits include them as paint units. Root pages and source workspaces with no content remain blank. Native checks and regression coverage are recorded in SESSION-SPEC.md.
