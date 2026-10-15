# Typed inspector and choice states

For the eleven basic text controls and tables using the shared renderer, the inspector reads catalog-effective values from canonical recursive nodes. It exposes implemented, catalog-visible text styles, colors, alignment, choice/disabled states and table header/row/selection/line options. Other controls retain their initial inspector. Specialized properties beyond the documented states remain deferred; inventory alone does not make them usable.

Edits use typed validation and sparse storage. Reset removes a difference from the current project default, including its inherited font size. Unknown values are preserved. Locked controls reject edits. Nested editing uses sibling-scoped IDs in the current group. Continuous edits participate in existing transactions; discrete state changes and resets can be undone/redone. Auto-size restores the catalog's applicable automatic dimensions and preflights layout; unsupported layout restores the prior dimensions.

Checkboxes and radio buttons support Normal, Selected, Disabled, Selected and disabled, Indeterminate, and Indeterminate and disabled. Default symbol and label ink follows the style: black in sketch and dark gray in clean. Explicit control colors survive style changes. Disabled ink is applied to the mark, outline, label and default inline-link color. Explicit rich-text color tags retain their formatting behavior. Choice labels start 20 units after the control origin; checkbox/radio symbols are 16/13 units. Requested state does not change automatic geometry. The shared renderer uses the existing licensed outline icons and an original rounded radio-minus bar. Optical checkbox bounds are14×14 inside a16-unit icon box. Half-unit placement rounds toward the positive axis, including symbols taller than their row. Typography and small raster edge differences remain acceptance work.

New checkbox/radio insertion creates a catalog node with automatic dimensions and inherited properties, instead of an initial fixed prototype rectangle. Older saved projects preserve their explicit geometry. New state edits clear legacy checked flags, and an explicit disabled-selected state is correctly projected as selected. Buttons, text inputs, text areas, search boxes and dropdowns also use catalog insertion; the remaining families still use the initial migration-backed path.

Native macOS checks: insert a checkbox; select mixed disabled; undo/redo; Reset and undo reset; save; close/reopen; verify persisted state; select disabled-selected and verify menu dismissal; undo to the saved state. The saved file retains automatic dimensions and one explicit state property. Historical geometry at that checkpoint was 73.6875×25. The subsequent line-box correction produces 74×23 at 13 points; exact glyph widths remain an acceptance item.

Five property/state tests cover sparse legacy transitions, unknown-field retention, storage round trips, undo/reset, nested contexts, locks, invalid states, project-font inheritance, auto-size undo/rollback, all twelve checkbox/radio state combinations, symbol dimensions and disabled link colors. That checkpoint passed 68 tests with Clippy warnings denied. A reproducible editable gallery and PNG/SVG are generated with examples/render_choices.rs.

Remaining: complete inspector/rendering coverage, exact glyph metrics and contours, clean/sketch style rules, icons/rotation, group/component override inspectors, richer link targets, multi-selection property editing and native Windows/Linux checks. This is developer verification, not independent review or complete product acceptance.

## Basic form controls

Button now exposes Normal, Selected, Focused and Disabled; text input and text area expose Normal, Focused and Disabled. The shared scene renders selected button fill, focused borders, disabled borders/labels, and readable button text on dark fills. Clean buttons default to gray; explicit fill overrides and Reset use the same drawing-style context as layout. Font and padding rules determine automatic sizes. New empty inputs start at 79×27 in sketch style or 79×24 in clean style; text areas start at 200×140. Existing explicit text and dimensions remain intact. The input catalog default is now empty instead of the initial placeholder.

The generated two-style gallery, native state-change/Undo and drawing-style checks passed. Three additional regression tests cover inserted families/default sizes, fill Reset, automatic font-height changes, round-trip geometry, all ten form-state combinations in both styles, stable text placement, and contrast. The form-state checkpoint passed 77 tests. Native reference equivalence is not established by those tests.

Remaining form work includes menu icons and broader family coverage. Native clean-font selection and original sketch contours remain incomplete. Library previews now use the same insertion defaults and positioned scene as the canvas. An isolated temporary document avoids changing the active project. Scenes are cached by family and invalidated on drawing-style or font-default changes. Two regression tests cover all 24 library families in both styles and setting changes; the full suite passes 79 tests with Clippy warnings denied. Native macOS checks verified button and empty-input insertion, preview/canvas/thumbnail agreement and style switching. Evidence is local under evidence/library-previews/.

## Search controls

Search now exposes Normal/Focused/Disabled, Rounded corners/Rectangle, Search icon and Microphone icon. It uses automatic width at least120, measured text plus26, with height text plus12 in sketch style or13 in clean style. Default text is “search”; default text ink is gray128/128/139 in sketch and gray51 in clean. Existing explicit text/color/dimensions remain intact.

The label starts22 units from the left with a search icon or6 without. Changing icon toggles does not affect automatic size. The12-unit microphone appears only when width minus label origin minus measured text width is greater than18. Disabled labels/icons/borders use disabled ink; focused borders use the style's selection color. White inset backgrounds, independent vector icons and border outlines are shared with exports; border draws last. These vector approximations still need contour acceptance.

Two search tests verify both styles, all states/shapes, stable geometry, icon changes, exact microphone threshold, Undo/Redo and encode/decode. Combined suite:81 passing tests; Clippy all targets with warnings denied passes. Native macOS gallery/shape/icon/Undo/style checks passed; the app is left on its saved sketch gallery. Windows/Linux and independent review are unperformed. Reproduce galleries with `cargo run --locked --example render_search -- evidence/search-controls`.

## Dropdown menus

Dropdown exposes Normal/Focused/Disabled and their closed variants, border/text colors and label-based Selection. The first text line supplies the initial title; subsequent lines define the menu. Blank/space-only rows and lines containing only hyphens or equals signs are separators. Initial `x ` and `o ` mark rows; a trailing space followed by `>` characters adds a submenu arrow. Commas remain literal. Blank rows are excluded from selection indexing; the selected title removes its markers. Original text survives all edits and storage.

Automatic width uses the selected title plus30; header height is text height plus14 sketch or11 clean. Open menu rows add text height plus3, separators5/1 and bottom padding10/9. Source parsing is bounded to10,000 menu rows. Explicit old dimensions remain unchanged. Title/menu text use fitting prefixes, and visible selection uses mapped source rows.

Canvas, library, thumbnails and SVG/PNG share original vector rendering. Native macOS checks verified label selection and Undo, collapse106→27 and Undo, clean expanded height98, and corrected filled arrow triangles. Two regression tests cover parsing/index mapping, six-state geometry/colors, undo/storage and resource bounds.83 tests pass; Clippy all targets with warnings denied passes. Gallery: `cargo run --locked --example render_dropdown -- evidence/dropdown-controls`.

Remaining: custom title icon, precise skin/font appearance and further rich-text edge comparisons. No independent review or Windows/Linux native acceptance is claimed.

## Embedded scrollbars

Dropdowns and text areas expose Scrollbar (default off) and Scroll position (0–1, default0), shown only when enabled. They share a vector renderer across canvas, thumbnails and exports. A fixed8×21 thumb appears only when track height exceeds45. Its Y position is track top+13+(height−48)×value. Position depicts scroll state without moving the content. Clean track ink stays #515151 in normal, focused and disabled fields. Sketch uses black normally and #80808b when disabled.

Text areas reserve16 units from wrapping width. Their16-unit track is inset2 on top/right/bottom in sketch style and0 in clean style. Dropdowns overlay a20-unit right strip beneath the header, ending2 above the bottom, with no change to menu sizing. Closed dropdowns and those with no menu rows omit the track. Covered inline link regions are removed.

Regression tests cover both families/styles, endpoint/midpoint values, the 45/46 height boundary, state palettes, stable dimensions/content, wrapping and link bounds, Undo/Redo and storage round trips. Native macOS checks cover inspector position changes and Undo for both families, dropdown toggle hiding the slider, and Clean/Sketch switching. SESSION-SPEC.md records current suite and bundle validation. Generate editable fixtures and SVG/PNG with `cargo run --locked --example render_scrollbars -- evidence/scrollbar-controls`. Standalone and embedded Clean bars now share the artwork described in LINEAR-CONTROLS.md. Exact Sketch contours remain incomplete; Windows/Linux native checks and independent review remain unperformed.

## Input appearance

Text input supports Rectangle and Underline borders. Both text input and text area expose Border color and Fill opacity. Normal borders use the chosen color; Focused/Disabled override it. Opacity affects the background primitive only and composites over lower controls consistently in canvas and SVG/PNG. Text, border and scrollbar ink remain opaque. Text-area borders are drawn after text.

Inherited text/link ink turns white when opacity is at least0.75 and weighted RGB brightness is below153. Otherwise the inherited text uses the drawing style's ink and links use8/83/148. Disabled text remains128/128/139. Explicit text-color edits are retained even if equal to the current default, so later fill changes preserve their intent; Reset removes the override and restores automatic contrast. Other properties retain their established sparse defaults. Rich inline color formatting retains its precedence.

Underline style removes the input label's left inset; automatic control dimensions are unchanged. The line is an original vector approximation; precise contour/font matching remains open. Input inspector state and Reset continue to use canonical recursive nodes and Undo transactions.

Three regression tests cover both styles/families,0.749/0.75 opacity and152/153 brightness boundaries, explicit color/reset/Undo/Redo/storage, state border colors, stable dimensions, textarea drawing order and alpha compositing against underlying red content in raster output. The suite passes88 tests and Clippy with warnings denied. Native macOS verified opacity1→0.5/Undo, Rectangle→Underline/Undo, and clean style27→24/Undo; the saved sketch gallery is restored. Generate both styles with `cargo run --locked --example render_input_appearance -- evidence/input-appearance`. No independent review or Windows/Linux native verification is claimed.


List measurement, typed fields, selection-preserving text edits and remaining limitations are documented in [LISTS.md](LISTS.md).
