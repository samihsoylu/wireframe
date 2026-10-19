# Shared image asset catalog

Status: live image entries, previews and shared insertion are implemented in the model and application routes; bounded native macOS acceptance is complete. Recoverable asset storage, live lookup, rendering, history and migration are implemented. Shared Trash, image context removal, mixed restoration and confirmed permanent removal are connected. Native macOS now verifies single-resource Trash/Restore/history/cold reopen with shared automatic/fixed controls; mixed page/image restoration and cancellation also have scoped native acceptance. Final permanent-removal execution remains unverified.

## Model and insertion

Use a stable typed Asset identity keyed by the document asset ID. The Assets category, All, category unions and Quick Add share this identity. Duplicate names remain separate choices. Only supported decodable image assets produce insertable image entries; names and filename extensions do not establish decode format or dimensions.

A catalog entry creates a fresh ordinary Image control referencing the existing asset. Natural geometry comes from the current decoded image, even if an old filename describes another size. Do not duplicate image bytes or inherit crop, rotation, border, dimensions or other overrides from a canvas control using the same asset. Double-click and Quick Add share default placement; drag uses the existing held-preview insertion lifecycle. A successful insertion is one Undo step and survives save/reopen with the same asset ID.

Recheck destination eligibility, current asset availability and decoded content at commit. Missing, changed or unavailable assets cancel stale previews/gestures without document mutation. Preserve existing image assignment/import/replacement behavior and saved document compatibility.

## Preview and lifecycle

Use bounded preview caching keyed by identity, current bytes and relevant display settings. Reuse unchanged entries without cloning the whole project per frame; refresh names, decoded geometry and previews when their dependencies change. Category unions deduplicate IDs, not names. Empty categories retain the established row height and keyboard/accessibility paths.

The intended asset menu includes recoverable removal. The backend contract below now implements retained image references, restoration, Undo and migration. The shared Trash list and image context menu are connected. Permanent removal and Empty Trash use the existing exact-selection confirmation and preserve broken image references. Native acceptance remains open.

## Acceptance

Cover duplicate names, replaced bytes with stale names, natural dimensions, shared bytes, unchanged previews, dependency invalidation, unsupported/missing images, stale drag cancellation, structural edit guards, Quick Add, one-step Undo and save/reopen. Native review must include actual insertion and narrow/wide presentation. Held-pointer cadence and current Windows/Linux native acceptance require their own evidence. Individual icon search and the external catalog remain separate work.

## Implementation checkpoint

Stable Asset identities now appear in Assets, All and Quick Add, retaining duplicate names as separate choices. The existing library activation, placement and held-drag routes use the shared insertion method. Header-readable image assets enter the catalog; insertion and preview fully decode their bytes and reject unsupported/corrupt data before creating a control. Missing assets, invalid coordinates, locked destinations and exhausted IDs fail without document changes. An unrelated pending edit survives failed insertion.

Asset previews use a fresh image recipe and only the selected asset, settings and shared project font. Cache hits compare borrowed inputs; unchanged artwork survives renames and unrelated edits. Content/settings changes refresh the scene; removed or corrupt assets cannot supply a preview. The cache retains at most 32 entries and budgets 64 MiB for encoded inputs, decoded pixels and PNG copies. That budget excludes scene metadata and GPU/process overhead; total memory and large-library cadence still require profiling.

Model and event tests cover natural sizing, shared bytes, crop/rotation isolation, nested coordinates, locked contexts, identity exhaustion, pending-edit preservation, duplicate names, category union/search, preview reuse/invalidation/eviction, Quick Add, held preview, stale drag cancellation, one-step Undo and save/reopen. Native acceptance remains separate from the lifecycle implementation checkpoints below. The later lifecycle checkpoints below add retained state and removal commands.


Native macOS verifies same-name choices, double-click, Quick Add, automated drag release, one-step Undo and saved insertion reopening in a fresh process. All 308 selected tests and all-target warnings-denied Clippy pass; optimized ARM64 candidate/signature checks pass. Narrow native layout for the added category, physical held cadence, native asset retention/removal, large-library memory/cadence and current Windows/Linux native acceptance remain open. No installation or commit in this checkpoint.

## Retained image backend

Version 15 adds an optional asset `trashed` flag. False is omitted, preserving older canonical asset serialization for recovery fingerprint migration. Versions 1–14 migrate to live assets; older version tags containing a true retained-image flag are rejected. Named files and recovery records are read without rewriting their bytes. A clean old baseline stays clean; an unsaved edit stays dirty.

Moving images to Trash preserves resource IDs, encoded image bytes, all control references and stored geometry/crop/rotation. Live lookup, insertion choices and image/icon previews exclude retained assets. Existing image controls render an unavailable state; automatic dimensions use the existing 77×79 fallback and explicit frames remain unchanged. Custom icons and cover-flow artwork also stop decoding the retained bytes. Restoration makes the same asset available again, including its natural dimensions and cached-component preview invalidation. No source property is cleared or replaced.

`trash_assets` and `restore_assets` validate the complete distinct selection and serialization before committing unrelated pending edits, then record one reversible action. Invalid/stale selections and unavailable insertion sources fail without mutation. New file imports do not implicitly reuse a retained same-name image. Clipboard, library and project transfers retain availability with the asset: byte equality alone cannot merge a live dependency into a retained one or vice versa. Explicit project import conflicts distinguish availability; Use New adopts incoming availability, Use Existing preserves destination availability, and Keep Both retains separate identities. The import review labels existing/incoming availability and explains the effect of Use New; read-only comparison can show retained artwork without changing its availability.

Verification covers both styles, shared image/icon/cover-flow use, automatic/fixed geometry, preview invalidation, exact Undo/Redo, pending-edit rejection, named save/load, working-copy reopen, version-tag rejection, clean/dirty recovery migration and clipboard/library/project transfer. This is model, rendering and event-harness evidence. Shared Trash selection/restoration/permanent removal now has the integration checkpoint below; native reference confirmation and final native product acceptance remain open. The existing physical drag acceptance is retained; this change does not claim a new physical test.

## Shared Trash and permanent removal

The image context menu offers Move Image to Trash. Shared Trash lists pages, images and libraries with typed identities; same-name entries cannot collide. Retained image cards show cached, read-only artwork without exposing the image to normal insertion, rendering or export. Mixed selection restores in one history action and retains the current surviving workspace. Image-only restoration uses the same operation-history path, preserving later view changes.

Delete Permanently and Empty Trash use the existing exact-scope confirmation. Cancel and document drift preserve state. Permanent removal deletes the image record and import provenance from the current document, pending edit and all Undo/Redo snapshots. It retains only a bounded, content-free ID in `removed_assets`, so unchanged image, icon, row-icon and component-override references remain unavailable. Undo cannot restore the bytes; unrelated history remains usable. Live resource IDs, removed page IDs and removed image IDs cannot collide, and the allocator stays above all reserved identities.

Clipboard and library envelopes carry only the required removed-image identities. Foreign transfers allocate fresh missing identities rather than binding to a destination's live image. Project and combined page/library imports carry the same mapping; unused provisional markers are removed after Use Existing decisions. Component and library previews retain missing markers without reactivating artwork. Named files and working copies preserve them without image data.

The event harness exercises the actual context menu, image Trash card, restore, cancellation, selected permanent deletion, mixed Empty Trash and availability comparison. Backend checks cover historical byte removal, mixed history, malformed markers, missing-reference clipboard/project/library transfer, component/alternate overrides and repeated imports. This completes the implemented shared-image lifecycle, not native acceptance. Reference drawing, native confirmation/focus/input and final platform workflow evidence are still required.

## Native recoverable lifecycle checkpoint

The current candidate imports one image through the native chooser, reuses it through the source picker, and preserves shared identity across automatic/fixed controls. Assets context Trash removes the library entry and shows unavailable canvas states without changing encoded bytes, references or stored geometry. Restore returns the exact original document; one-step Undo/Redo return exact saved states. Unicode-path cold reopen and restored library double-click insertion also pass. External native-resources-current/report.md, verification.json and bundle.json identify evidence. No runtime changes or new broad tests. Subsequent mixed page/image restoration returns the exact saved baseline with one-step Undo/Redo. Selected-delete Escape and mixed Empty Trash Cancel preserve saved data and Redo (external native-mixed-resources-current). Component/icon/crop combinations and final permanent-removal execution remain unverified; no permanent deletion was attempted.
