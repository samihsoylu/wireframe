# Braces and markup strokes

Horizontal brace, Vertical brace, Red cross and Scratch out are available in the Markup library and Quick Add. All four start as markup and resize independently in either direction. Editing enforces a ten-unit minimum; smaller saved dimensions remain intact and render within their frame. These controls do not offer Auto-size or a whole-control page link.

| Control | Initial size | Editable content |
| --- | --- | --- |
| Horizontal brace | 200 × 80 | Multiline rich text; top or bottom direction |
| Vertical brace | 180 × 140 | Multiline rich text; left or right direction |
| Red cross | 240 × 104 | Geometry only |
| Scratch out | 205 × 107 | Color and opacity |

Brace artwork occupies 15 units across its short axis, with a 20-unit text offset. Direction determines text alignment. Text appears only when both available text dimensions are positive. Font size, bold, italic, underline, text color, inline links and page-name expansion use the shared text engine. Changing text does not resize a brace. Text color also colors its brace.

Red cross uses fixed colors: `#771822` in Sketch and `#cf2b28` in Clean. Scratch out defaults to black in Sketch and `#333333` in Clean; its opacity ranges from zero to one. The static marks do not open a text editor.

The artwork is original procedural geometry. Scratch out uses one continuous ribbon with non-overlapping turns, preserving uniform transparency. Canvas, PNG and SVG consume the same drawing data. Original contours, text metrics and rasterization remain fidelity work.

`examples/render_markup.rs` generates a 40-control gallery in both styles, including narrow, tiny, reversed, empty, formatted and translucent examples. It also accepts neutral JSON layout cases with `--geometry`. `tests/markup.rs` checks placement, strict text thresholds, history, component-local dimensions, storage, formatting, clipping and mesh validity. Shared app tests cover inspector support, resize minimums, tiny selection and keyboard editing eligibility.
