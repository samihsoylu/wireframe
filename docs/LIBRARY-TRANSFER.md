# Component library transfer

This is the implementation contract for the resource clipboard. Existing single-source and same-library multiple-source control clipboard behavior remains independent. The resource payload, persisted correspondence, pure import planner and atomic application are implemented. Single-library row Copy and resource Paste review are connected. Cross-library selection and conflict previews are connected, with scoped macOS native acceptance. Remaining comparison, lifecycle and platform cases are still required.

## Current correction — resource notes on replacement

W94's deeper replacement-path study establishes that Use New retains destination library-level notes and name. The implementation now preserves those fields while replacing source content, source notes, layout and artwork. A propagated parent conflict that differs only in retained library notes counts as reuse and creates no empty history entry. The 31 library-import tests pass, including exact Undo/Redo for actual source changes. Earlier statements that replacement overwrites library-level notes are superseded; source-level notes remain conflict-bearing content.

## User-visible behavior

- Copying a library includes its complete editable content, every owned source, source/library notes, positions, locks, crop/markup, paint order and required dependencies/assets. Unlike source-group Copy, a whole-library transfer preserves locks and positions.
- Selecting sources from more than one library copies each complete owning library once. A selected library also requests resource transfer; same-library source-only selections keep the current control clipboard route. Explicit context targets outside a selection retain their own scope.
- Same-project Copy/Paste creates fresh library/source identities and resolves library-name collisions. Internal references among the copied resources target their corresponding copies; unrelated existing dependents keep their original targets. Dependency-only resources already present may be reused.
- Resource Paste adds libraries regardless of whether the active workspace is a page, library or focused source. It does not insert ordinary groups into that canvas or apply the control-paste offset. Preserve the current active target; do not navigate into the last imported source.
- Foreign Paste imports missing resources and records correspondence. Repeating an unchanged import reuses it and causes no empty history entry. Changed resources require Use New, Use Existing, Keep Both or Cancel; allow applying a choice to remaining conflicts. State added, replaced and reused counts truthfully.
- Use Existing preserves local contents; Keep Both allocates independent identities and names; Use New updates the matched resource without breaking unrelated dependents. Cancel performs no mutation. One Undo/Redo restores the exact complete graph, including local overrides, notes, identities, import provenance and allocator state.

## Model and transaction design

Introduce an explicit resource payload in the clipboard instead of interpreting dependency-only library metadata as a request to paste libraries. Preserve old ordinary/single-source clipboard decoding. The payload identifies explicitly requested libraries, source project origin and the complete required graph. Validate distinct requested IDs, owner integrity, reference mappings, asset lengths and existing depth/count/coordinate limits before any changes. Reject mixed incompatible payload kinds atomically. Empty libraries are valid clipboard contents.

Separate a pure import planner from transaction application. The plan resolves origins/dependencies, records proposed identity mappings and names, and exposes conflicts without altering the editor, clipboard, document or history. Collect every conflict choice before applying one guarded transaction. If the destination has changed while a dialog is open, revalidate/replan rather than applying stale mappings.

Persist import correspondence with explicit originating project/resource identities and definition mappings; canonical filenames alone do not establish that unrelated numeric IDs denote the same resource. Define migration/validation/recovery behavior for this metadata before changing the on-disk format. Retain provenance across save/reopen and audit SaveAs semantics. Distinct explicit resources must not silently collapse through shared ancestry or same names. Name matches may identify a conflict candidate but do not authorize replacement.

Use whole-document history snapshots for import application. Capture each destination resource once even if several incoming resources refer to it. Allocate above reserved missing identities, remap all internal references and paint paths consistently, preserve external overrides, and coalesce requested roots with dependency-only imports only where their identities truly match. Do not reuse the source-copy unlock or viewport translation logic for resources.

Represent library/source navigator selection explicitly, including cross-library selections. Resolve action scope once for keyboard/native/fallback/context routes. Text-entry copy/paste retains priority. Publish the clipboard successfully before any Cut mutation. Cut/removal and missing-resource identity restoration require separate lifecycle integration and evidence; do not route them through immediate page deletion.

## Acceptance still required

Core tests must cover empty/full/mixed libraries; independent source identities and complete nested graph preservation; same-project new copies versus missing identities; foreign reuse after serialization/restart; all conflict choices/cancellation; ambiguous shared ancestry; references and assets; invalid payload rollback; allocator boundaries; and exact Undo/Redo. UI checks must cover library and cross-library selection scope, current-target retention, text focus, truthful status and conflict navigation. Combine native acceptance with the pending source-context menu correction. Existing clipboard and persistence suites remain regression requirements.

The current research verifies same-project copies, page-target resource paste, two-library source selection, foreign initial/repeated import, and conflict choices on a seven-source fixture. It does not establish native asset/dependency permutations, missing-library Cut, SaveAs/restart provenance or other-platform behavior. Broader product and platform acceptance remain open.

## Implemented foundation

`Editor::copy_libraries` captures requested libraries separately from dependency-only libraries, including the transitive whole-library graph and referenced assets. Complete editable content, source notes, artwork and positions remain unchanged; no control-paste offset or unlock is applied. Destination-specific correspondence is cleared from copied metadata. External page links remain explicit in the payload until planning preserves same-project links or clears foreign page targets. Missing definitions remain missing references. Empty libraries are nonempty resource clipboard contents.

`Clipboard::library_resources` validates the envelope and graph, rejects mixed control/resource payloads, unrelated libraries, missing assets and invalid ownership, and applies document hierarchy/count/coordinate/size constraints. Ordinary control/source clipboard decoding remains compatible and clears destination-specific correspondence from imported dependency metadata. The control paste entrypoint rejects resource payloads without mutating the document, selection or history until the resource workflow exists.

Document version8 persists foreign library/definition correspondence with bounded validation and historical-ID reservation. Version7 migration and clean/dirty recovery are covered. Follow with pure conflict planning, exact atomic application, SaveAs/origin policy and navigator/dialog integration; this foundation is not user-visible resource Paste acceptance.

## Import planner and application

`Editor::plan_library_import` validates the clipboard and destination without mutation. Each requested/dependency library has an Add, Reuse or Conflict entry. Exact foreign origin matches are reserved before weaker name matches, and each destination can be claimed once. Existing same-project dependency libraries are reused; explicit same-project copies receive fresh identities, while missing resource identities can be restored. Foreign import correspondence survives serialization and allows unchanged repeated imports to leave document, allocator and history untouched.

A plan collects Use New, Use Existing or Keep Both for every conflict. Dropping it cancels without mutation. Preparation builds and validates a complete candidate graph; application rejects a changed document or destination origin, then uses one full-document history transaction. The active page/library and valid selection remain unchanged. Assets are reused by media type and bytes; asset/definition references are remapped together. Foreign page links are cleared rather than accidentally linking to equal numeric page IDs in another project. This includes parsed inline text targets in ordinary controls, table cells and nested overrides. Text targets are rewritten before resource/path correspondence; label formatting is retained. Preview clearing only changes its isolated clone. Existing URL/Back links follow the normal clipboard rules.

Use New retains matched library/definition identities and replaces their contents. Keep Both creates independent resources with collision names and internal references targeting the corresponding copies. Use Existing preserves local contents. Historical missing-source bindings can preserve a placeholder identity across repeated imports and reconnect it when its source returns. Distinct restored sources cannot claim the same historical target. Correspondence may therefore include missing dependency definition IDs in addition to owned definitions.

A changed dependency makes an existing parent require a decision because Keep Both may change the referenced identity. Keeping that dependency and importing an otherwise unchanged parent produces no empty history entry. Definition order is preserved for replacements; added/replaced/reused counts reflect resulting content changes rather than requested actions.

Ten core import tests cover full/empty libraries, requested versus dependency scope, repeated foreign import after serialization, all choices, exact Undo/Redo, external overrides under known source identities, cancellation/unresolved plans, stale document/origin rejection, name/identity collisions, missing source restoration and no-op history. The selected43 integration tests and all-target Clippy are recorded in the external checkpoint.

## Required before UI acceptance

The later sections implement descendant correspondence, orphan-path decisions, same-name source mapping, the library/cross-library navigator and conflict dialog. Lifecycle and clipboard-confirmed Cut are connected through [library lifecycle](LIBRARY-LIFECYCLE.md) and [clipboard publication](CLIPBOARD-PUBLICATION.md). Retain the scoped native limits below, including broader ancestry/return/SaveAs and asset permutations. Core API checks alone do not establish native resource-transfer acceptance.

## Verified comparison contract

Whole-library repeated imports use content equivalence rather than complete stored-resource equality. The planner implements these verified cases:

- Library-level notes do not create a conflict. Preserve the destination note and existing history when reusing.
- Moving an entire source within a library does not create a conflict. Preserve its local placement. Child layout changes do create a conflict, even when automatic source-bound normalization also changes the source origin.
- Source-root editing locks do not create a conflict and remain local on reuse. Changed source crop does create a conflict.
- Individual source notes do create a conflict.
- An otherwise identical source can have a new root identity after Cut/Paste and still be reused. Do not use raw source/root IDs as an artwork-difference test. Any dependent mappings must target the actual current equivalent source, not an obsolete identity retained in import correspondence.

Keep conflict equivalence separate from exact transaction equality. An explicitly chosen replacement can alter fields that do not themselves trigger a conflict; actual history and added/replaced/reused reporting must still reflect the complete resulting graph. Reuse must never overwrite local ignored fields or silently restore stale identities.

The verified native cases are library notes, root placement, source notes, relative child layout, root lock, root crop and an ID-only Cut/reinsert followed by project reopen. The later implementation supplies nested identity/path mapping, ordinary-control/order comparison, unrelated same-name decisions and the conflict dialog. Native cases beyond the explicitly accepted scope, including broader ancestry/SaveAs behavior, remain open. Exact Undo and protection against numeric coincidence remain requirements.

## Current comparison implementation

Conflict comparison normalizes owned source-root placement IDs, position and editing lock, ignores library notes and compares effective library paint paths. Source notes, crop, canonical contents and references remain significant. Exact replacement accounting is separate and retains local fields ignored by conflict detection. Reuse returns the complete destination graph unchanged.

Same-project source restoration retargets import bindings atomically with instance references. Import planning prefers a recorded current source, then a unique current source name, before restoring an absent historical target. This also handles stale saved bindings without making new dependents point to a retired source. Current-source matching does not equate unrelated descendant identities or rewrite override paths.

Seventeen import tests now cover these cases, including stale bindings, source Cut/Paste/reopen, new dependent imports, ordinary-control/order preservation and exact history/counts when dependency conflict propagation exposes a parent with only local metadata differences. All50 selected graph/clipboard cases pass. The later correspondence and UI sections supersede the earlier descendant-mapping and unconnected-dialog gaps. Native ordinary-control/order equivalence and broader ancestry/SaveAs cases remain open where not covered by those scoped checks.

## Descendant correspondence requirement

Verified identity-only changes include35 descendant IDs across seven sources. An unchanged library must be reused even when those IDs differ, preserving destination IDs and history. Descendant identity must therefore be separated from source content, with meaningful type, hierarchy, ordering, geometry and property differences retained. The current core implements descendant correspondence as described below, with safe orphan-path decision handling described below.

A coherent path map must accompany resource matching. Incoming instances that refer to a reused dependency must target its current child paths. When replacing a source with changed content and different IDs, existing local overrides must not silently become orphaned merely because the incoming representation uses different identities. Keep exact document Undo/Redo; preserve stable current child identities or translate dependent paths where correspondence is established. Explicitly handle ambiguity, deleted children and nested instances; do not match unrelated children solely because their numeric IDs coincide.

Required regressions include identity-only whole-library reuse without history, nested group/path renumbering, new parents with overrides referencing reused sources, changed-source replacement with existing page/alternate/source/library overrides, resolved visible output, scoped-ID overlap, distinct/ambiguous children, structural edits, serialization and exact Undo/Redo. This requirement is not completed by relaxing equality alone. Stable origin/SaveAs and navigator/dialog integration remain additional work.

## Source-child and override path mapping

The import builder establishes scoped child correspondence before copying canonical contents. It first recognizes identical ordered content independently of IDs, then retains established ancestry IDs, matches unique equivalent children and pairs sole remaining compatible kinds. Ambiguous or new children get fresh IDs. Scoped paths cross literal groups and nested source references; incoming overrides are rewritten before resource IDs are remapped. Source comparison and graph construction therefore share the same current identities. Foreign page-link clearing and asset reuse participate in canonical content matching.

Existing local override paths remain valid when a corresponding child retains its destination identity. New or ambiguous children cannot receive a deleted child's overrides by numeric coincidence; allocation also reserves existing orphan-path values. `PreparedLibraryImport::newly_orphaned_overrides()` reports newly unresolved occurrences across editing surfaces. The UI must disclose this impact before applying a source-replacement choice. Keep Both preserves existing consumers and provides an alternative for structural losses.

Nine additional import regressions cover identity-only reuse, nested dependency paths, page/alternate/source/library local edits, repeated identical siblings, unrelated same-name resources, ambiguous controls, reordering/type changes, exact Undo/Redo, serialization, foreign page links, orphan-ID allocation and mutation-free rejection of unresolved preexisting orphan paths. All59 selected integration cases pass. These are model checks; native workflow acceptance is still required.

## Orphan paths and safe decisions

Already-unresolved incoming edit paths translate their known prefix and preserve the unknown suffix when it cannot attach to another control. This permits safe identity changes and repeated imports without new history. If a mapped child or an extra child in a retained destination source could capture the edit, planning marks the affected dependency for conflict review. `LibraryImportEntry::has_unmapped_overrides` identifies the provisional replacement risk. Actual choice combinations are rechecked; retaining a destination source can expose additional conflicts absent from replacement.

Keep Both copies the dependency separately and retains the original orphan metadata. Keeping the referring resource can avoid importing the problematic edit. Unsafe combinations return an actionable preparation error without changing the document/history. Planning-only guard IDs are unique by missing path prefix, preserve distinct edits and can never reach an applicable prepared import or saved document. Choice safety depends on the whole resource graph, not just an individual row flag.

Additional regressions cover safe translation/repeat reuse, colliding child/group IDs, size edits, destination-only children, new parents, mixed choices, multiple distinct orphan paths, resolved output, exact Undo/Redo and serialization. Conflict UI is connected. Retained macOS native checks verify the three-edit impact warning, revised choices and cancellation, unsafe incoming-edit reuse rejection, Keep Both recovery, exact atomic Undo/Redo and cold reopen (external `orphan-import-current/report.md`). This does not establish every ancestry or nested-override permutation. Unique-kind matching remains an implementation decision, not a claim of exhaustive observed matching rules.


## Named-file origin lifetime

A copied whole library duplicates when pasted into its originating named file, including after close/reopen and process restart. Native Save As to another filename, or an identical file copy at a different path, treats the original clipboard as foreign and reuses unchanged resources. Preserve same-path origin continuity and change the origin for Save As. The current canonical-path hash provides this behavior without exposing filenames in clipboard metadata. This is not a persisted document UUID requirement. Unnamed editor sessions remain separate; alias/move and first-save details require their own acceptance rather than inference from named-file cases.


## Import review integration

Library row Copy and context Copy publish a resource payload; source-only Copy remains a control payload. Paste routes resource payloads before the ordinary source/control path. Immediate add/reuse preserves the active editing target. Conflicting resources collect Use New, Use Existing or Keep Both decisions, with an apply-to-remaining option. Choices are provisional until the complete graph can be prepared. Cancel or Escape drops the review without mutation.

Newly unresolved edit occurrences require an explicit impact review before application. The count includes repeated uses and can include newly imported orphan metadata; it is not a count solely of previously visible edits. Review choices returns to conflict decisions. Invalid combinations expose the preparation error without applying partial changes. A changed project or named-file origin resets the review; no stale choice can be applied. Standard shortcut/canvas/modal boundaries protect the review, and repeated clipboard events cannot replace it.

Current row focus identifies libraries as well as sources, including keyboard focus on a row whose canvas is not active and consecutive Duplicate/Copy before redraw. Whole-library Cut is connected through guarded clipboard publication and captured-scope validation; failed publication or a changed target leaves content intact. Multiple/cross-library row selection and conflict previews are implemented as described below. Their later scoped native evidence supersedes this section’s original implementation gaps; broader lifecycle and exact visual acceptance remain bounded by the latest sections.


## Mixed navigator selection

The navigator tracks library headings and source rows in one ordered selection. Command/Ctrl toggles a row; Shift spans displayed rows from the anchor, including headings and excluding collapsed children. Preserve the current active resource while it remains selected, otherwise use the first selected row in document order. Selection changes do not enter document history.

Any selected library heading, or sources with more than one owner, uses whole-library Copy with unique owners in document order. Same-library source-only Copy/Cut remains independent. Context actions on an unselected row act on that explicit row; context actions on a selected row use the applicable selection. Global Duplicate is limited to one source; row-context Duplicate retains its explicit source scope. Rename is disabled during multiple selection. Whole-library/mixed Cut now uses confirmed clipboard publication before removal; lifecycle context actions follow the scoped selection rules in [library lifecycle](LIBRARY-LIFECYCLE.md). Global mixed Delete remains a separate route and must not reuse the whole-owner clipboard resolver.

After successful resource Paste, retain the active canvas and collapse selection to its active resource, matching the observed cross-library paste outcome. Native acceptance must still verify full range/toggle/collapse behavior. Clipboard serialization and reading share the64 MiB serialized-JSON project bound. A valid image-heavy payload above the former4 MB limit passes application canvas/page/library transfers and exact history; native macOS canvas Cut/Paste also passes. Native large library/page transfers and other-platform capacity remain unverified; see [clipboard publication](CLIPBOARD-PUBLICATION.md).

## Conflict preview and native acceptance

The review displays Existing on the left and New on the right, independently fitted using the common renderer and destination appearance settings. Incoming rendering uses a validated isolated graph with external page links cleared only in the preview clone; the original clipboard remains intact. Cache only the current conflict and invalidate it when destination review becomes stale. An empty library has an explicit empty state, while rendering failure reports preview unavailability.

Navigator modified selection uses the primary click event modifier snapshot, so releasing Command/Ctrl or Shift before the frame does not turn a range/toggle into single selection. Rendered regressions cover both source and heading targets. Native macOS acceptance covers cross-library complete owner Copy, same-project Paste/Undo/Redo, conflict cancellation, all-remaining choices, retained canvas, repeated reuse, process restart/resave and scoped context Duplicate. Narrow native layout, individual mixed conflict choices, impact-warning presentation, unrelated/ambiguous ancestry and whole-library removal remain explicit follow-up work.
