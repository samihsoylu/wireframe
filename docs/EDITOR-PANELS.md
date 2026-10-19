# Editor panel fidelity

The target is a compact page navigator, a separate component-source navigation mode, a control/component library, and an inspector whose contents follow the current selection. Keep original product code/assets and all current editing capabilities.

## Current increment

The page navigator uses separate, labelled thumbnail/list icon buttons and white page cards with dark captions. Blue identifies the selected row around the card. Thumbnail drawings fit the card, including magnifying small artwork. A duplicate page-add button is removed; the main toolbar's New wireframe action remains. Existing named-version selection moves below the page list alongside the retained component/layer controls. This footer measures its contents, grows when sections expand, and scrolls within a bounded share of the available height.

Presentation lives in `src/app/navigator.rs`; page switching and mutations stay in the existing editor. List mode must not render page contents, and off-screen thumbnails must remain unmaterialized. Switching views or selecting pages must not edit the document or add history.

## Complete action destinations

| Capability | Current route | Target / remaining work |
|---|---|---|
| Add page | Main toolbar | Preserve toolbar and shortcut. |
| Thumbnail/list selection | Navigator heading icons | Preserve selection/context on switch and keyboard access. |
| Page rename/delete | Navigator Rename modal and recoverable Trash | Redundant inspector actions removed after replacement routes were verified. |
| Page duplicate/cut/copy/trash | Hierarchical Duplicate, recoverable Trash and whole-page Cut/Copy/Paste in both navigator views | Preserve the selection/history and publication contracts in PAGE-CLIPBOARD.md and PAGE-NAVIGATOR.md. |
| Alternate versions | Page inspector and page context menu | Preserve all creation/rename/duplicate/promote/merge/discard operations. |
| Named version across pages | Versions selector below page list | Keep reachable when final panel structure removes this temporary footer. |
| Component insert/source editing | Top library, Quick Add, source navigator and instance inspector | Shared previews and double-click/drag/Quick Add insertion replace the duplicated source-workspace footer. Page-mode creation/navigation remains. Source mode retains Versions and Layers. Empty-workspace entry and broader layout fidelity remain open. |
| Layer selection | Collapsed Layers section | Retain locked/nested selection through an explicit View surface before removing the footer. |
| Project rename | File → Rename project in native and fallback menus | Selected-name dialog and undoable metadata change implemented; saved file path unchanged. |
| Page notes | Page inspector | Page-name header, readable notes, inline editor with Save/Cancel, and separate versions region; transaction contract below. |
| Control text | Inspector and double-click/Enter editor | Preserve text entry, commit/cancel, formatting and Undo when compacting inspector. |
| Geometry/layering/alignment | Inspector | Compact controls with existing enable/lock/resize constraints. |
| Group/component overrides | Context-sensitive inspector | Preserve crop, source/instance distinction, overrides/reset and return paths. |

## Acceptance

Check empty, long-title, multipage, selected/unselected, alternate-version, grouped and component-source states. Verify narrow and ordinary windows, native pointer/keyboard navigation, page/version switching and save/reopen preservation. Keep current performance bounds for hidden page content. Document actual platform coverage; macOS visual evidence does not establish Windows or Linux display fidelity. Broader panel relocation and native acceptance are still in progress.

## Page inspector implementation

Use the official page name as the inspector header, including while an alternate is selected. Notes occupy the primary body; alternate versions occupy a separate region below a divider. Hide the official-version row when no alternates exist, while retaining the add action. Keep all existing alternate actions and the named-version selector reachable.

Notes start in a readable state. Activating them opens an inline editor with Cancel and Save. Escape cancels; ordinary Enter inserts a newline; Command/Control-Enter saves. Clicking the canvas or changing page/version commits the old notes before departure. Each completed edit is one document history operation. An alternate owns its notes separately from the official page.

Bind editing state to the project session, page and alternate. Preserve input ordering and commit before Save/export or other document commands; never apply an old draft to a new surface. Cancelling must leave contents and document history unchanged. Verify focus changes, modal boundaries, inspector hiding and save/reopen as well as the ordinary editing path. Existing active-surface projection already supplies the correct notes; do not replace it with direct official-page reads.

Implemented and accepted within the current macOS scope: seven focused regressions,182 application tests, all-target Clippy and optimized build pass. Native ordinary and narrow windows verify readable/editing states, history, alternate ownership, page departure, project rename and byte-identical reopen/resave. Long Unicode notes round-trip through native paste and scroll correctly. Automated typing did not deliver all non-ASCII characters, so physical typing/IME remains unverified. A panel-width regression caught during native review is corrected and covered by an idle/editing stability test.

Rich-note formatting/links, an adjustable internal split, complete typography, title-validation edge fidelity and current other-platform acceptance remain open. This is scoped self-review, not independent design acceptance.

## Component library and source workspace contract

A component library is an editable collection with a name, its own notes, ordinary controls and reusable sources placed together on a canvas. Each source has its own name, notes and content. The source navigator is a collapsible library/source list. The insertion library exposes source artwork/name previews across libraries, excluding ordinary library controls. Empty sources remain valid entries. Creating a component from the page reuses or creates a default project library and leaves an instance in that page.

Enter the source workspace from Edit or a selected instance's Edit Source action. Show an explicit Back to wireframes action. Library selection displays the complete library; source selection enters that source. A visibility toggle hides surrounding library contents or shows them faded while editing one source. The source inspector shows source notes and usage links; library selection shows library notes. Context routes provide rename/duplicate/copy/cut/delete at the appropriate scope. A library action creates an empty source and enters it; the main toolbar creates a library.

The tested Back/usage-link route returns to the current wireframe with control selection cleared and its viewport retained. Multipage/alternate and nested-source return need further native evidence. Do not infer general return semantics from the one-page probe. Independent source/library notes and empty-source creation are established. Inter-library source movement is now implemented with native product drag/menu, exact history and cold-reopen evidence; see [source movement](SOURCE-MOVEMENT.md). Exact reference drag parity remains unverified.

Version6 extends the canonical Definition{id,name,content} model with library metadata, ordinary library content, source placements and source notes. Definition/instance identities and local override paths remain unchanged. Migration, validation/limits, allocator reservations, dependency checks, clipboard, recovery, history and surface lookup have model coverage. The editor core now supports library/source navigation, notes, empty-source creation, usage destinations, canonical source rendering and independent viewports. Populated source navigation, independent source/library notes, usage links, and explicit Back are connected. Resource-focused Duplicate/Delete route to canonical source operations; clipboard/group commands do not fall through to canvas or page actions. Rename uses the existing transaction-bound modal. Toolbar creation adds a library in source mode, and each library offers empty-source creation. Source insertion now uses the shared top-library catalog; Versions and Layers remain reachable below the source tree. Starter content must be independently authored. Retained-library lifecycle is implemented under [library lifecycle](LIBRARY-LIFECYCLE.md). Exact sorting/collision rules, rich notes, large-library behavior and broader portable-platform acceptance remain open.

Source root appearance is a prerequisite for library-level editing actions. Preserve shared crop/markup separately from child content and distinguish inherited appearance from an explicit local clear. Source-focused editing shows full child content; the library placement and inheriting page instances apply the shared crop. Version7 now supplies shared root appearance on owning source placements, explicit instance root overrides, migration/recovery and conversion/reset/clipboard/history/rendering coverage. Source root-lock selection/rendering and clipboard destinations have the scoped follow-ups in COMPONENTS.md and LIBRARY-TRANSFER.md; navigation alone does not establish those behaviors. Full toolbar/panel fidelity, including zoom icons, spacing and action grouping, remains part of the design pass; current differences are not intentional product divergence.

Library source actions follow the identity and lock contract in COMPONENTS.md. Duplication and library-root Group/Ungroup now have explicit atomic editor operations; populated source panels and root-lock rendering/selection are connected and have the scoped follow-ups below. Distinguish resource selection from canvas selection: source duplication/grouping cannot fall through to ordinary instance commands. Locked source entry still permits Quick Add; do not disable the entire workspace as read-only.

## Source panel integration limits

Notes drafts now bind either to an explicit page/alternate pair or to an explicit library/source identity. Save, Cancel, validation, departure and history preserve that owner; an invalid draft blocks Back. Source notes fill the primary inspector region with usage links below. Library notes use the available region without page alternate actions. Source rows use stable identities and Duplicate transfers keyboard focus to the new resource before another resource action. Unsupported resource clipboard and grouping cannot fall through to a selected canvas control.

The original populated-navigation increment is superseded for Edit entry, shared catalog insertion, root-lock behavior and library lifecycle by COMPONENT-CATALOG.md, COMPONENTS.md and LIBRARY-LIFECYCLE.md. Full keyboard tree traversal, rich notes, adjustable internal splitting, alternate/nested usage labels and exact source-row styling retain their separate acceptance limits. Native acceptance remains scoped to the candidate and cases actually exercised.

Source owner-lock clarification: explicit source entry remains uncropped, and children without their own locks remain selectable. Disable geometry and destructive selection commands for the inherited owner lock while retaining Quick Add and text editing. Text changes can still update automatic measurements. Unlock All affects immediate children, so the owner is unlocked from library scope. The earlier no-selection interpretation is superseded; these operation-specific restrictions now have model/application checks and bounded native macOS acceptance.


## Catalog replacement gate

The source-mode lower Components list was removed after the shared catalog replacement checks in COMPONENT-CATALOG.md. Top-library previews, double-click/drag insertion and Quick Add preserve stable source identities, cycle prevention and current project appearance. Versions and Layers remain reachable. The page workspace keeps its component creation/navigation section.

## Panel gesture verification

Fast movement/release batches resize visible editor and presentation panes without moving canvas artwork or changing document history. The shared handler rejects stale, hidden, blocked and cancelled presses, and gives the resizing pane ownership of that frame. Native macOS checks cover navigator, selected/unselected inspector, comments and presentation panes, followed by ordinary canvas movement and exact Undo. Retained Linux package checks cover sampled pane interactions; physical held cadence and complete platform/keyboard acceptance remain separate.

## Component navigator keyboard contract

When a component row owns keyboard focus, Up/Down moves through visible library and source rows. Shift extends or contracts the selection from its anchor; the current editing surface stays on the existing selected member while that member remains selected. Left collapses a library heading and Right expands it, without entering its first child. These keys do not move a source row to its parent; Return leaves the selected resource unchanged. Collapsed children are excluded immediately, including when several key events arrive before repaint.

Row navigation reveals the destination and retains list focus. The UI framework must not apply a second directional focus movement. Following Copy/Duplicate commands use the newest resource owner; a duplicate supersedes an earlier pending row focus. Save restores the focused resource row and preserves a multi-selection anchor, including when the next key arrives on the save-completion frame. Text fields and blocking dialogs retain their input. Navigation alone must not change project data or drawing history. Broader keyboard traversal, platform and accessibility acceptance retain their separate limits.

Scoped acceptance: 31 component-navigation and 17 persistence tests cover the new routes, ownership handoffs and both fast/delayed Save. Native macOS verifies ranges, visible-row traversal, heading expansion, source-row no-ops, Save focus and subsequent duplication, with exact saved Undo/Redo. All 529 prior recovery/save-setting files are unchanged and both review processes are closed. External `source-keys-current` retains reference observations, failing-before evidence, candidate identities and the native Save-focus discovery. New Linux/Windows acceptance and broader keyboard/IME/accessibility remain open.
