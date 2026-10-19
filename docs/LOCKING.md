# Locking contract

Status: correction implemented; affected automated checks and scoped native macOS contact workflow pass.

Locks prevent accidental movement and deletion while leaving content editable. A directly locked object requires explicit selection; normal selection/marquee behavior continues to skip it. Once explicitly selected, its text editor, formatting and supported content properties remain available. Selected locked groups and component instances can be entered. Their descendants remain protected by the ancestor lock for movement/deletion and direct numeric or pointer resizing.

Auto-size is an explicit content-sizing command and remains available for supported controls, including directly locked controls and children of locked containers. It preserves protected origin coordinates. Do not conflate automatic measurement changes after content edits with direct geometry manipulation.

Mixed selections apply content edits to all applicable selected controls. Movement and displayed editable geometry use only movable members; locked members and their protected descendants retain their positions. Geometry-only changes must not slip through a content-property update. Crop creation, adjustment and removal are disabled for a directly locked target. Retain existing crop, resize anchoring and reset behavior. An unlocked child group can change or remove its crop while its ancestor remains locked; its direct position controls remain protected. Crop actions must complete without an internal error.

## Component ownership and persistence

A source node's lock protects source editing and does not propagate into instances. An instance's own lock protects that instance and its descendants. Local child locks belong to the nearest canonical owning instance and the full stable descendant path, never to the shared source or a sibling instance. Nested component expansion must not accidentally inherit editing locks from a definition, including local locks stored inside that definition. Outer instance patches apply after inherited content is resolved.

Local locks are durable document state. Applying or removing one is a document history step and survives Save/reopen and recovery. Unlock must work inside an instance without requiring structural source editing. Explicit local lock state must remain isolated through source property changes, page/alternate switching and nested paths. Reset semantics must be deliberate: resetting a content property does not unlock; a full local-override reset clears the local lock as part of the same reversible operation. Orphaned paths remain retained and cannot affect unrelated nodes.

New persisted lock overrides require a format version that older readers reject rather than silently discard. Earlier formats remain readable; migration preserves IDs, content, fonts, assets, source lock flags and existing overrides. It does not invent local locks from prior source-lock inheritance. Save and recovery use the same decoder and version contract.

## Implementation and acceptance

Use a shared operation policy so workspace commands, inspector controls, double-click/text entry, shortcuts, local overrides and pointer handling agree. Preserve valid geometry guards when replacing blanket property guards. UI copy must describe position protection without falsely claiming content is read-only.

Verify locked text/format edits and Auto-size, selected locked container entry, ancestor movement/deletion protection, mixed formatting/movement, source-lock isolation, local lock/unlock and history, nested override ownership, full/property reset, orphan paths and migration/save/recovery. Keep negative checks for direct move/resize/crop/deletion attempts. Check that content-derived size updates retain protected positions.

After affected regressions and lint checks, use an optimized native contact-page candidate to verify selection, text editing, formatting, Auto-size, crop protection, local locks, Undo/Redo and Save/process restart. Inspect saved data as well as visible state. The macOS contact workflow passes these checks with saved-data inspection and process restart. Explicit selection through Layers and Command-click is accepted. A follow-up timing regression fixed loss of the Command/Shift state when modifiers release before the next paint; native selection and normal-click exclusion pass. Windows/Linux native behavior and wider interaction acceptance remain open.
