# Architecture

Wireframe uses egui/eframe 0.36.2 and a custom Rust canvas. Native file dialogs use rfd. The UI toolkit supports macOS, Windows and Linux; actual platform checks are tracked separately.

- model: versioned data, geometry, control definitions and validation.
- editor: selection, edit transactions, undo/redo, grouping, alignment and clipboard transformations.
- scene: deterministic drawing primitives consumed by the canvas and SVG/PNG exporters.
- storage: bounded reads, validation, atomic writes and recovery paths behind DocumentStore.
- app: interface composition, input routing, dialogs and user workflows.

The design favors small responsibilities, a narrow storage interface, and shared rendering behavior. Closed sets of controls use enums; no artificial class hierarchy is needed. History stores before/after document snapshots with a size cap; profiling can guide a later delta representation.

Documents use versioned .wireframe JSON, stable object identifiers and explicit properties. Loading a malformed or unsupported document must not replace the current project. New/Open/Quit guard unsaved changes. A five-second recovery cadence writes a separate local copy, never a remote service.

References for the chosen libraries: https://docs.rs/eframe/0.36.2/eframe/ and https://github.com/emilk/egui.

## Recursive document transition

The `document` module now defines version-2 recursive data, validation, component expansion and legacy migration. `ProjectStore` is an opt-in storage API. The current application/editor still use the version-1 `model` and `JsonStore`. This boundary prevents partial editor support from overwriting richer data. See DOCUMENT-FORMAT.md for scoped IDs, parent-local geometry and verification. Next integrate effective properties and shared layout before switching the application.
