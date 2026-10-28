# App bars

App bars combine a title with optional navigation, filter and menu icons, an ordered collection of custom icons, and a background fill. The default title size is 16. Automatic sizing accommodates the title and configured icons with a minimum of 320×44. Manual dimensions preserve the title's measured extent and clip the result at the control boundary.

The inspector supports licensed catalog icons and imported project images. Add or remove icons with the picker, or move them left and right. Icon destinations follow matching icons in occurrence order; repeated identical icons retain their destinations from first occurrence to last. Navigation, filter and menu destinations remain independent of the custom collection. Hiding an icon keeps its destination stored. Editing the title does not change icons or their links.

Title and icon ink switch between black and white according to the background brightness. Inline text formatting remains available. Global bold, italic and underline fields are not offered for this control. Explicit destinations belong to icon hit regions; inline URL markup in the title does not create navigation.

The canvas, thumbnails, presentation, PNG and SVG exports use the shared renderer. Component-local collection edits and resets preserve the component source. Image import, sizing and collection edits participate in undo/redo, and imported images are embedded in the project.

Generate an editable example with `cargo run --example render_app_bars -- <output-directory>`. Focused checks are in `tests/app_bars.rs`.

Automated coverage establishes layout boundaries, source-slot navigation, duplicate remapping, component overrides, atomic invalid edits, image import/history and storage round trips. Native macOS checks cover insertion, catalog search, image import, collection movement/removal, duplicate destination retention, text edits, automatic-size undo/redo and presentation in both styles. Native PNG/SVG exports match generated renders byte for byte; saved manual and gallery projects resave unchanged after full process restarts.

A named Beta 2 (`ccca24b`) Mac ARM64 Clean check imports one local transparent image into two selected direct children of a component instance with unequal 3/1 icon collections. One embedded asset is shared at each eligible slot; source, other instance, local fills and destination roles remain exact. Eight complete saved comparisons cover chooser cancellation, one-step Undo/Redo, cancellation with pending Redo, presentation return and cold Save; four surviving links reach their expected destinations. Independent artifact/visual review passes. Both sessions exit normally and all 546original data/settings files and50 bundle entries/signature are exact. Private `appearance-finishing-current/delivery/shared-appbar-image-import/REPORT.md` records the case and excluded path-entry attempts.

Exact glyph contours, typography and rasterization remain broader visual work. Other component/duplicate/image-format/selection/style/input/platform variants retain their separately recorded limits; this named Mac case does not establish Windows/Linux acceptance. These checks establish their recorded behavior scopes, not complete visual identity or finished cross-platform support.
