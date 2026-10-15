# Image carousels

Cover Flow is a seven-panel perspective diagram with an optional embedded center image. It starts at 493 × 198 and stretches independently on both axes. Editing dimensions has a 10-unit minimum; opening or selecting a smaller saved control does not rewrite it. The outline is transparent in both styles and uses original vector artwork.

The center image occupies 31% of the frame width and 78% of its height. Import, source selection, removal and sketch filtering share the image pipeline. Rotate turns the image by a quarter turn while preserving the control position, dimensions and stored asset. Cropping and automatic image sizing are not offered for this diagram. A missing source leaves the outline visible. Changes support Undo/Redo and project persistence; component-local source and rotation changes preserve the shared definition.

The optional horizontal scroll indicator is 16 units high and appears only above a control height of 16. It reserves that height from the artwork. Its width is 75% of the control width, rounded to the nearest unit, and its centered position is rounded too. The shared scrollbar thumb appears when the rounded bar width exceeds 45. Its value is editable from 0 to 1; it illustrates scroll position without moving the editor canvas.

Canvas, thumbnails and PNG/SVG exports share the same geometry, raster transformations and clipping. `examples/render_cover_flow.rs` generates a 23-control gallery in both styles and accepts an optional image fixture path. `tests/cover_flow.rs` covers geometry boundaries, asset-preserving rotation, filtering, removal, missing sources, clipping, exports, history, persistence and local component edits. App tests cover tiny saved selection, pointer minimums and absence of a text editor.

Native macOS checks cover insertion, sizing, minimums, image editing, history, save/restart and export equality. Component-local changes and one-unit selection are covered by automated tests; this checkpoint does not establish their native acceptance. Exact artwork and image resampling remain under refinement. Windows/Linux native acceptance remains open.
