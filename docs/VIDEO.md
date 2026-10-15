# Video player

Video is a static media placeholder with an opaque white frame, play symbol, fixed progress fill, speaker and fullscreen symbol. It has no media source, playback state, text editor or page destination.

New controls start at 300 × 200. Width and height resize independently, with a 10-unit minimum during inspector and pointer edits. Smaller saved dimensions remain intact when loaded or selected. Shift resizing preserves the starting aspect ratio; corner and side handles preserve their opposite edge or center. Components retain local size overrides and normal Undo/Redo.

Original vector artwork uses fixed outer regions and a stretching center. Sketch regions are top 7, left 146, right 74, bottom 43; Clean regions are 1, 137, 71, 39. Opposing regions compress proportionally below their combined size. Middle pieces overlap by a quarter unit on either side; corners overlap by a half unit when the center collapses, including the exact threshold. The separate white background has a one-unit inset and minimum dimensions of one. All output is clipped to the control and its parent.

Canvas, library previews, thumbnails and PNG/SVG exports use the same scene. The renderer reuses the original media drawing primitives and convex fills. `examples/render_video.rs` produces a sizing gallery and can emit geometry for supplied diagnostic cases. Geometry checks do not establish exact visual identity or platform acceptance.
