# Static layout controls

Formatting toolbar, Street map, Modal overlay and Vertical rule are available from the library and Quick Add. They share canvas, thumbnail, PNG and SVG rendering. These are wireframe controls: toolbar icons and the map do not invoke editing or navigation actions.

Formatting toolbar starts at 261×29. Only its width changes; icons, separators and the style menu appear when there is sufficient width. It has no text or appearance editor and no Auto-size action. Icons use the bundled, attributed icon catalog.

Street map starts at 252×222 and stretches independently on both axes. Its original street artwork has transparent roads in Clean style and white roads in Sketch style. Modal overlay starts at 318×240, uses black at 40% opacity, and offers a link covering its clipped area. Its color and opacity are fixed.

Vertical rule starts at 10×100. Only its height changes. It offers Solid, Dashed and Dotted strokes, color and opacity. Auto-size restores its automatic width while retaining height. The Sketch solid stroke is original tapered artwork; Clean uses a thin straight stroke.

Editing clamps each adjustable dimension to ten units. Loading and selecting smaller saved controls preserves their dimensions. Fixed axes render at their specified size while preserving stored values. Resizing, property edits, Undo/Redo, storage and component-local overrides use the shared editor behavior.

The gallery generator is `examples/render_static_layouts.rs`. Geometry, clipping and history have automated coverage. Native acceptance and remaining limitations are recorded in SESSION-SPEC.md. Exact contours, font metrics, antialiasing and native Windows/Linux acceptance remain open.
