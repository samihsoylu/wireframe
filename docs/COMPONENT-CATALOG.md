# Shared component catalog

Status: shared model, previews, top-library tiles, Quick Add, placement and drag integration implemented. Combined category selections persist per project. Native macOS replacement insertion, narrow/full-width layout and saved category persistence are verified; broader platform and performance acceptance remain open.

## Identity, preview and eligibility

Use stable typed item identities for built-in controls and component sources, independent of display names. The Components category lists live sources across libraries. All includes component entries. Quick Add searches eligible entries and preserves separate items with duplicate names. Categories support Shift selection and a deduplicated union with stable name ordering; persist the selection with project view preferences.

Preview a canonical fresh instance, including crop/root appearance, nested dependencies, project fonts/colors and assets. Editing/renaming/removing/restoring a source and dependency/settings changes must refresh the appropriate labels/previews. Use bounded caching, not full-project cloning on every frame.

Disable the active source and transitive dependents to prevent cycles; whole-library scope shows component items disabled. Keep unavailable items visible in their category but exclude them from Quick Add. Recheck source existence, dependency safety and destination structure-edit constraints when committing insertion. Fail atomically without allocating lasting IDs or changing Undo history. Existing document representations remain supported.

## Interaction and migration

Share the existing held-preview drag lifecycle with built-in controls. Preserve image-region activation, pointer anchor, coalesced/release events, cancellation and one-step insertion Undo. Preview and final insertion must agree. Quick Add and double-click use canonical default placement. A stale source, changed surface, Save boundary or modal must not commit a former payload.

Move source insertion into the top library before removing the duplicate lower Components list. Retain current layer-selection and project-version capabilities until their replacements are verified. The library and keyboard/accessibility activation must work without a pointer event; absent modifiers mean an ordinary activation.

## Acceptance still required

Verify page and nested-source insertion, direct/indirect cycle rejection, duplicate names, empty components, cropped/root appearance, dependency/asset/font changes, source removal/restore, multi-category deduplication, Quick Add exclusion, cancellation/stale gestures, one-step Undo and save/reopen. Inspect actual narrow/full-width desktop behavior. Retain existing built-in drag and typography coverage. Other-platform native checks remain required.

## Core implementation checkpoint

`workspace::CatalogItem` separates control kinds and component identities. Catalog entries and category unions preserve duplicate names and include disabled sources; Quick Add search excludes unavailable choices while retaining existing built-in alias matching and ranking. The top library and Quick Add now use this catalog.

Insertion rechecks availability, destination structure rules and direct/transitive recursion before mutation. Whole-library component insertion is disabled through this catalog API; existing document representations and the lower-level insertion API remain supported. Cycle checks include retained definitions because the document must remain valid after Restore. Pending edits survive failed catalog insertion, including exhausted identity allocation; successful insertion uses existing one-step history.

`ComponentPreviews` renders a fresh instance from the reachable live definitions, shared root appearance and referenced assets. It preserves nested overrides and missing/empty components through the existing resolver/renderer. Cache hits compare borrowed dependencies without cloning the complete project. The cache retains at most 32 entries and 32 MiB of serialized input; this is not a measured bound on total renderer memory. Unrelated page notes, unrelated source changes and unused assets do not invalidate an unchanged entry. Changes to used assets, dependencies, appearance, font or settings do. Large-document cadence and memory profiling remain open.

The app carries typed identities and captured preview/surface/context/session through the existing drag lifecycle. Changed or unavailable sources, changed previews and modal/Save boundaries cancel stale gestures. Only visible tiles render. Empty categories retain the thumbnail row height. Disabled tiles use the UI framework opacity once.

After native replacement insertion checks, the duplicated Components footer is removed only in the source workspace. Versions and Layers remain; the page workspace keeps its existing component-creation/navigation route. Large-document cadence, renderer memory, physical held-preview cadence for components and native Windows/Linux acceptance remain open.

## UI acceptance checkpoint

Final251 application tests and all-target Clippy pass. Eight new app regressions cover category composition/history, component Quick Add/default placement, disabled/stale sources, preference reopen/legacy defaults, held/release gestures at three zooms, seven invalidation modes and coalesced input. Sixteen model/preview catalog regressions passed before the final footer/opacity-only changes.

Native macOS verifies physical double-click, automated drag release, Quick Add on a page and inside a source, accessible insertion, one-step toolbar Undo, disabled self/whole-library entries, excluded Quick Add results, combined categories and per-project/fresh-process persistence. Both saved fixtures return byte-for-byte to baseline. Full-width and941-logical-pixel layouts retain Versions/Layers; landing and phone sketches fit. This is scoped self-review. Physical held cadence for component artwork, large documents, Windows/Linux native behavior and broader layout/clarity remain unverified.

## Category and preset implementation

Eight distinct catalog presets now cover mobile alert alignment/buttons, circular slider thumbs in both orientations, a borderless icon list, thin progress, rectangular search with microphone, an underlined input and a bordered pointed button. All contains 94 built-in choices plus reusable components. Each preset has a stable identity, ordinary saved control properties, its own matching preview and one-step insertion Undo. Quick Add, keyboard activation, double-click and drag share the preset creation path.

Android, Big and iOS categories are now explicit alongside the refined memberships of the other control categories. Combined categories deduplicate identities while preserving default and alternate versions of one kind. Legacy Navigation/Charts preferences remain visible when selected. Existing component eligibility, preview caching and stale-gesture guards remain in place.

Progress controls have standard and thin styles, value, fill and indeterminate state. Automatic dimensions are 100 by 20 or 100 by 10; saved fixed geometry remains intact. They do not expose a caption editor. The shared renderer supports these properties in canvas and exports, with original line and stripe artwork.

Asset and individual-icon catalogs were implemented in subsequent checkpoints; see ASSET-CATALOG.md and ICON-CATALOG.md for their current contracts and evidence. Category typography, complete naming fidelity, exact sketch contours and broader native platform acceptance retain their scoped limits. Built-in count alone does not establish complete library fidelity.

## Compact category strip

Categories use the measured 11-point interface style in a centered 30-point row. Buttons retain fixed text sizing and 4-point spacing when the viewport narrows, with horizontal overflow access and focus reveal. Components is immediately before Text; stored identities and legacy categories are unchanged. Category activation shares pointer-event modifier handling with the source navigator so a quick Shift release cannot accidentally replace a combined selection. See UI-TYPOGRAPHY.md for the layout contract and verification scope.
