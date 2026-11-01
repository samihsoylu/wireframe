# Data grid property evidence

Source audit, 1 November 2026. All 16 visible Data grid properties have current consumers. The three hidden compatibility fields remain hidden. This is source-consumption evidence, not a new native or per-property fidelity pass. Q7 remains open.

| Property | Consumer |
| --- | --- |
| `text` | workspace/view.rs text projection; table.rs Grid::parse and CellContent text/choice rendering |
| `font_size` | workspace/view.rs font projection; table.rs TextStyle.size and grid layout |
| `bold` | workspace/view.rs bold projection; table.rs TextStyle.bold |
| `italic` | table.rs TextStyle.italic from effective properties |
| `underline` | table.rs paint_options; render/mod.rs PaintOptions.underline |
| `fill` | table.rs base rectangle and alternating row backgrounds |
| `align` | table.rs alignment fallback; structured column overrides and alignment offsets |
| `vertical_scrollbar` | table.rs reserved width and conditional vertical_scrollbar |
| `value` | table.rs scrollbar position parameter; requires vertical_scrollbar=true |
| `selected_index` | table.rs selected row background with header offset and real-row bound |
| `border` | table.rs bordered top/inset and conditional rectangle_frame |
| `header` | table.rs header metrics, gray row, selection offset and scrollbar start |
| `vertical_lines` | table.rs style-weighted column separator primitives |
| `horizontal_lines` | table.rs conditional row separator primitives |
| `row_height` | table.rs clamped16..80 grid metrics/filler rows; content can impose a larger height |
| `alternate_fill` | table.rs odd-row background colour |

The renderer reads effective layout properties. Text, font size and bold are projected into the legacy control by `src/workspace/view.rs`; the other fields are consumed directly or through shared paint/alignment helpers. Column markup can override the global alignment; selected rows exclude the header and empty filler. Scroll position needs an enabled scrollbar, and content may dominate the minimum row height. These supported contexts explain the earlier inactive default-fixture results.

Hidden `text_color` participates in the existing palette rules. Hidden `checked` and `link` are not new Data grid inspector controls; per-cell choices and destinations use the existing structured text syntax. No hidden field was exposed or removed.

Nine table geometry/options/palette functions passed in retained Linux CI [41154197685](https://github.com/samihsoylu/wireframe/actions/runs/41154197685), source `71306ba7`. All nine relevant renderer/layout/catalog/view/test files are byte-identical at the frozen Beta 4 source `fc1d98b`. This supports reuse of that bounded deterministic evidence; no test suite was repeated. The nine functions cover table geometry, choice/sort alignment, scrollbar position/header/frame variants, border removal, persistent/history/auto-size options and palette/disabled/storage behavior. They do not establish every native/property combination.

Earlier both-style catalog diagnostics and active-context checks retain their original `97e7a99` identity and input limits. Broader nesting/crop/native/input/platform cases remain open. Windows execution remains held. Detailed current source hashes and retained test names are in the private delivery record `table-property-consumption-v1/source-evidence.json`.
