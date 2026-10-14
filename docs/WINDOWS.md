# Windows

Window creates a fixed450×400 desktop frame with a rich title, header and footer. Header/footer heights, minimize/maximize/close buttons, resize handle and scrollbar are independently editable. Font size and emphasis affect the title; the outer frame stays fixed. All edits support Undo, Redo and per-property Reset.

Titles reserve space for chrome even when buttons are hidden. A zero-width title area uses outer-frame clipping; positive narrow widths truncate. Macros remain literal. Inline title links receive their text appearance without creating presentation hit regions. The scrollbar position moves its thumb; it does not scroll other controls.

Header height grows to accommodate measured title height. The footer appears only when it fits below the header. The scrollbar continues to reserve footer space when the footer itself is hidden. Chrome buttons and resize handles disappear at their size thresholds. All drawing is clipped to the frame and its parent crop.

Legacy `panel` identifiers still resolve to Window. Explicit old dimensions, text and font settings survive migration and round trips. New insertion uses current catalog defaults. Window already belongs to the41 implemented families.

Run `cargo run --example render_windows -- /path/to/output` for an editable desktop-layout example and both-style PNG/SVG exports. Six focused tests cover narrow titles/chrome gates, footer/scroll reservation, history/components/storage, clipping/link exclusions, thumb thresholds and legacy settings.

Native macOS checks cover insertion/defaults, inspector editing, header Undo/Redo, chrome flags, multiline title, scrollbar position, both-style exports and full-process save/reopen. Four native exports are byte-identical to shared-renderer exports; the restarted project's resave is byte-identical. Fonts/chrome are original, and exact visual fidelity remains open. Per-control proximity snapping, a visual header/footer drag inspector, advanced editing, platform text adapters and native Windows/Linux acceptance remain separate work.
