# Local review comments

This is the implementation contract for review workflow G07. The core, project persistence/history/import integration, visible editor/presentation panel and canvas overlays are implemented. Bounded native macOS editor/presentation workflow and cold-reopen acceptance now pass; matched-reference comparison and remaining input/surface boundaries are listed below.

## User behavior

Discussions belong to a page or a specific alternate. Show newest root discussions first and oldest replies first within each thread. Support posting, replying, editing the reviewer's own non-imported comments, deleting, inline restoration during the current session, and read/like state. Preserve original attribution on imported feedback. A deleted parent remains as a deleted row when replies still exist; an isolated deleted leaf disappears after its session restoration opportunity ends. Deleted text and markers must not remain in saved tombstones.

Use a local reviewer identity; no account or network is required. Comments on an unsaved project participate in normal save/recovery. This is an intentional local-product adaptation. Do not require an email address. Comments are distinct from Notes and from the document Callout artwork control.

Keep one draft per page/alternate outside canonical document state. Switching pages must not post or discard drafts. Cancel changes neither comments nor drawing history. A removed/deleted reply parent invalidates that draft safely. Posting a reply marks its parent read. Text focus owns character keys, including presentation shortcuts, until the editor relinquishes focus.

## Review callouts

A comment may own numbered markers and refer to markers on its surface. Insertion places a spaced marker token at the text caret. Persist stable marker references, body text, position, tip and color. Editing a comment copies its body and markers into the draft; moving or changing a marker remains uncommitted until posting. Removing a marker's text reference removes that owned marker on commit.

Use page coordinates and the current canvas transform for hit-testing/painting. Marker positions denote their centers. Circle diameter is 40 points; left/right tip bounds 65×40, top/bottom 40×65, no tip 40×40. Default is left tip and blue; also provide green, yellow, red and gray with readable contrasting labels. Labels start at 1 and advance beyond existing labels on that surface. Resolve imported collisions without changing unrelated text links.

With no active marker or edited thread, hide review markers. Selecting a marker reference reveals and scrolls to that marker. Editing/replying reveals the thread's markers; an explicitly selected marker takes precedence. Ignore stale marker identities safely. These overlays never become artwork, page-link hotspots or ordinary image/PDF exports.

## Persistence and lifecycle

Version the format when review data is added. Validate surface ownership, identities, parent membership, acyclic/bounded threads, bounded text/data, finite coordinates and marker references. Old documents load with empty discussions. Malformed input must fail without changing the current document. Save/reopen/recovery preserves attribution, thread order, text, markers and read/like state; drafts and deletion restoration caches are session state.

Comment commands sit outside drawing Undo/Redo. A later drawing Undo must preserve feedback posted/edited/deleted since that drawing edit. Undo which would remove a newly created page/alternate carrying feedback must fail visibly without losing that feedback. Likewise protect later feedback when replay would replace imported review. Unchanged import history remains exactly reversible. Retaining a page in Trash retains its discussions; permanent deletion purges them from the live document and all retained history.

Duplicating a page or creating an alternate copies artwork/notes without copying discussions. Promote/merge preserve discussions on their original surfaces. Discarding an alternate removes its discussion with the alternate, with coherent history recovery; ordinary creation Undo must still protect newly added feedback.

Whole-project import includes comment-only pages. Keep Existing preserves the destination discussion. Use New replaces it with imported discussion and preserved attribution; Keep Both creates an isolated discussion on the new surface. Remap surface, parent, marker and alternate-link identities together. A combined page/asset/library/comment import is one guarded drawing-history action and must preserve exact Undo/Redo when no later feedback intervenes.

## Acceptance

Backend checks: old-format migration; validation failures; root/reply order and deleted-parent visibility; save/reopen; per-alternate scope; draft cancellation; history after posting/edit/delete; guarded resource removal; all import choices; exact import Undo/Redo; permanent removal with no resurrection.

UI checks: editor/presentation entry, empty/thread/composer states, attribution, reply/edit/delete/restore/read/like, caret marker insertion, marker reference navigation, drag/tip/color changes, cancel, alternate links, notes/comments resizing, text-focus shortcut isolation, settings/navigation return, save/cold reopen. Verify ordinary keyboard access and material visual fidelity on the release candidate. Backend tests do not establish native UI acceptance.

## Core checkpoint

`review::Discussion` now implements validated per-surface threads, author attribution, posting/editing/deletion/session restoration, read/like sets, deterministic rows and marker visibility. `Draft` isolates changes and rejects stale bodies or a missing reply parent. Rich-text parsing recognizes typed marker links; deletion strips the saved body. Monotonic marker allocation prevents dangling links from retargeting later markers, while comment revisions protect stale edits and restoration tokens.

Eleven focused core checks and fifteen integration checks pass. Version 11 attaches discussions to Project with surface ownership/limits and migration from older formats. Integration checks cover actual temporary project files and working-copy checkpoints, drawing-history preservation, protected page/alternate creation Undo, discard history, Trash/placeholder behavior, permanent removal without history resurrection, import choices, comment-only pages and remapped alternate links. These checks do not establish native UI acceptance.

`Editor::change_review` validates the final encoded document before applying review state without a drawing-history entry. A failed change leaves document/history intact. Import maps complete scoped discussions; imported feedback keeps attribution and is not locally editable. Keep Existing retains feedback and permits no-op repeat import after reopen. Type-aware source text rewriting preserves unrelated formatting, escaped literals and web links.

## UI checkpoint

The editor toolbar and View menu open Comments in the right panel; Inspector returns to properties without discarding the draft. Presentation uses its existing Comments area and visibility setting. The thread list supports local reviewer settings, attribution/date, reply/edit/delete/session Undo, read/like, rich text and page/alternate navigation. Unavailable links give feedback. Reviewer identity is stored locally and persisted before attributed actions; unreadable settings are preserved.

Composer aliases keep resource identifiers out of ordinary editing. Parsed source ranges preserve formatted link labels, escaped examples and external URLs. UTF-8 caret mapping supports callout insertion. Surface-wide duplicate marker labels receive stable suffixes. Numbered overlays use document coordinates, pointer movement and separately editable tip/color/position; cancellation changes no canonical review data. Review changes do not reframe the presentation canvas.

Drafts survive page changes and are separate from saved comments. Posting commits them; Cancel discards them. Escape leaves text focus without losing the draft. Each new composer receives a distinct text-history identity. Replacement/removal of a discussion clears restoration tokens and invalidates posting of existing drafts while keeping their text available to copy. Successful project load/recovery clears all project-local draft/cache state.

Fourteen app and geometry checks cover input batching, presentation shortcut isolation, navigation, draft/cache lifetime, local identity reopen/corruption, action semantics, duplicate labels, source-preserving text conversion, pointer placement/cancellation and settings interaction. The composer pins insertion/submission actions below scrolling fields, keeps its caret visible and limits Notes height so Comments remains usable. Its callout settings popup retains its own open state while a dropdown is active. Reviewer settings remain open during name editing.

Seven windowless graphics snapshots exercise the production OpenGL renderer at 1×/2× density, including 940×620 windows, 180-point side panes, long drafts and the callout settings popup. Scoped visual self-review found and corrected panel overlap, hidden actions and a scrolled-away caret. Fourteen review, twenty-two presentation, ten notes and sixteen shortcut checks pass. The opt-in graphics check is separate from these 62 interaction checks. Native comments layout/input, physical dragging/resizing, IME/screen-reader behavior, matched-reference comparison and integrated native save/reopen remain acceptance work. Windowless graphics do not establish those results.

## Native workflow checkpoint

Current Scenario Review verifies draft persistence across page navigation and exact Cancel, posting/replying/editing, callout fields/reference navigation, likes/unread, recoverable parent deletion/session restoration and retained replies. Drawing Undo preserves newer feedback. Presentation key events remain in the composer; inserted page links navigate correctly. Saved comments, attribution, markers and reactions survive process restart and Unicode-path reopen. External native-comments-current/report.md and verification.json retain the scope and unchanged candidate identity. No new implementation or broad tests. Native physical marker dragging/pane resizing, alternate/imported-feedback edge cases, matched-reference comparison, IME and screen-reader behavior remain unaccepted.

## Panel comparison follow-up

The comments-panel comparison follow-up corrects two visible differences: the editor now uses the shared shaded panel header, and composer actions place Cancel to the left of a blue primary Post/Save comment button at the right. Native posting, editing, editor cancellation and presentation cancellation preserve existing discussions and all drawing data; eight saved-state assertions pass. Thirteen review tests, seven inspected production-renderer states, format and all-target warnings-denied lint pass. The isolated macOS ARM64 debug review executable is `4d6e7ce82b48194b222767b2e5951b28d7e24fac2176dc1b0e90684c871ce04a`; its process is closed. External `comments-comparison-current/report.md` records the paired empty/draft observations and remaining limits. The comparison could not exercise reference posting/callouts beyond an identity-and-terms gate; no agreement was accepted. Idle composer entry and wording still differ; local reviewer identity remains an intentional offline adaptation. Prior native workflow and cold-reopen evidence is retained for unchanged behavior. The macOS/Linux distribution archives predate this UI-only correction and need a final batch refresh; they are not the current source build. No commit, installation or publication occurred. Windows remains held.


## Native marker gesture follow-up

Native review gesture acceptance found and corrected dropped quick presentation-panel resizing. A press can arrive separately from a coalesced movement/release; the app now retains the visible resize handle until release, honoring focus, modal, cancellation, threshold and ownership guards. All26 presentation tests pass, including regressions failing before correction, plus final all-target warnings-denied lint, format, debug build and signature. Native sidebar width300→400, notes height300→400 and navigator width200→260 persist and survive presentation re-entry. Editor callout drag saves only on comment commit; draft Save/Cancel and presentation drag Cancel preserve exact canonical data. Eight saved-state assertions pass. External `native-review-gestures-current/report.md` pins the corrected macOS ARM64 executable `bc5ea8d2314df732dd7a53f2841843da55713ba0f3175bd2d5dad6c3c6f5c301` and both failed intermediate candidates. Both checkpoint app processes are closed; other owners and pending confirmations remain untouched. This establishes sampled native gesture outcomes, not physical held cadence, reference callout parity, every pane, cold-process preference retention or other-platform input. Windows remains held; macOS/Linux distribution refresh remains required. No commit, installation or publication.

