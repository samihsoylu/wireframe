# Editor toolbar

The top strip displays the project title. On macOS, File, Edit, View, Window and Help use the system menu bar after successful native initialization. Other backends retain in-window File, Edit, View and Help menus. The real window title follows the project name; macOS also shows the represented file and edited state. The action row groups Pages and New wireframe on the left, Undo/Redo and zoom in the center, and Quick Add, Library, Comments, Inspector and Present on the right. Hover an icon to see its command. Active panel icons are blue; unavailable history actions are dimmed.

The four zoom icons are Zoom out, Zoom in, Actual size and Fit page. Double-click Actual size or Fit page to apply it to every page. The current zoom percentage appears in the bottom status bar. View also offers zoom and panel commands, Grid and Snap to 10. File contains Save, Sketch style and Font and colors. Existing keyboard shortcuts remain available.

Editing a component library or source replaces the page-navigation actions with Back to wireframes and New component library in the same row. Editing a nested group or instance retains its contextual exit action below the main toolbar. Long project titles are bounded to preserve the menus. The action row is checked at widths from 860 to 1920 logical pixels.

## Verification and limits

The toolbar has scoped macOS native checks for zoom, insertion, history, saving, group exit and presentation, plus automated layout and routing checks. The macOS native menu backend is implemented, with scoped native command and active-text Save acceptance. Native menu clicks have scoped checks for text and canvas history, zoom, Paste followed by Save, represented file and Quit protection. Native dialog menu-click editing is checked; filename keyboard accelerators remain under investigation. This is a focused toolbar improvement; it does not establish complete visual parity or Windows/Linux runtime acceptance. The toolbar and Unicode wrapping correction are integrated in the optimized macOS review build. Package and platform acceptance are tracked separately.

## Command ownership

Native menu callbacks enqueue input through the existing window backend. Commands are revalidated during dispatch, so stale menu states cannot bypass pending saves, dialogs or held gestures. Text fields retain local editing operations. Native Open/Save/font/image/export pickers temporarily use AppKit responder-chain editing, then restore editor commands on return. All prior in-window File/View operations remain reachable. macOS Window actions use standard platform responders; this does not implement multiple document windows.

The latest source-mode layout acceptance and remaining visual differences are tracked in [Toolbar fidelity](TOOLBAR-FIDELITY.md).

## Quick Add input ownership

After inserting a result, clear the query and keep keyboard focus in Quick Add. Command/Control-Z and Redo first use available query-text history. When that history is exhausted, the same command reaches document history; crossing to document history ends the old query redo branch. Other text fields retain their existing local-only history policy. Menus use the same ownership rules as shortcuts.

Escape clears a nonempty query and keeps focus. With an empty query, Escape leaves the field without clearing the canvas selection or leaving a component source. An active input-method composition owns cancellation before either action. Query edits, insertion and history commands in a single input batch must run in order, including Save boundaries. Plain Return in an empty query opens text editing only for one eligible selected control. Empty or multiple selections, groups and non-text controls keep their selection, editing context and query focus. Closing that text editor returns focus to Quick Add. Placeholder conversion remains a separate Undo step even when the text draft is cancelled. Composition owns Return before this route; nonempty-query Return retains insertion behavior.
