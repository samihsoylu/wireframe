# Wireframe

A local desktop workspace for turning rough ideas into clear wireframes. Built in Rust for macOS, Windows and Linux. No account or cloud service is required.

## Features

- 37 controls for forms, text, shapes, navigation and layout.
- Pages, notes, searchable library and layers.
- Move, resize, align, nest groups, duplicate and lock controls.
- Text-defined menus and menu bars with shortcuts, selection, row links and optional bar icons.
- Undo/redo, local project files and recovery copies.
- Sketch and clean styles; PNG and SVG export.
- [Project fonts](docs/PROJECT-FONTS.md): import four font faces, save them with the project, and use them on canvas and in exports.
- Page links and presentation mode, including individual links on list rows, checkbox/radio group rows, button-bar segments, breadcrumbs, link bars and tabs.
- Searchable offline icons and imported project images for Lists, Buttons, Dropdowns, circular buttons, Icons and Icon labels, with size/rotation controls, undoable assignments and self-contained exports.
- Embedded PNG/JPEG/GIF images, shared sources and replacement with Undo.
- Canvas image cropping and quarter-turn rotation, including local component edits.

## Run from source

Requires Rust 1.95 or newer and platform build dependencies. Run `cargo run --release`. Use `cargo test` for behavioral and persistence checks.

On Linux, install a C toolchain, pkg-config, and the development packages for X11, Wayland and OpenGL. File dialogs use the desktop portal.

## Project files

Editable documents use the versioned `.wireframe` JSON format. Files are validated before saving and replaced atomically. Projects remain on your computer. A local recovery copy is offered after an interrupted session.

## Development status

The first native build runs on macOS. Windows/Linux verification and packaging are in progress. See [current state](SESSION-SPEC.md) for actual checks and remaining work. The desktop editor supports nested groups and version-2 project saving. Double-click a group to edit its children; use Escape or Exit group to return. Basic rich text and table formatting appear on canvas, thumbnails and PNG/SVG exports. The text/table inspector includes styles, Reset, Auto-size and six checkbox/radio states. Create a reusable component from a selected control or group in the inspector. The Components sidebar inserts instances and opens the shared source; Detach component makes an independent group. Double-click an instance to edit its children locally; Reset resumes the source value. Deleting a component source retains missing-instance placeholders and can be undone in the current session. Empty sources and groups remain visible and editable; adding children populates them again. Image controls import embedded PNG/JPEG/GIF files and share their sources; instances can resize without changing their source. Final typography, full control coverage and asset editing remain under development.

## License

GNU General Public License v3.0 only (`GPL-3.0-only`). See [LICENSE](LICENSE). Copyright 2026 Wireframe contributors. Dependency licenses remain applicable; see [third-party notices](THIRD_PARTY_NOTICES.md).
