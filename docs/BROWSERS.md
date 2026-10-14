# Browser frames

A Browser frame provides a title, address row and content background for website wireframes. New frames start at 450 × 400. Resize them directly or with the inspector; editing their content preserves their dimensions.

Double-click to edit the title and address together. The first line is the title; the rest is the address. An empty source stays empty. Rich text and soft line breaks are supported. The `{mockup-name}` placeholder displays the current page name as literal text. Title and address markup does not create clickable inline destinations. The control's own page destination remains available.

The inspector offers a content color, square or torn bottom border, and an optional vertical scrollbar with a position value. These are visual wireframe controls, not a functioning web browser or scrolling viewport. Clean and Sketch styles use different original chrome drawings and content insets. Text offsets remain fixed when small frames compress the chrome. Address text and scrollbar thumbs disappear below their respective size thresholds.

The torn edge clips the entire frame, including text and scrollbar. Its cutouts remain transparent, revealing controls below. Canvas meshes and SVG clipping use the same boundary; PNG exports use the shared SVG scene. Fills with nonpositive dimensions are omitted for tiny frames.

Property changes participate in undo/redo. Component-local changes and resets preserve the component source. Existing project values, including older border choices, remain readable. The two offered border choices are square and torn bottom; older border values keep their prior square appearance.

Generate an editable example with `cargo run --example render_browsers -- <output-directory>`. Focused coverage in `tests/browsers.rs` includes source splitting, page-name escaping, layout thresholds, history, component overrides, serialization, tiny-frame exports, transparent cutouts, and canvas mesh texture/order preservation.

Native macOS checks cover Quick Add, multiline source editing, fixed dimensions, resize Undo/Redo, border/fill/scrollbar properties, both styles, transparent overlaps and presentation. Four native PNG/SVG exports from the final build equal shared renders byte for byte. A full process restart and resave preserves the manually edited project bytes; the neutral gallery also resaves unchanged.

Exact typography, sketch contours and rasterization remain ongoing visual work. Native component editing and Windows/Linux execution are not established by this pass. The seven focused tests include a pixel regression for scrollbar visibility at the small-height boundary.
