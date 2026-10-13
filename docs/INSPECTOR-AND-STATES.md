# Typed inspector and choice states

For the eleven basic text controls and tables using the shared renderer, the inspector reads catalog-effective values from canonical recursive nodes. It exposes implemented, catalog-visible text styles, colors, alignment, choice/disabled states and table header/row/selection/line options. Other controls retain their initial inspector. Unimplemented button/input state rendering and specialized properties remain deferred; inventory alone does not make them usable.

Edits use typed validation and sparse storage. Reset removes a difference from the current project default, including its inherited font size. Unknown values are preserved. Locked controls reject edits. Nested editing uses sibling-scoped IDs in the current group. Continuous edits participate in existing transactions; discrete state changes and resets can be undone/redone. Auto-size restores the catalog's applicable automatic dimensions and preflights layout; unsupported layout restores the prior dimensions.

Checkboxes and radio buttons support Normal, Selected, Disabled, Selected and disabled, Indeterminate, and Indeterminate and disabled. Disabled ink is applied to the mark, outline, label and default inline-link color. Explicit rich-text color tags retain their formatting behavior. Choice labels start 20 units after the control origin; checkbox/radio symbols are 16/13 units. Requested state does not change automatic geometry. The implementation uses original vector geometry; exact icon contours and typography remain acceptance work.

New checkbox/radio insertion creates a catalog node with automatic dimensions and inherited properties, instead of an initial fixed prototype rectangle. Older saved projects preserve their explicit geometry. New state edits clear legacy checked flags, and an explicit disabled-selected state is correctly projected as selected. Other insertion families still use the initial migration-backed path.

Native macOS checks: insert a checkbox; select mixed disabled; undo/redo; Reset and undo reset; save; close/reopen; verify persisted state; select disabled-selected and verify menu dismissal; undo to the saved state. The saved file retains automatic dimensions and one explicit state property. Product checkbox geometry is 73.6875×25 with current fonts. The native inspector rounds width to 73.69. This is a measured checkpoint, not final typography.

Five property/state tests cover sparse legacy transitions, unknown-field retention, storage round trips, undo/reset, nested contexts, locks, invalid states, project-font inheritance, auto-size undo/rollback, all twelve checkbox/radio state combinations, symbol dimensions and disabled link colors. Combined with existing checks, 68 tests pass and Clippy is clean with warnings denied. A reproducible editable gallery and PNG/SVG are generated with examples/render_choices.rs.

Remaining: complete inspector/rendering coverage, exact glyph metrics and contours, clean/sketch style rules, icons/rotation, group/component override inspectors, richer link targets, multi-selection property editing and native Windows/Linux checks. This is developer verification, not independent review or complete product acceptance.

## Basic form controls

Button now exposes Normal, Selected, Focused and Disabled; text input and text area expose Normal, Focused and Disabled. The shared scene renders selected button fill, focused borders, disabled borders/labels, and readable button text on dark fills. Clean buttons default to gray; explicit fill overrides and Reset use the same drawing-style context as layout. Font and padding rules determine automatic sizes. New empty inputs start at 79×27 in sketch style or 79×24 in clean style; text areas start at 200×140. Existing explicit text and dimensions remain intact. The input catalog default is now empty instead of the initial placeholder.

The generated two-style gallery, native state-change/Undo and drawing-style checks passed. Three additional regression tests cover inserted families/default sizes, fill Reset, automatic font-height changes, round-trip geometry, all ten form-state combinations in both styles, stable text placement, and contrast. The complete suite has 77 passing tests. Native reference equivalence is not established by those tests.

Remaining form work includes menu icons, input border style/color/opacity, text-area scrollbars and inherited text contrast on filled inputs. Native clean-font selection and original sketch contours remain incomplete. Library previews currently use older drawing code.
