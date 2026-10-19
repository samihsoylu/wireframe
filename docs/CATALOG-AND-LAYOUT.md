# Catalog and shared layout checkpoint

The catalog defines 86 insertable control families with stable product IDs, names, search aliases, categories, default requested sizes, automatic sizing policy and resize axes. Groups and component instances are structural nodes rather than library entries. All 86 families have default render coverage; availability and meaningful property variants have separate interface and family contracts.

## Current release audit

All 86 schemas validate default nodes. The current shared pipeline renders all 172 Sketch/Clean default variants to SVG and to 344 PNGs at standard/double resolution, with no layout or export failures and no document mutation. All eight fitted overview sheets were visually inspected. Browser, Cover Flow and Window retain generic measurement warnings despite successful dedicated rendering; those warnings are not automatic-layout failures. See [ON/OFF switch](SWITCH.md). Fitted previews and default export success do not establish native canvas sharpness, every property variant or exact visual equivalence.

The Shape family now has dedicated geometry, captions/icons, colors/opacity, inspector and insertion/category routing; see [Shapes](SHAPES.md). Triangle quarter-turns are implemented. Other shapes intentionally ignore rotation, so the initial ellipse/45-degree comparison was not a valid defect test. Native visual fidelity and tiny torn-content masking remain open.

Arrow geometry, captions, color, heads and three-point editing are implemented in [Arrows](ARROWS.md). Horizontal-rule styles and inspector routing are implemented in [Rules](RULES.md). Whole-control, row and inline Page/Back/Web Address editing, destination creation and transfer preservation are implemented in [Link regions](LINK-REGIONS.md). Native appearance and interaction remain explicit acceptance work. Image source, crop and rotation use a separate inspector and must not be classified as missing solely from the common property loop.

## Property rules

The catalog supplies typed values, context defaults, enum/range checks and per-control visibility. Geometry remains in the node. Ordinary edits store differences from current defaults; component patches compare against inherited effective values. Reset removes explicit values. Unknown stored fields survive decoding; unsupported new edits fail. Legacy checked flags affect checkbox, radio and switch states. Existing ellipse/line/panel IDs remain recognized.

Document validation rejects known property type and state errors before saving. Schema presence and inspector visibility do not by themselves prove that a renderer consumes the field. G01 requires checking meaningful variants against each family's contract. Family documents retain their detailed requirements; this audit does not supersede them.

## Layout and evidence boundary

The layout engine expands components and resolves sparse properties without mutating project data. Requested dimensions, uncropped bounds, clipped bounds and inherited clipping remain separate. Ancestor positions, locks and markup propagate through groups; migrated paint order remains stable. One expansion budget covers the whole page.

Text measurement and positioned drawing are shared by canvas and exports. See [rich text](RICH-TEXT.md), [inspector and states](INSPECTOR-AND-STATES.md) and [outlines](OUTLINES.md) for their contracts. Fixed controls can retain an old measurement warning despite having a dedicated renderer; inspect each warning rather than treating it as either automatic failure or acceptance. Unsupported automatic layout is an error.

The release checklist owns completion. Automated defaults and variant comparisons are source-level evidence; they do not close native appearance, interaction, platform or packaging gates. The remaining family work must be followed by the integrated scenario pass on a coherent candidate.
