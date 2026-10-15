# Control outlines

Button, Text Input and Text Area use stable original vector outlines in Sketch style. Rectangle supports Square, Rounded, Dashed and No border, with independent border color, fill and fill opacity. The existing text, font size and bold fields remain available for stored rectangle captions.

Sketch contours keep their corner regions at a fixed size as the center stretches. When a frame is too small for opposing corners, those regions compress proportionally. There is no redraw-time randomness. Button has heavier lower and right edges; input and area use a lighter frame. Existing Clean form geometry and intrinsic input height remain unchanged.

Borders are connected vector surfaces with transparent interiors. Dashed rectangles use a continuous eight-unit dash and eight-unit gap around rounded corners, with three-unit Sketch and one-unit Clean strokes. Fill opacity affects only the fill, so a fully transparent rectangle retains its border. All drawing is clipped to the control and its inherited clip. Canvas, PNG, SVG and PDF consume the same scene geometry.

Rectangle border values are `solid`, `rounded`, `dashed` and `none`. The legacy `dotted` value remains readable as a dashed border but is not offered as a duplicate choice. Existing `shape` values for rounded and dotted rectangles remain recognized. No document migration is required. Local component overrides, ordinary document history and storage use the existing typed property path.

## Acceptance and limits

The size matrix covers six outline variants at five sizes in both styles, including frames smaller than their normal corners. Additional checks cover 0%, 50% and 100% fill, custom border colors, form states, component overrides, exact Undo/Redo and storage round trips. Native macOS checks cover inspector choices, resize/Undo, fill opacity, RGB editing, style switching, Save and process restart/reopen.

The integrated original scenarios, table order variant and table options remain acceptance fixtures. All ten Clean images and two unaffected Sketch images remain identical; changes in the other eight Sketch images stay inside the affected controls. Four exported PDF pages were visually inspected and retain searchable text.

Exact Sketch contour curvature and ink distribution remain approximation limits. In particular, the Button's size-matrix pixel comparison is worse than the previous straight outline despite its improved resize stability. Button ink distribution and rounded Rectangle curvature need a focused fidelity follow-up; functional acceptance does not close those discrepancies. This acceptance does not establish all catalog shapes, broad nested component combinations, native Windows/Linux behavior or complete typography fidelity. See SESSION-SPEC.md for the current full-suite result and remaining scope.
