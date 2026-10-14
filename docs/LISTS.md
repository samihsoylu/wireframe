# List controls

List text is one rich-text row per newline, including blank rows. The optional first row is a header and is excluded from the selection menu. Data-row selection is zero-based, with -1 meaning None. Reordering preserves the first matching raw row; same-count replacements retain the selected index. A changed count without that row clears selection. Selection and text are one undoable edit, including component-local overrides.

New Lists use automatic dimensions, minimum100×126. Width follows the widest measured row plus16. Row height is at least font size+8, default21. Sketch top padding is3, clean2; bottom3. Without a header, height is padding plus all rows. With a header, automatic height uses the first row height+6 and n-2 data-row heights; the painted header uses top padding+6+first row height. This intentional compatibility rule can clip the last row. Fixed dimensions are preserved until Auto-size.

Canvas, library previews, thumbnails and exports use the same clipped scene. Text starts8 units from the left, does not wrap and truncates to a fitting prefix. Selected rows use blue fill; alternating rows and dark-fill text contrast are supported. Header background and outline remain visible when the outer border is hidden. Optional16-unit scrollbar overlays the right edge and changes only its thumb position. It does not scroll content. Blank space below text continues the row fills.

The inspector exposes text/font styles, header, selection, row height, fill/alternate fill, border and scrollbar. Local component edits preserve source data and support per-property Reset. Tests: tests/list_control.rs. Gallery: cargo run --example render_list. Local native evidence: evidence/list-control/.

Row icons and per-row navigation are still missing. Inline text link formatting is decorative inside Lists; it does not establish row navigation. Exact fonts/contours and broader native edge-case/platform acceptance remain open.
