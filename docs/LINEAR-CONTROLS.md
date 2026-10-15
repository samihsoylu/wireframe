# Scroll indicators and dividers

Horizontal and Vertical scrollbars start at 100×16 and 16×100. The inspector controls their position from 0 to 1, starting at 0. A 21×8 thumb appears only when length exceeds 45. The short-axis thickness remains 16. Clean backgrounds start two units inside the frame, with each dimension reduced by four and clamped to zero. Sketch retains its one-unit background inset. These are illustrative controls for wireframes; their arrows do not scroll the editor.

Horizontal and Vertical splitters start at 100×12 and 12×100. They have two rails and a centered three-dot grip, with transparent space between marks. Their thickness remains 12 and they have no editable text or state.

Both families and the sliders use two canvas end handles. Resizing a single control has a minimum length of 10, preserves the opposite edge and ignores Shift aspect scaling. Existing smaller saved dimensions continue to render and are not rewritten by selection. Thumb travel at lengths 46 and 47 retains its slight reverse movement as the value increases.

The library, inspector, canvas, thumbnails and PNG/SVG exports share measured geometry and original vector artwork. Value edits support Undo/Redo, persistence and component-local overrides. The example `examples/render_linear_controls.rs` produces a 34-control gallery in both styles, including values, short controls and thumb visibility boundaries.

Exact artwork contours and native Windows/Linux acceptance remain open; SESSION-SPEC.md records current validation and remaining work.

Clean standalone and embedded scrollbars share one original drawing function. Tracks, arrows and capsule thumbs use #515151, including within disabled fields. Four filled one-unit edge strips keep square corners opaque. Arrow compartments have no separator rules. Vertical arrows measure four units along the axis by eight across; horizontal arrows measure five by nine and sit one unit closer to the ends. Thumbs retain their 13/14-unit end allowance, rounded radius four and existing visibility threshold. Wider embedded tracks scale arrow width while keeping the thumb centered and eight units wide. Placement, covered-link handling and Sketch rendering remain the callers' responsibility.

Pixel regressions cover solid frame corners, axis-specific arrows, capsule shape, short tracks, and standalone/embedded equivalence through normal, focused and disabled text fields. Translated vector antialiasing can differ by one 8-bit channel level; the equivalence check bounds that difference. Current native acceptance is recorded in SESSION-SPEC.md.
