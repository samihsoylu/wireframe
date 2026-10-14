# Mobile menus and pickers

Add **Mobile menu** or **Mobile picker** through Quick Add or the library. Both use editable multiline text and work in components, saved projects, the canvas, thumbnails and PNG/SVG exports.

## Mobile menu

Each line creates a row, including empty lines. The first unescaped comma separates the main label from an optional right-hand caption. Write `\,` for a literal comma and `\r` for a line break within a label. Inline text formatting is supported.

| Prefix | Left side |
|---|---|
| `+ ` | Circled plus |
| `- ` | Circled minus |
| `v ` | Checkmark |
| `* ` | Bullet |
| `_ ` | Reserve a small icon space |
| `__ ` | Reserve a large picture space |

After a comma, a trailing ` >`, ` (>)`, ` =`, ` ON` or ` OFF` adds a chevron, circled chevron, drag bars or switch. Spaces and capitalization matter. For example, `Notifications, ON` draws a switch, while `Notifications,ON` displays the caption as text.

Width can be resized. Height follows the rows and their content. Auto-size restores a width based on the font size. Main-label bold, italic and underline are independent of the smaller right-hand caption. Keep long labels brief when combining a caption and a trailing icon, since they share available space. Hiding the border retains the white background and row dividers.

Use **Edit row links** to link any row, including a blank row, to a page, web address or navigation back. Moving or editing rows preserves destinations by matching labels and occurrence order, with positional retention when the row count stays the same.

## Mobile picker

The picker is a fixed 214×160 preview. Commas separate columns; newlines separate rows. Escaped commas and inline formatting work. It displays up to seven rows, emphasizes the fourth row and retains additional rows in the saved text. Empty rows keep their positions. A short list starts at the top. Wide columns and text are clipped to the control boundary; soft line breaks do not increase the 22-unit row pitch.

The picker is decorative. It does not implement a spinning wheel or per-row navigation. Text resembling a placeholder remains literal.

## Validation

Eight focused regressions cover parsing, row geometry, sizing/history/storage, local component edits, links, caption styles, visible-row limits, clipping and filled marker drawing. The full suite reports 347 passing tests. Native macOS checks cover insertion, edits, menu Auto-size Undo/Redo, blank-row link editing and presentation navigation, fixed picker behavior, full restart/resave and both export formats/styles. Four native exports match shared-renderer output byte for byte.

The gallery example is `examples/render_mobile_rows.rs`. Exact font and contour matching, native component editing for these controls, advanced nested/rotated clipping, and native Windows/Linux checks remain open. This is self-review; no independent review is claimed.
