# Keyboard command order

Document shortcuts run in input order, including multiple commands received within one frame. Undo followed by Save writes the undone state. Save followed by Undo keeps the saved snapshot and leaves the editor dirty. Repeated Undo, Redo and movement events each run once; key releases do not run commands.

Commands recheck modal state after each event. A new-project confirmation prevents later commands from bypassing that decision. Held library insertion remains protected from document commands. Text editors and popups retain their own editing shortcuts, including local Undo; unhandled input remains available to the widgets.

Specific modifiers take precedence: Command/Ctrl+Shift+N adds a page, while Command/Ctrl+N requests a new project. Clipboard copy, cut and paste follow the same ordered document-command path.

The app tests save and reload real temporary files to verify both command orderings, repeated history, movement before and after Save, clipboard order, modal boundaries and modifier variants. Native platform behavior remains subject to the platform acceptance record.
