# Field sets

A Field set provides a labeled frame for related form fields. New frames begin at 200×170 and grow in width for longer labels. Once width or height is set manually, that dimension stays fixed as the label or font changes. The inspector provides label text, font styles, fill color and fill opacity.

Rich text and soft line breaks (`\r`) are supported. Macro-like text stays literal. The label sits in a transparent cutout in the border; an empty label retains a small gap. Long labels clip at the frame boundary. The fill covers the whole frame, including the strip above the border. Fill opacity leaves the border and label opaque. Dark fills use light text when fill opacity is at least 75%.

Canvas, library previews, page thumbnails and PNG/SVG exports use the same layout and drawing. Component overrides are local and resettable. Text, dimensions and appearance use normal document history and storage. Typing a fractional opacity is one undoable edit; slider dragging previews live until release. The frame is a drawing control; grouping and component commands organize contained controls.

See `examples/render_field_sets.rs` for a gallery with empty, narrow, formatted, dark, translucent and overlapping examples. Tests cover geometry, opacity/contrast boundaries, clipping, literal source, history/storage, component reset and pixel-level transparency. Exact typography and sketch contours remain part of broader visual fidelity work.
