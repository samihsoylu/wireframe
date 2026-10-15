# Buttons

Buttons retain their content, fixed or automatic dimensions, optional icons, alignment and page links across drawing styles.

Clean buttons have a one-unit outline with a 3.5-unit outer corner radius and a 2.5-unit inner radius. Both radii shrink to fit small saved dimensions. A button narrower or shorter than two units has only its outer surface. Corners reveal the artwork beneath the button; there is no rectangular background behind the rounded shape.

The normal outline is #333333, the disabled outline is #CCCCCC, and focus uses #97BDF0. Selected buttons use #97BDF0 for their fill. Custom fill and readable text contrast remain independent of corner geometry. Sketch buttons keep their existing outline and offset shadow.

The shared scene supplies canvas, PNG, SVG and PDF geometry. Form-state raster tests check colors and stable text placement. Corner tests check underlying artwork, unchanged dimensions and tiny saved controls in raster/PDF output. Realistic landing, desktop contact, phone contact and tablet order fixtures exercise complete compositions and navigation. This coverage does not establish exact typography, every clipped/grouped state combination or native Windows/Linux acceptance.
