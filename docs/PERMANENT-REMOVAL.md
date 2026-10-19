# Permanent-removal contract

## Observed behavior

Permanent removal is separate from recoverable Trash and reversible Cut. Delete Permanently confirms its selected retained resource scope; Empty Trash confirms the complete retained set. Both warn that the operation cannot be undone and offer Cancel. Removing a page removes its official and alternate contents and thumbnails. A named alternate/version shared by another page survives; it disappears when its last page membership is removed.

Surviving controls retain incoming page-link destinations after the target is permanently removed. The inspector displays Broken. Do not silently clear the stored link. This is established for ordinary Button links in official and alternate page contents; nested controls, row links, components and clipboard reconnection need additional coverage.

Permanent removal does not add a reversible history entry or clear all earlier history. An earlier rename of a surviving page remains undoable/redone without restoring the removed target. Earlier Trash and rename operations targeting a permanently removed page fail, explain that the resource is missing and advance past those failed entries. They must not restore removed contents or remove the generated last-page placeholder. The failure behavior is observed for those two operation types; mixed-target operations remain to be specified.

Trashing the last live page creates a fresh usable blank. Empty Trash removes only retained pages, preserving that blank. The selected removal and all-retained removal scopes were checked separately. Whole-library permanent removal remains a separate evidence gap.

## Implementation requirements

The Trash dialog now exposes selected Delete Permanently and complete Empty Trash with a second confirmation. The required boundaries are:

- Represent known missing page destinations without weakening validation of arbitrary malformed files. Preserve identity, serialization, readable Broken status and non-navigable behavior. Reserve identities against accidental reuse; deliberate clipboard reconnection requires an explicit contract.
- Apply selected retained-resource removal atomically; revalidate captured scope before confirmation completes. Delete page alternates and unreferenced version memberships; retain unrelated content and shared dependencies.
- Prevent every old snapshot and pending edit from resurrecting permanently removed resources. Keep unrelated history usable. Consume failures of operations whose targets no longer exist with a clear message. Mixed history entries need targeted handling rather than blanket history clearing.
- Keep Cancel mutation-free, maintain a live page and valid active editor context, invalidate affected previews and ensure saved/reopened files preserve broken references and removal.
- Cover pending notes/text edits, stale confirmation selections, source dependencies, errors/limits and all save/recovery paths before native acceptance.

## Model implementation

Known removed page destinations were introduced in version10; the current version15 stores these IDs separately from page content and also reserves removed image identities. Confirmed retained-resource removal prepares the new document, pending edit and all historical snapshots before applying it. Invalid/stale selections and validation/size-limit failures leave the editor unchanged. Official/alternate content is removed, parent references are repaired and unused named versions are pruned. Shared versions, component sources, assets and surviving content remain. Removed IDs stay reserved, and named/recovery files preserve broken links.

Undo and Redo snapshots are stripped of removed pages. Operations with no surviving effect are consumed with an explanatory error and do not enter the opposite history direction. Entries that require a live page that no longer survives are also consumed: they cannot replace the current page with an incomplete historical graph. Mixed Trash entries retain surviving targets. Mixed ordinary snapshots retain surviving edits. Unrelated rename, control and alternate edits remain undoable. A pending surviving edit commits against its sanitized baseline after preparation succeeds. The existing history byte limit still applies.

The inspector uses Broken for stored missing destinations. Presentation rejects navigation without changing route history. Control Paste retains known broken destinations in the receiving project; same-project library Paste does likewise. Foreign library transfers retain their existing link policy. Deliberate page reconnection and foreign control-link identity remain separate contract questions.

Automated model/application/storage coverage is implementation self-review, not new native evidence. The confirmed UI and shared page/library model are implemented, with native acceptance still pending. Broader mixed-operation fidelity, nested/row/override behavior, historical source moves, asset retention/cleanup and deliberate clipboard reconnection remain open. Existing reversible immediate removal is still a separate operation; its test name now reflects that.


## Shared library and confirmation behavior

Permanent library removal removes its source definitions and source ownership. Surviving official/alternate instances, nested dependencies, overrides, assets and unrelated libraries remain stored. Missing sources use the existing unresolved-component behavior. Historical projections remove the library and its definitions, including earlier ownership records for the removed identities. The default-library pointer is cleared when its target is removed. Missing component references already have a representation; the current document format is version15.

Delete Permanently captures the selected retained pages/libraries. Empty Trash captures the complete retained set, including libraries. Both display an irreversible warning and Cancel, while the underlying Trash controls are disabled. Escape cancels the confirmation. Any document change invalidates the captured confirmation, requiring Cancel and review; new retained items are never silently added to an already shown scope. Successful partial removal selects a remaining item; removing the final item closes Trash. Opening Trash first finishes pending notes and pointer edits.

Automated coverage checks Cancel/Escape, mixed removal, stale confirmations, remaining selection, missing components, exact history and serialization, plus button layout at supported desktop widths. This does not yet prove native deletion behavior. Assets now support retained state, typed shared Trash restoration and permanent removal with content-free missing identities. See [shared image lifecycle](ASSET-CATALOG.md). Native final image/library deletion remains unverified; model/event tests do not close that boundary.
