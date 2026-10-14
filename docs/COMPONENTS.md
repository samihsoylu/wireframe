# Reusable components

Select one control or group and choose **Make component** in the inspector. The Components sidebar lists definitions. Click a name to edit its shared source or + to insert an instance. **Back to wireframe** returns to the page. Shared source edits update instances throughout the project.

Double-click an instance, press Enter on it, or choose **Edit this instance** to work on local children. Enter nested groups the same way. The toolbar identifies local editing; Escape or Exit component level returns one level. Text, appearance, movement and supported leaf size edits affect only that instance. The inspector marks locally changed fields and offers **Reset** to inherit the source again.

**Reset local changes** clears all local changes on a selected child or group. **Reset instance overrides**, available on a selected instance outside its contents, restores the whole instance. Reset leaves the source intact and can be undone. **Detach component** converts an instance to an ordinary group with its current effective values.

Source definitions retain control structure. Add/delete/group/reorder operations inside an instance report that the source must be edited. Copying a local child preserves its current effective values; leave the instance before pasting new content. Locked instances or source children cannot be locally edited.

## Data and verification

The editor locates the first canonical instance crossed by the canvas context. Every descendant edit is stored relative to that instance, even through further nested component boundaries. Read-only resolved nodes supply effective properties and geometry; they never replace canonical definitions. Individual properties compare against inherited component values, including nested overrides, rather than global defaults. Reset removes the local patch field and resumes inheritance. Unrelated and obsolete stored paths survive ordinary edits.

Canvas, thumbnails and exports use the same resolved layout. History captures context and selection as well as the document. Tests cover deep paths, two levels of competing inheritance, explicit defaults over nondefault inherited values, per-property/subtree/whole-instance reset, geometry, copy, locked edits, structural guards and storage. Native macOS checks cover local text, shared propagation, reset/history and reopening. See SESSION-SPEC.md for current totals and evidence scope.

Whole-instance resizing, definition deletion, some crop/lock/resize combinations, large-project performance and Windows/Linux native acceptance remain open. Existing-format import is deferred.
