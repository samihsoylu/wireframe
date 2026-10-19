# Editor toolbar

The top strip displays the project title. On macOS, File, Edit, View, Window and Help use the system menu bar after successful native initialization. Other backends use in-window File, Edit, View and Help menus. File includes New page, New project, Close project and Quit; Edit includes Undo, Redo, Cut, Copy and both system and copied-control Paste actions; View includes Present. The real window title follows the project name; macOS also shows the represented file and edited state. The action row groups Pages and New wireframe on the left, Undo/Redo and zoom in the center, and Quick Add, Library, Comments, Inspector and Present on the right. Hover an icon to see its command. Active panel icons are blue; unavailable history actions are dimmed.

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

## In-window menu editing

Opening File, Edit, View or Help leaves an active canvas text draft provisional. Closing the menu returns to that draft. Edit commands operate on the original text field: Select all and Undo must not act on the drawing while a text draft or Quick Add owns editing. Commands from a closing popup are delivered on the following input frame, before newer input, with project/draft and blocking-dialog checks. Starting another canvas text draft cancels the queued action. Existing command enablement and save/close protection remain shared with native menus.

The fallback-menu checkpoint passes89 affected application tests on macOS (35 menu/toolbar/text checks repeated after the final validation-cost adjustment), including four new regressions for command availability, Quick Add selection, canvas draft selection/local Undo and rejected delivery after a replacement draft or intervening dialog. The existing draft/save regression now covers pointer and accessibility activation/cancellation of all four top-level menus. These are application event tests of the in-window backend, not native Linux/Windows menu or clipboard acceptance. External `fallback-menu-current` retains the failing checks, corrected results and candidate identity. Native Linux replay and final package refresh remain required; Windows testing is held.
