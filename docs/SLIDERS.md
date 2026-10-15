# Sliders

Horizontal slider and Vertical slider are illustrative controls for levels and other bounded values. They start at 33×12 and 12×33. Only the long axis resizes; thickness stays 12. The canvas exposes two end handles, and Shift does not stretch the fixed axis. Resizing a single control has a minimum length of 10. Smaller saved dimensions remain readable and render without normalization.

The inspector provides Pointy, Rectangle and Circle thumbs, a value from 0 to 1, Normal/Disabled state and a color. Value starts at 0.05 and moves left-to-right or top-to-bottom. Color applies to the leading segment. The remaining track and thumb use the style's default ink. Disabled state renders all parts gray while retaining the stored color.

Thumb travel depends on its shape and style; the thumb remains 12×12. All artwork is clipped to the control and parent crop, including tiny controls. These controls illustrate a value; dragging them on the canvas moves the mockup element rather than changing a system setting.

The catalog, inspector, canvas, thumbnails, library previews and PNG/SVG exports share the same layout and independently drawn vector artwork. Properties participate in Undo/Redo, save/reopen and sparse component overrides. The example `examples/render_sliders.rs` creates 29 controls covering shapes, values, disabled states and narrow lengths.

Acceptance evidence and remaining visual/platform limitations are recorded in SESSION-SPEC.md. Exact skin contours and native Windows/Linux execution remain open.
