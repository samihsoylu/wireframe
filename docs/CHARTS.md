# Charts

Column, bar, line and pie charts provide static sample graphics for layouts. They are available in the Charts library and Quick Add. The controls have independent dimensions, position and layering, with no data or text editor. Column, bar and line start at 187×175; pie starts at 180×180.

Bars retain absolute length offsets while their positions and thickness scale with the other axis. Their fill and original outline are shared by the canvas, thumbnails and exports. Line charts draw two fixed series before the axes. Sketch series strokes grow from four to eight units; Clean strokes remain two units. Pie charts remain circular and centered within rectangular bounds, with fixed 150- and 210-degree sectors.

Editing dimensions enforces a ten-unit minimum. Smaller loaded geometry remains unchanged by selection and is clipped to its saved frame. Resizing participates in Undo/Redo, persistence and component-local overrides. Original chart artwork is procedural; exact contour and antialiasing equivalence is not claimed.

The example `render_charts` generates both styles and accepts `--geometry` for diagnostic layout output. Native platform and broader application acceptance remain tracked in SESSION-SPEC.md.
