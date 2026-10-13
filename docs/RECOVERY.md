# Local recovery lifecycle

The workspace keeps one atomically written recovery project under an exclusive process lock. Dirty documents are checkpointed on the five-second write interval; unchanged snapshots are not rewritten. Undo returning to the saved document removes the older recovery immediately. Save, explicit discard and successful project replacement clear the active snapshot.

The shutdown callback flushes the latest unsaved document, including changes newer than the interval. An explicit quit/discard prevents that callback from recreating discarded work. An unresolved startup recovery offer blocks recovery writes and survives loading a startup file or closing the window. Restore opens it as a new unsaved project and does not overwrite the source file. Start fresh explicitly removes the offer.

Failed writes retain the previous snapshot and are retried. Failed removal is reported in the status line and retains the in-memory snapshot marker for retry. These operations do not modify the saved project.

Four regression tests cover clean Undo followed by dirty Redo and immediate exit/reopen, pending-offer preservation, explicit discard, and failed recovery write/retry. Native macOS tests created a synthetic checkbox, observed its snapshot, undid to clean and verified file removal, redid and quit, reopened/restored the edit, closed an unresolved offer and verified byte preservation, then explicitly discarded the restored test through the normal close prompt. The full suite has 74 passing tests and warning-free Clippy.

Limitations: forced process termination can lose changes since the latest completed checkpoint. Native macOS Cmd-Q currently exits without the app's Save/Discard/Cancel prompt; the shutdown callback preserves the current document as recovery. Window close shows that prompt. Native quit-dialog integration, unreadable recovery-file handling and Windows/Linux lifecycle checks remain acceptance work.
