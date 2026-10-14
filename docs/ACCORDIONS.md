# Accordion panes

Use one heading per line. Prefix a child with `- ` or a tab. Children belong to the preceding heading; nesting is one level deep. Blank rows are retained. An empty source uses the default panel labels. A child in the first row produces an explanatory label.

Selection controls which heading expands. Selecting a child highlights both that child and its heading, while showing its siblings. Choosing None collapses all headings. Automatic width includes hidden labels; automatic height follows visible rows. Fixed dimensions can truncate labels and omit rows. Remaining headings align near the bottom when space permits.

The scrollbar is a diagram element. Its position changes the thumb without scrolling the content. Each visible row can own a destination; the scrollbar strip and empty expanded area are excluded. Inline link formatting does not create an additional destination. Repeated label occurrences retain their destinations through source edits. Selection follows the same occurrence, or retains its slot for an equal-length replacement.

The parser, layout and renderer share the document, component overrides, Undo/Redo and export paths. Automatic sizing is restored by Undo after a manual resize.

Generate an editable example and both PNG/SVG styles with `cargo run --locked --example render_accordions -- /absolute/output/directory`.

Seven focused tests and the full 245-test suite pass; all-target Clippy passes with warnings denied. Native macOS acceptance covers Quick Add, source and selection editing, row destination remapping, scrollbar settings, fixed dimensions, Auto-size, Undo/Redo, both drawing styles, presentation navigation and excluded areas. Native clean/sketch PNG/SVG exports match generated exports byte for byte. The gallery and a manually edited project resave unchanged after a full process restart.

Exact typography, sketch contours and rasterization remain fidelity work. Component overrides are covered by automated tests; native component editing and Windows/Linux runtime acceptance are not claimed.
