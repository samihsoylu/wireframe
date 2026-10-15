# Compact form controls

Numeric stepper, Color picker and Help button use the library, Quick Add, typed inspector and shared canvas/PNG/SVG renderer.

The numeric stepper starts with `3`. Its automatic width follows measured text plus 33; height follows measured text plus 11. Width can be set explicitly, with a ten-unit editing minimum. Smaller saved widths remain intact. Horizontal handles resize it independently of height, including with Shift held. Blank text stays blank; rich formatting and explicit line breaks are supported. Page-name tokens stay literal. The arrows are wireframe artwork, and formatted links inside this field do not create navigation targets.

Normal stepper text and border use the chosen border color. Disabled text is gray; the Clean disabled border is lighter. Clean style uses a gray arrow background; Sketch uses white. The original frame artwork retains corner dimensions while its central regions stretch, and small frames proportionally compress the corners. All artwork and text clip to the field.

Color picker is fixed at 26×28 and exposes only its color. Help button is fixed at 18×18 and exposes its link. Neither offers text editing, resizing or Auto-size. Saved dimensions do not change their visible size; original serialized dimensions remain preserved. Their original artwork follows the style setting. The help link uses the full clipped control area.

History, project storage, component-local overrides, native editing and shared exports are checked in the project acceptance record. Exact glyph metrics and artwork contours remain fidelity work; Windows/Linux native acceptance is still open.
