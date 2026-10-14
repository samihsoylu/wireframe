# Adding controls from the library

Drag a control onto the canvas to preview its placement before releasing. The preview uses the project drawing style, font and current zoom. The pointer marks the control origin, preserving the existing placement convention. The library tile stays highlighted while held.

Release over the canvas to insert and select it. Undo removes that insertion in one step. Escape, a loss of app focus, or releasing outside the canvas cancels without editing the project. Double-click still inserts a control at the default position.

The preview is transient: it never enters the document, undo history, recovery file or export. Release commits before canvas painting to avoid an empty frame between preview and placed control.

Validation covers actual egui pointer events across frames, preview paint bounds following the pointer, placement at 50/100/200% zoom with panning, small pre-drag movement, Escape/focus/outside cancellation, one-frame coalesced drags and one-step history. Native macOS validates drop, Undo/Redo, outside rejection and double-click. A held native preview capture and native Windows/Linux testing remain outstanding.
