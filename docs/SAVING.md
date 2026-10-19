# Saving and recovery

Save chooses a `.wireframe` file for a new project. After the first save, automatic saving updates that file after edits settle: a local checkpoint follows a one-second quiet interval, then the named-file write waits five seconds. File → Automatically save named projects turns this on or off; the preference persists. Local recovery continues in either mode.

Save and Save As run through the background worker. The app waits for confirmation before completing New, Open or window close. A failed save keeps the document open with Save, Save As, Discard changes and Cancel available. Save As can explicitly replace or recreate the selected file; ordinary saves reject files changed or removed outside this window. A background acknowledgement applies to the document it actually wrote, so newer edits remain unsaved.

Ordinary Save writes committed document changes while leaving an active canvas text draft open and provisional. Continued typing remains in that draft, and Escape cancels only the draft. Save As accepts the canvas draft before opening the destination chooser; cancelling the chooser retains that accepted edit in document history. Inspector and other document-field input is applied in event order before the save snapshot; later input remains a later edit. The distinct text-session boundaries and remaining input limits are documented in [text entry](TEXT-ENTRY.md).

In manual mode, closing a changed window asks whether to save. Application quit retains local work. In automatic mode, both ordinary window close and application shutdown request a named flush. The shutdown hook cannot cancel an application exit after a save failure; retained local work is offered on restart. A stalled filesystem can delay shutdown. This is not a power-loss guarantee.

Recovery copies appear at startup and through File → Recover work. Later keeps them available. Restoring unchanged named work keeps its file association. If the file has changed, disappeared or become unreadable, recovery opens a new unnamed document so the other version is not overwritten. Opening a changed named file instead retains different local edits as a separate recoverable copy. Previous destination records retained during Save As can also be restored as unnamed projects. Invalid copies remain in place and produce an error.

The previous single recovery file is migrated only after a new per-project copy has been durably created. An unresolved legacy recovery offer stays intact while other work proceeds. Discard changes removes the active local copy and shutdown does not recreate it. Recovery archives are retained until managed outside the app; a recovery-copy management screen is not yet included.

Files stay on this computer. No account, network sync or external storage service is used by this workflow.
