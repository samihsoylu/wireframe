# Static media controls

Volume slider, Playback controls and Webcam are static wireframe visuals. They share the canvas, thumbnail, library preview and PNG/SVG renderer and do not access system audio or cameras.

| Control | Default size | Resize behavior |
|---|---|---|
| Volume slider | 72 × 16 | Horizontal only; two end handles; minimum width 10 |
| Playback controls | 110 × 36 | Intrinsic fixed size; no resize handles |
| Webcam | 177 × 146 | Independent width and height; minimum 10 on either edited axis |

Volume uses a fixed 52-unit left group and 2-unit right margin, extending the trailing line at widths above 54. Narrower saved widths compress the fixed grouping. There is no adjustable volume value. Old unknown properties remain serialized for compatibility but do not affect this drawing.

Playback uses three original previous/play/next circles. Explicit stored dimensions do not change its intrinsic size. Webcam stretches its original portrait and opaque background in each direction; Shift during a corner resize retains its aspect ratio. Sketch uses a white outlined smiling portrait; Clean uses a white silhouette on gray.

The interactive minimum applies to edits, not loading or selection. Smaller saved dimensions are retained. Resizing is undoable and persists through saving, including local component overrides.

Geometry is in `src/media_controls.rs`; original artwork is in `src/render/media_controls.rs`. `examples/render_media_controls.rs` creates a synthetic gallery, and `--geometry <input.json>` emits diagnostic layouts. Native canvas fills use convex pieces, including the speaker and portrait neck/shoulders, so the canvas and exports agree. Regression tests cover this requirement, constraints, original unknown-property retention, fixed dimensions, component overrides and clipping. These checks do not establish exact pixel equivalence with other software or native Windows/Linux behavior.
