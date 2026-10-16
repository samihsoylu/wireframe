# Local recovery lifecycle

The desktop uses per-project working copies through the background project session. Changed projects are checkpointed after a one-second quiet interval in either automatic or manual saving mode. A named project keeps its saved-file identity and fingerprints alongside the latest local document. Automatic saving subsequently updates the named file; manual saving retains local edits without doing so. See [saving](SAVING.md), [working copies](WORKING-COPIES.md) and [project sessions](PROJECT-SESSION.md).

Startup and File → Recover work offer changed local projects and retained destination archives. Later keeps those copies available. Restoring work whose named file is unchanged keeps that association. If the file changed, disappeared or became unreadable, restoration opens an unnamed project; saving asks for a new destination. Restoring an archive also creates unnamed work while preserving the archive. Invalid recovery records remain available for diagnosis and produce errors.

Ordinary close, application quit and explicit discard have distinct behavior. Manual window close offers Save/Discard/Cancel; application quit checkpoints local work. Automatic close/quit requests a named flush. Explicit discard removes the active local record and prevents shutdown from recreating it. A blocked filesystem can delay shutdown. The exit hook cannot cancel an application exit after a save failure; retained local work is offered on restart.

## Abrupt process termination

macOS storage tests forcibly terminate a child writer before its new checkpoint, after the checkpoint, after the pending-write journal, after the named-file replacement and after acknowledgement. The tests first verify that a live process owns the record, then verify lock reacquisition after termination. Reopening retains the last completed local checkpoint and either the old or completed named snapshot, as appropriate to the boundary. A named snapshot older than the local document never replaces newer local edits. Interrupted acknowledgement reconciles a matching named file; modified, missing and invalid named files remain protected against ordinary Save.

Native macOS checks also forcibly terminated the full app after confirming its local checkpoint. Restart offered the edited four-page project; Restore and Save produced the complete expected project. In a second run, the named file was changed while the app was stopped. Restore opened the local version as unnamed work, Save requested a destination, and saving a separate copy preserved both versions. These checks used a disposable project and restored the original automatic-save preference.

These results do not guarantee recovery of edits before a completed checkpoint. They do not test power loss, filesystem/device failure, termination inside an atomic write, all Save As interruption points or native Windows/Linux crashes. Current atomic writes flush temporary-file contents before replacement; Linux also syncs the parent directory. Platform and filesystem durability remain separate from process-termination evidence.

## Legacy recovery and fallback

The former single `recovery.wireframe` file remains supported for migration and fallback when per-project persistence is unavailable. An unresolved legacy offer stays intact while other work proceeds. Restore creates a durable unnamed working copy before removing the legacy file. The legacy five-second checkpoint routine does not run while per-project persistence is active.

An unreadable legacy file is moved to a unique `unreadable-recovery-*.wireframe` beside its original location before the slot is reused. Original bytes are preserved, existing archives are not replaced, and the startup notice gives the location. If preservation fails, the original stays in place and automatic legacy recovery is disabled for that session with a manual-save notice. An OS lock prevents competing owners. Earlier native and automated checks cover malformed legacy files, archive preservation failures, pending offers, clean Undo/dirty Redo, discard and retry; these are fallback checks rather than a description of the current primary workflow.
