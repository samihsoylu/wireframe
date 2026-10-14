# Checkbox and radio groups

Add a Checkbox group or Radio group from the library or Quick Add. Each source line is a row. Checkbox markers use brackets and radio markers use parentheses:

| State | Checkbox | Radio |
|---|---|---|
|Empty|`[ ] Option`|`( ) Option`|
|Selected|`[x] Option`|`(o) Option`|
|Mixed|`[-] Option`|`(-) Option`|
|Disabled|`-[ ] Option-`|`-( ) Option-`|

Selected markers also accept `o O v V x X *`; mixed markers accept `- =`. Empty markers accept zero or one space. Leading whitespace prevents recognition. Rows without the group's own marker use ordinary rich text. A literal `\r` inside a row adds a soft line break while keeping one row destination.

The inspector's Row links section assigns a destination to each row. Disabled appearance does not disable an explicit destination. Only complete visible rows render and receive link hit regions. Auto-size restores the full measured width and height; fixed sizes preserve deliberate clipping. Row edits remap links by existing source-row identity, with positional fallback for equal row counts.

A color wrapper around a marker changes its symbol; a wrapper around marker and label also formats the label. Disabled rows use gray for ordinary ink and symbols, while explicit inline label colors remain intact. Original source stays editable and is stored without rewriting its markers.

Parsing, measurement and painting are shared across canvas, thumbnails, presentation, PNG and SVG. Local component overrides, reset and Undo/Redo use the ordinary document operations. `cargo run --example render_choice_groups -- /path/to/fresh-directory` creates an editable twelve-case gallery and both style exports.

Verification covers marker states, opposite-kind/plain/blank rows, soft breaks, empty labels, clipping boundaries, visible links, source edits, history, components and storage. Native macOS acceptance covers insertion, editing, sizing, explicit row links, presentation, export and full-process reopen. Exact symbol contours and font metrics remain visual work. Windows/Linux runtime, native component-local group edits and all formatting corners are not accepted yet.
