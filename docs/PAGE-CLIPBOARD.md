# Whole-page clipboard

Status: implemented in the editor and navigator. Native macOS same-project single-page Copy/Paste and Cut/Paste now pass, including notes/link targets, identity and exact saved Undo/Redo. Native discontiguous multi-page cross-project transfer, shared assets, remapped/missing links, cancellation and all page conflict choices also pass. The named Mac collapsed three-page context Cut/Paste sample below also passes. Other nested/collapsed/dependency/mixed-conflict/input/platform variants remain open.

## Scope and data

Page focus routes Cut/Copy/Paste to the explicit canonical page selection. A collapsed parent contributes its selected descendants; an expanded parent alone contributes only itself. Context-menu actions can target a noncurrent page without changing the current canvas. Text fields retain their own editing commands. A control clipboard pasted while page navigation owns focus does not mutate canvas controls.

Copy includes official content, every alternate, notes, named-version memberships, discussions, nested groups, component edits, crops, locks and paint order. Only required component sources and images are included. Unselected pages, unrelated sources and import provenance are omitted. External page destinations are content-free reserved identities. Images in Trash remain missing in the transferred page without publishing their retained bytes.

The version-2 application envelope has a distinct optional page payload. Mixed page/control/library payloads, invalid graphs, unrelated dependencies, invalid hierarchy and invalid origins are rejected. The clipboard envelope and project share a64 MiB serialized-JSON limit; the version prefix is excluded from the envelope bound. Bounded publication stops encoding when that limit is exceeded, and readers reject oversized input before parsing. Publication failure must preserve the original document and history.

## Same-project paste

Session identity preserves this relationship across first Save and Save As; a reopened document can match its qualified file origin. Existing dependencies retain their current contents, even when changed after Copy. Missing dependencies can be restored without overwriting another resource type. A copied page or alternate gets a new identity when its old identity is occupied, including in Trash. An absent cut identity can be reused. Existing links keep their original destinations, including self-links and links between copied pages.

Names avoid live-page collisions case-insensitively, beginning with `(1)`. A name already ending in a numeric parenthetical suffix fills the first available suffix rather than nesting another suffix.

Paste preserves the current page and active alternate. It adds one history action; exact Undo/Redo and encoded reopen preserve the result. A prepared paste rejects a changed document. Context-menu native paste delivery additionally checks the captured project/session and target selection.

## Hierarchy

Single-page paste onto a parent inserts its first child; onto a leaf it inserts a sibling after that leaf. Multiple pages reconstruct the source selection's depth sequence: an increase adds one level, an equal depth stays equal, and a decrease resets to the minimum selected depth. Destination offset and the five-level placement cap apply before placement. A contiguous chain of selected collapsed ancestors anchors a multi-page paste beside the outermost closed ancestor. Current page selection remains unchanged.

## Cut

Cut uses the existing native publication acknowledgement before removing anything. A failed write or changed document, session, editor target or navigator selection preserves the pages. Removal is reversible and separate from Trash and permanent deletion. Surviving children adopt the retained-page removal hierarchy; retained children lose a physically removed parent. The last-live-page case creates an empty page. Page and alternate identities remain reserved for broken links until restored. Comments and complete page data remain in reversible history.

## Cross-project paste

The established combined page/image/library import review handles collisions and dependency correspondence. No-conflict transfers apply directly. Conflicts show existing/new comparisons and Keep Both, Use Existing and Use New decisions; cancellation and Escape leave the document unchanged. The destination font and style remain the destination project's choices. Incoming links among imported pages are remapped; omitted destinations remain missing instead of capturing coincident local IDs. Reused/replaced pages keep their location; new pages use the navigator insertion rules. Repeated Use Existing is a no-op when nothing changes.

## Verification boundary

Model checks cover complete page/alternate content, discussions, identity collisions, last-page Cut, nested and discontiguous selection, dependency scope, retained images, naming, stale preparations, repeated foreign import, exact history and encoded reopen. Application checks cover publication acknowledgements, stale selection, context targets, cancellation, modal boundaries, native-paste event routing and preservation of canvas controls. These checks do not establish native OS clipboard delivery, physical navigation, visual equivalence or Windows execution.

## Native same-project checkpoint

Current Scenario Review verifies navigator-focused Copy/Paste naming and new page identity, preserved note and destination links, unchanged current canvas, Cut/Paste restoration of the original identity, and exact saved Undo/Redo states. External native-resources-current/report.md and verification.json retain evidence and unchanged candidate identity. This is bounded native delivery evidence; the broader verification boundary above remains applicable.

## Native cross-project checkpoint

Three discontiguous pages transfer together with shared image bytes, notes and remapped internal links; an omitted destination remains missing. Cancellation and repeated Use Existing are exact no-ops, Keep Both remaps copied links, and Use New preserves page identity while replacing content. Native saved Undo/Redo snapshots verify atomicity. A native clipping defect in the page-conflict modal is corrected by reserving its bounded scroll viewport independently of the previous summary height. A real-pointer regression at two window sizes plus all nine affected app tests pass; native coordinate selection/apply and reopened comparison captures confirm visible actions. External native-resources-current/cross-project owns current evidence and candidate.

## Current Linux foreign-page checkpoint

The exact extracted `f944e37` Debian 12 ARM64/X11/Mesa package now passes native page-context Copy, GTK project switching and page-context Paste with the real X11 clipboard. First transfer, unchanged repeat without duplicate/allocation/empty history, changed-source conflict Cancel/Keep Both, exact saved Undo/Redo and cold Save with disabled Undo/Redo pass. Nine whole-file comparisons match expectations declared before UI input from retained native macOS output, changing only the two exact canonical source-path identities. Both source files remain unchanged; Workspace/current canvas stays intact. Isolated recovery records name only the three opened fixtures.

External `appearance-finishing-current/delivery/linux-page-clipboard/REPORT.md`, verification.json and cleanup.json retain candidate hashes, actual inputs, snapshots and cleanup. CLI fixture/manual-save setup and fresh-process launch do not establish native creation/preferences. Remote viewer delivery and settled screenshots bound native visual/input evidence. Both apps quit normally, cold driver exit 0; test infrastructure stopped, original limits/context and all 546 host data files exact. No runtime/test/build/package change. Nested/discontiguous/dependency/mixed-choice/Cut, broader native focus/accessibility/platform variants and parent gates remain open; Windows held.

## Collapsed noncurrent subtree — Beta 2 Mac sample

Published `ccca24b` Mac ARM64 Clean/list context Cut on an unselected collapsed Parent includes Child/Grandchild and isolated Child alternate20 while Sentinel4 stays current. Actual context Paste restores complete notes/content/IDs/links and hierarchy after Sentinel, with unchanged allocator21. Eight canonical whole saved states verify separate one-action Cut/Paste Undo/Redo and fresh-process Save with empty history/current Sentinel. Ten captures support hierarchy/current page and toolbar-history availability. Native menu labels were parent-observed but are not independently retained in the incremental AX files; no full-tree/menu-label artifact claim.

Independent artifact/visual/cleanup review passes. Processes579/695exit0; all546original paths/bytes/modes are restored, owned recovery records archived/removed and exact50-entry published bundle/signature retained. Private `delivery/collapsed-tree-cut-paste-native/REPORT.md`, `parent-acceptance.json` and `independent-native-review.md` retain oracles/results/exclusions. Launch is the argv fixture route, not chooser acceptance. Normal Cut/Paste replaced the clipboard with synthetic pages; original clipboard was not inspected/restored and no preservation claim is made. Multiple roots/deeper chains, shared versions, resource collisions/dependencies, last-page/failure/cadence, other input/accessibility/platform variants and parent G04/Q5/Q7/Q8 remain open. No product/build/package/release change; Windows runtime held.
