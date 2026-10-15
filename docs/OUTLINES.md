# Control outlines

Button, Text Input and Text Area use stable original vector outlines in Sketch style. Rectangle supports Square, Rounded, Dashed and No border, with independent border color, fill and fill opacity. The existing text, font size and bold fields remain available for stored rectangle captions.

Sketch contours keep their corner regions at a fixed size as the center stretches. When a frame is too small for opposing corners, those regions compress proportionally. There is no redraw-time randomness. Button has a foreground frame with connected shadow bands that step beyond its lower and right edges; input and area use a lighter frame. Rounded Rectangle curves extend into the center regions, rather than ending at the fixed corner boundary. Existing Clean form geometry and intrinsic input height remain unchanged.

Borders are connected vector surfaces with transparent interiors. Solid border quads choose their interior diagonal after corner compression, avoiding folded triangles in tiny frames. Dashed rectangles use a continuous eight-unit dash and eight-unit gap around rounded corners, with three-unit Sketch and one-unit Clean strokes. Fill opacity affects only the fill, so a fully transparent rectangle retains its border. Rounded fills use a shared triangle fan, retaining coverage when compressed boundaries repeat or become non-convex. This avoids missing small fills and canvas tessellation seams. All drawing is clipped to the control and its inherited clip. Canvas, PNG, SVG and PDF consume the same scene geometry.

Rectangle border values are `solid`, `rounded`, `dashed` and `none`. The legacy `dotted` value remains readable as a dashed border but is not offered as a duplicate choice. Existing `shape` values for rounded and dotted rectangles remain recognized. No document migration is required. Local component overrides, ordinary document history and storage use the existing typed property path.

## Acceptance and limits

The size matrix covers six outline variants at five sizes in both styles, including frames smaller than their normal corners. Additional checks cover 0%, 50% and 100% fill, custom border colors, form states, component overrides, exact Undo/Redo and storage round trips. Native macOS checks cover inspector choices, resize/Undo, fill opacity, RGB editing, style switching, Save and process restart/reopen.

The integrated original scenarios, table order variant and table options remain acceptance fixtures. All ten Clean images and two unaffected Sketch images remain identical; changes in the other eight Sketch images stay inside the affected controls. Four exported PDF pages were visually inspected and retain searchable text.

Exact Sketch contour curvature, ink distribution and antialiasing remain approximation limits. The refined Button and rounded Rectangle profiles improve all 18 tested size comparisons, including eight cases at sizes not used to choose their parameters. The other 50 style/size crops are identical. This does not establish every catalog shape, broad nested component combinations, native Windows/Linux behavior or complete typography fidelity.

Rendering fixtures assign exact stored dimensions. The earlier fixture's interactive resize helper had clamped 8×8 to 10×10, invalidating twelve tiny comparisons; those claims are corrected. Text Input fixtures use auto height to express the interactive intrinsic-height contract. Explicit fixed Text Input heights remain a separate loading-contract question.

The refinement passes the complete 601-test suite, native resize/history/opacity/style/reopen checks and shared export checks. A canvas tessellation regression test verifies 135 interior sample points at five sizes and three zooms. See SESSION-SPEC.md for the current checkpoint.
