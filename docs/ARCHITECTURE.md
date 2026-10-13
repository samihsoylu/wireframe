# Architecture

Wireframe uses egui/eframe 0.36.2 and a custom Rust canvas. Native file dialogs use rfd. The UI toolkit supports macOS, Windows and Linux; actual platform checks are tracked separately.

- document: recursive version-2 data, validation, component expansion and legacy migration.
- model: legacy file representation and temporary control-renderer view.
- workspace: live recursive editing, transactions/history, nested contexts, clipboard dependencies and derived views.
- editor: legacy implementation retained for migration/regression tests.
- scene: deterministic drawing primitives consumed by the canvas and SVG/PNG exporters.
- storage: bounded reads, validation, atomic writes and recovery paths behind ProjectRepository for the desktop app.
- app: interface composition, input routing, dialogs and user workflows.

The design favors small responsibilities, a narrow storage interface, and shared rendering behavior. Closed sets of controls use enums; no artificial class hierarchy is needed. History stores before/after document snapshots with a size cap; profiling can guide a later delta representation.

Documents use versioned .wireframe JSON, stable object identifiers and explicit properties. Loading a malformed or unsupported document must not replace the current project. New/Open/Quit guard unsaved changes. A five-second recovery cadence writes a separate local copy, never a remote service.

References for the chosen libraries: https://docs.rs/eframe/0.36.2/eframe/ and https://github.com/emilk/egui.

## Recursive document transition

The desktop app uses the recursive `document` model, workspace editor and ProjectStore. Rendering is a temporary projection into the existing control primitives; projections never replace project data. Version-1 reads migrate without rewriting the input, while saves/recovery use version 2. See DOCUMENT-FORMAT.md and RECURSIVE-EDITOR.md for scoped identity, parent-local geometry and native verification.

The catalog now owns control identity, common property contracts, defaults, inspector visibility and resize policies. The layout engine consumes expanded recursive content plus a TextMeasurer and emits geometry/clip information without changing documents. The egui adapter measures actual glyph advances. Catalog inventory, layout support and rendered control support are separate acceptance measures; see CATALOG-AND-LAYOUT.md.
