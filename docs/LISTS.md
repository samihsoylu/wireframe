# List controls

List text is one rich-text row per newline, including blank rows. The optional first row is a header and is excluded from the selection menu. Data-row selection is zero-based, with -1 meaning None. Reordering preserves the first matching raw row; same-count replacements retain the selected index. A changed count without that row clears selection. Selection and text are one undoable edit, including component-local overrides.

New Lists use automatic dimensions, minimum100×126. Width follows the widest measured row plus16. Row height is at least font size+8, default21. Sketch top padding is3, clean2; bottom3. Without a header, height is padding plus all rows. With a header, automatic height uses the first row height+6 and n-2 data-row heights; the painted header uses top padding+6+first row height. This intentional compatibility rule can clip the last row. Fixed dimensions are preserved until Auto-size.

Canvas, library previews, thumbnails and exports use the same clipped scene. Text starts8 units from the left, does not wrap and truncates to a fitting prefix. Selected rows use blue fill; alternating rows and dark-fill text contrast are supported. Header background and outline remain visible when the outer border is hidden. Optional16-unit scrollbar overlays the right edge and changes only its thumb position. It does not scroll content. Blank space below text continues the row fills.

The inspector exposes text/font styles, header, selection, row height, fill/alternate fill, border and scrollbar. Local component edits preserve source data and support per-property Reset. Tests: tests/list_control.rs. Gallery: cargo run --example render_list. Local native evidence: evidence/list-control/.

Built-in row icons are available through a searchable offline picker; see [icons](ICONS.md). Custom image icons remain open. Inline text link formatting is decorative inside Lists; row navigation is configured separately. Exact fonts/contours and broader native edge-case/platform acceptance remain open.


## Row navigation

Row links are an ordered `Value::Links(Vec<Option<Link>>)` property. Each raw row, including a header, may target a page, HTTP(S) URL, Back or None. The inspector's expandable row editor exposes these choices. URL input remains a draft until Apply; invalid/partial input cannot enter the document. Reset returns to default or inherited component row links.

Text edits match raw labels in occurrence order, consuming duplicate labels sequentially. Unmatched labels retain their old position's metadata only when row count is unchanged. Empty text clears targets. The whole transaction uses its initial metadata, so clearing text before replacement does not lose links. Component edits retain local ownership. Missing page targets and oversized/invalid link arrays fail document validation; page deletion and cross-document paste clear unavailable page targets, including source definitions and override patches.

Visible row backgrounds, including the header and partially clipped rows, provide shared scene hit regions. Ancestor clips and later controls limit them; the List scrollbar column remains part of the row region. Presentation follows page/URL targets and tracks page history for Back without modifying project data. Starting a new presentation clears that history. Standalone SVG retains web links as transparent row-sized anchors. Internal page and Back links have no standalone-SVG destination and are omitted; linked multi-page/PDF export remains open.

Verification: tests/list_links.rs plus the app's real-canvas presentation test; local native evidence evidence/list-links. Native macOS verifies target change/Undo, background navigation/Back, reordering, partial URL draft/Apply, save/cold reopen and SVG URL anchors. External URL browser launch and native component editing were not exercised. New/duplicate destination shortcuts, editor Alt-click navigation, custom image icons and broader platform/fidelity acceptance remain open.
