# Mobile keyboards

Mobile keyboard is a decorative control with Phone and Tablet layouts, each in portrait and landscape. Phone portrait starts at 226×160, phone landscape at 403×114, tablet portrait at 546×188 and tablet landscape at 728×251. Variant changes update automatic dimensions. Explicit width and height remain unchanged until Auto-size is used.

The complete keyboard scales proportionally and centers inside its bounds. Its light gray background covers the fitted rectangle, leaving surrounding space transparent. Font sizes, icon sizes, gaps and key borders scale with the layout. Phone has 33 keys and Tablet has 38, including stacked punctuation and a dismiss-keyboard symbol. Keys are illustrations; clicking them selects the control rather than entering text.

The geometry module is independent of the original rounded key contours. Sketch uses heavier, slightly irregular outlines; Clean uses lighter outlines. Globe and microphone artwork use the existing licensed offline icon catalog. The remaining functional symbols are original vectors. Canvas, previews and SVG/PNG export share the same rendering pipeline. The control keeps the standard exterior alignment and spacing targets.

Device and orientation are typed properties with history, reset and component overrides. Existing documents retain explicit dimensions. No external assets or network access are needed. `examples/render_mobile_keyboards.rs` generates an editable eight-control gallery and both drawing styles.

Focused tests cover automatic and fixed sizes, Undo/Redo, storage, component overrides, proportional fitting, row content, finite tiny geometry, available icons and transparent letterboxing. Font shapes and exact contour appearance remain visual fidelity work; native checks on additional operating systems remain required for release acceptance.
