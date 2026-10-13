# Catalog and shared layout checkpoint

The catalog defines 86 insertable control families with stable product IDs, names, search aliases, categories, default requested sizes, automatic sizing policy and resize axes. Groups and component instances are structural nodes rather than library entries. This inventory is not a claim that all 86 families are rendered or available in the desktop interface.

## Property rules

The implemented common property set has typed values, context defaults, enum/range checks and per-control inspector visibility. Choice controls distinguish six checkbox/radio states from four button states; input and dropdown states differ. Shape variants are separate from control identity. Geometry remains in the node rather than an untyped property map.

Ordinary edits store differences from current defaults. Component patches compare against inherited effective values, so an override equal to a global default can still differ from its source. Reset removes the explicit value. Unknown stored fields survive decoding; unsupported new edits fail. Legacy checked flags affect only checkbox, radio and switch states. Existing ellipse/line/panel IDs remain recognized.

Known property type/state errors are now rejected by document validation before saving. The schema is still incomplete for specialized device, icon, chart, menu and other control-specific fields. Common fields retained for legacy compatibility can be hidden from the inspector. Completing the remaining field contracts is part of per-family implementation, not a completed acceptance claim.

## Layout

The layout engine expands components and resolves sparse properties without mutating project data. It keeps requested dimensions, uncropped bounds, clipped bounds and inherited clipping separate. Ancestor positions, locks and markup propagate through nested groups. Migrated explicit paint order remains stable. Expansion has one shared budget across the whole page, preventing many individually valid instances from multiplying work without a limit.

Text measurement uses the canvas font system's glyph advances and wrapping through a narrow TextMeasurer interface. The app supplies bundled canvas font faces; geometry tests and native specimens do not establish final typography. Rich text, basic buttons, choices, inputs, simple shapes and structured table cells have initial sizing logic. See RICH-TEXT.md. Image intrinsic dimensions/rotation, most specialized automatic layouts and broader native visual comparisons remain outstanding.

Unsupported automatic layout returns an explicit error. For unsupported controls with fixed dimensions, layout retains those dimensions and records a warning. Neither behavior counts as an implemented renderer. The desktop interface now edits the recursive project and uses derived control views and resolved properties for the shared scene. See RECURSIVE-EDITOR.md.

## Verification

68 tests pass across the app, legacy workflows, recursive document/workspace, catalog/layout and text. New cases cover IDs/defaults and migration coverage, search ranking, sparse edits versus inherited overrides, state distinctions, hidden inspector fields, failed schema validation, per-axis resize policy, real glyph widths/wrapping, nested override layout and clipping, and explicit unsupported-layout outcomes. Clippy passes with warnings denied. Five saved native test documents still migrate read-only with their source bytes unchanged.

The shared text/table scene is integrated; continue control-specific measurement/rendering and visual comparisons. Representative native version-2 editing and save/reopen are verified; full visual fidelity, control coverage and Windows/Linux runtime acceptance remain outstanding.

Typed text/table properties and six-state choice rendering are integrated; see INSPECTOR-AND-STATES.md for the current field boundary and native evidence.
