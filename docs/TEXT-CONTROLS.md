# Text controls

The Text library includes headings, subtitles, text blocks, text lines and tag clouds. These controls use the shared canvas, preview, PNG and SVG renderer.

Headings default to font40 and subtitles to24. Width can be fixed or automatic; height comes from text measurement. Saved explicit font sizes and formatting retain their values. Empty text displays the neutral default caption. Explicit line breaks, soft breaks, formatting and page-name expansion are supported; text truncates within the available width.

Text blocks start at275×80 and text lines at200×27. Clean style uses gray rectangles at half opacity. Sketch style uses original continuous pen strokes. Blocks follow the project font size, wrap words and include paragraph gaps. Lines crop a long stroke to the frame instead of squeezing the whole drawing into the available width. The original word rhythm and contours are not an exact visual match to the measured target.

Enter, double-click or Edit text converts a text block to a paragraph and a text line to a subtitle before opening its editor. Cancel keeps the conversion; Undo restores the placeholder. Applying text creates a separate history step. Conversion retains identity, origin, ordering and references. A subtitle keeps the previous line's width and measures its height. Structural conversion inside a component instance requires editing its source, consistently with other structural editing operations.

Tag clouds start at350×110. Separate tags with spaces; use `\ ` for a space inside a tag. Empty tokens are preserved, and a single token gains an ellipsis. Each identity has a stable font-size cadence. Tags wrap when their right edge plus the gap reaches the frame width. Baselines begin at25 and advance24; a row must fit strictly before the bottom edge. Rich tag text and links are supported; page-name macros remain literal in visible tags.

Inspector edits and pointer resizing enforce a ten-unit minimum. Small saved dimensions remain intact. Title height is interpreted as automatic even if an older file contains a fixed height; rendering does not rewrite that original data. Larger fonts, nested clipping, component source/instance behavior and platform acceptance still require the broader product checks recorded in SESSION-SPEC.md.
