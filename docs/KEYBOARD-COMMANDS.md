# Keyboard command order

Opening a control's text editor focuses the field and selects its existing text. Return applies a single-line edit. Multiline controls retain Return for newlines and use Command/Ctrl+Return to apply. Escape cancels the draft. Input-method composition retains its confirmation keys; confirming a composition does not also submit the dialog. Local Undo stays inside the draft until it is applied, after which the entire edit is one document Undo step.

Document shortcuts run in input order, including multiple commands received within one frame. Undo followed by Save writes the undone state. Save followed by Undo keeps the saved snapshot and leaves the editor dirty. Repeated Undo, Redo and movement events each run once; key releases do not run commands.

Commands recheck modal state after each event. A new-project confirmation prevents later commands from bypassing that decision. Held library insertion and page dragging remain protected from document commands. Text editors and popups retain their own editing shortcuts, including local Undo; unhandled input remains available to the widgets.

Specific modifiers take precedence: Command/Ctrl+Shift+N adds a page, while Command/Ctrl+N requests a new project. Clipboard copy, cut and paste follow the same ordered document-command path.

The app tests save and reload real temporary files to verify both command orderings, repeated history, movement before and after Save, clipboard order, modal boundaries and modifier variants. Native platform behavior remains subject to the platform acceptance record.

When the page navigator owns focus, arrows and Page Up/Down traverse its visible rows, and modified selection retains closed descendant scope. Tab indents and Shift-Tab outdents the selected pages; F2 or Command/Ctrl+Alt+R opens the current page's rename dialog when no canvas controls are selected. Page-focused clipboard, Duplicate, Trash and Select All operate on pages. Text fields, popups and modal dialogs keep their own keys. Changed hierarchy commands use one document history step; no-op limits retain focus. See [navigator contract](PAGE-NAVIGATOR.md) for batch hierarchy behavior.

When a navigator-owned lifecycle/history action changes the current page, keyboard focus follows its resulting row. Delete, Duplicate and Undo/Redo are checked through the immediate subsequent command. A current-page change initiated with another widget focused does not steal that focus. These routes pass application event tests; native OS delivery, physical keyboard/accessibility and integrated acceptance remain separate.

## Layering

With canvas controls selected, Command/Ctrl+Up brings the selection forward across the next overlapping sibling; Command/Ctrl+Down sends it backward. Add Shift to move to the front or back. The inspector and Edit menus expose the same four actions. Steps skip non-overlapping siblings and gather selected siblings at the crossing; backward movement can change their internal order. Text fields keep their own arrow keys. Page/source navigators and component-instance descendants do not rearrange canvas children. Undo restores the complete prior order.

## Alignment and sizing

With canvas controls selected, Command/Ctrl+Alt+1 through 6 align left, center, right, top, middle and bottom. The inspector and Edit > Align menu also offer horizontal/vertical center distribution and equal spacing. Alignment requires at least two unlocked controls and uses only their bounds. Auto-size is Command/Ctrl+Alt+0; it resizes supported selected controls, including explicitly selected locked controls, as one undoable action. The inspector shows Auto-size only when every selected control supports it. Text fields, modal dialogs and focused page/source navigators retain ownership of these keys.

## Markup classification

Command/Ctrl+Shift+K toggles the selected controls' markup classification as one history action. Mixed selections become markup; all-markup selections become plain. Position locks preserve geometry while allowing classification. Empty selection, text focus and modal/held-gesture boundaries prevent unintended canvas changes. Native delivery remains separate acceptance work.

Command/Ctrl+K toggles editor markup visibility independently from classification. It changes per-project view settings without a document Undo entry. Focused text, modals, active gestures and presentation retain their input ownership. Native delivery remains unverified for this new route.

## Inspector pickers

Arrow navigation inside an open picker stays above background controls. Selecting or cancelling with Return, Space or Escape returns focus to that picker; pointer dismissal retains its destination. Navigation alone does not edit the document. Choosing a different value makes one Undo step, and cancelling preserves Redo. Application tests cover these boundaries and the native macOS Alignment route is verified; complete accessibility and platform scope remain in the release checklist.

The page navigator uses Tab for indentation. Command/Ctrl+J hides it when traversing to inspector fields; Command/Ctrl+L hides the library. Slash focuses Quick Add, from which Tab continues through the visible interface.
