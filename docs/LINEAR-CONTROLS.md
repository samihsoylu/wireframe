# Scroll indicators and dividers

Horizontal and Vertical scrollbars start at 100×16 and 16×100. The inspector controls their position from 0 to 1, starting at 0. A 21×8 thumb appears only when length exceeds 45. The short-axis thickness remains 16. The background starts one unit inside the frame, with each dimension reduced by two and clamped to at least one. These are illustrative controls for wireframes; their arrows do not scroll the editor.

Horizontal and Vertical splitters start at 100×12 and 12×100. They have two rails and a centered three-dot grip, with transparent space between marks. Their thickness remains 12 and they have no editable text or state.

Both families and the sliders use two canvas end handles. Resizing a single control has a minimum length of 10, preserves the opposite edge and ignores Shift aspect scaling. Existing smaller saved dimensions continue to render and are not rewritten by selection. Thumb travel at lengths 46 and 47 retains its slight reverse movement as the value increases.

The library, inspector, canvas, thumbnails and PNG/SVG exports share measured geometry and original vector artwork. Value edits support Undo/Redo, persistence and component-local overrides. The example `examples/render_linear_controls.rs` produces a 34-control gallery in both styles, including values, short controls and thumb visibility boundaries.

Exact artwork contours and native Windows/Linux acceptance remain open; SESSION-SPEC.md records current validation and remaining work.
