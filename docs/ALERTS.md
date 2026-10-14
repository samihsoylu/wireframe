# Alert dialogs

Add an Alert dialog from Quick Add or the library. Its text contains three lines: title, message and comma-separated button labels. Only the first two buttons are displayed. Extra lines and button labels remain in the saved source but are not displayed. A missing line and a blank line are distinct: a blank third line still creates one empty button. Commas always separate buttons; a backslash does not escape them.

The title is two points larger than the base text. Inline formatting and soft line breaks (`\r`) work in all fields. The message wraps; title and buttons truncate horizontally. Automatic width uses the title and buttons, with a 200-point minimum that grows with font size. Automatic height includes the wrapped message. Fixed dimensions can hide the message or buttons when space is insufficient. Auto-size and manual dimensions participate in Undo/Redo.

Choose **Divided** for centered buttons with separators or **Inline** for right-aligned underlined buttons. Inline buttons default to blue. Button color affects buttons only; resetting it restores the style-dependent default. Global underline applies to the title and message. Divided buttons use their own inline formatting; inline buttons always underline. Border can be solid or absent; both retain a white rounded background.

Use **Edit button links** to assign page, URL or Back destinations. Links follow button labels through source edits, including a single edit session that temporarily clears text. Clipping never transfers a hidden button's destination to a visible button. Formatted URLs retain their appearance without adding navigation inside a dialog. Use Present to activate button links.

Dialogs work in component sources and local instance overrides. Saved documents retain source text, dimensions, properties and destinations. Canvas, PNG and SVG use the same renderer. Generate the editable example with `cargo run --example render_alerts -- <output-directory>`.

Eight focused regression tests and native macOS checks cover source editing, button styles, destinations, clipping, automatic sizing, history, PNG/SVG export and restart persistence. Component overrides are covered by automated tests. Exact font rendering and sketch contours, native component editing and native Windows/Linux acceptance remain open.
