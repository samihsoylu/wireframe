# Arrow contract and implementation state

Arrows now support curve and two-segment modes, independent arrowheads, stroke
style/color/opacity, a movable caption and three editable points. The catalog,
saved model, component overrides, inspector, canvas and exports use this behavior.
[Keyboard quick-draw](QUICK-DRAW.md) is connected; G01 remains open for native fidelity acceptance.

## Compact property inspector

Shape, independent head toggles and stroke styles use named icon controls. Caption
position has five slider stops mapping to1,3,5,7,9; the adjacent number retains
the full0–10 range, including head-dependent defaults2 and8 without normalizing
stored data. Color/opacity, B/I/U, fractional font size and expandable caption
editing retain existing reset and component override behavior. Caption editing
shows readable destination names and stores stable link identities.

Clicking either slider gives it keyboard focus. Directional keys remain with
focused interface widgets; clicking the canvas restores selection nudging.
Save retains the existing canvas focus handoff. Native uninterrupted click/Left,
caption edits/collapse, double-click selection, Escape, resets and exact saved
Undo/Redo pass on Inspector Review. All432 application tests pass; width checks at
220/300/400 points cover the property section rather than the entire inspector.
The subsequent Width Review isolates and fixes that panel growth: fractional
opacity fields exceeded the space reserved for their number, including on the
first pointer-edit frame. Slider layout now reserves the widest permitted number
plus padding/gap. A full-panel test covers pointer edits, caption expansion and
inline links at220/300/400 points and1×/2× density. Native300-point width remains
unchanged through the same sequence and exact saved Undo/Redo. Seven affected
arrow tests and final lint/build/format/signature pass. Evidence: external
`inspector-width-current/report.md`. That macOS check did not establish narrow
resizing. A later current Linux checkpoint supplies the bounded narrow route
below. Common layout, physical slider snapping and exact appearance remain open;
this does not close G01 or other platform/native coverage.

The exact extracted `f944e37` Linux package now reaches the narrow clamp
(approximately 220 logical points from scaled capture/range evidence). Ten native
captures retain the exact same panel edge through opacity 0.49, expanded and
wrapped caption editing, a readable destination resolving to `page:1`, collapse,
Save and one-step saved caption Undo/Redo. The whole opacity file equals retained
macOS bytes; the whole caption graph matches its declared expectation. Cold Save
is byte-exact with empty history. The inspector returns to default width after
restart; width persistence is not accepted. A zero-duration remote click missed
the slider and a following key nudged the Arrow, then Undo restored it. The
accepted 100 ms click/key sequence preserves position. Rapid input, physical
delivery, every property/variant and wider platform behavior remain unverified.
External `appearance-finishing-current/delivery/linux-inspector-width/REPORT.md`
retains measurement/setup exclusions and terminal cleanup. No runtime or package
change; Windows remains held.

Current source/runtime `521d988` reproduces and fixes complete press/release
clicks arriving in one frame. Sliders accept the click and focus on press;
following keys wait for the new widget owner, while Escape retains immediate
gesture cancellation. Six width/batching cases, text ownership and existing
held-history checks pass within 559 macOS application tests (three ignored),
551 Linux tests and 51 egui library tests per platform. Both lint, formatting,
optimized builds and refreshed private package checks pass.

The extracted Linux package repeats the zero-duration click/Left successfully:
opacity becomes 0.44 and X/Y remain 50. Clicking the canvas still lets Left nudge
X to 49. Slider drag changes opacity to 0.80 with one-step byte-exact saved
Undo/Redo; cold Save is exact with empty history. New macOS native input,
physical cadence/snapping and other variants remain open. The older macOS
review app is preserved. External `delivery/rapid-slider-input/REPORT.md` records
the corrected/failing checks, artifact identities and complete isolated cleanup;
the preceding `f944e37` narrow-layout evidence retains its own identity.

## Geometry implemented

`src/arrows.rs` owns endpoints and a middle point expressed along the endpoint
chord and its clockwise normal. A quadratic curve passes through the middle at
half its parameter range. In line mode, caption placement follows distance along
both segments. Bounds include the quadratic extrema, not just the endpoints.

Moving endpoints retains the middle's relative position. Moving the middle edits
that relationship. Shift constrains endpoint movement to horizontal, vertical or
45-degree directions. Mode changes retain the three points in world coordinates.
Resize scales endpoints and curve geometry, rounding endpoints to document units.
Middle reset straightens curves or selects the nearest corner/midpoint for lines.
Frame rebasing uses the original handle target before rounding so half-unit
positions cannot drift through a coordinate conversion.

Collapsed endpoints have no unique relative middle. Keep the previous finite
middle relationship and stable world endpoints; retain a minimum 1×1 frame so a
later endpoint drag can expand the arrow. Resizing a collapsed chord uses the same
finite fallback. Invalid coordinates must never enter a saved document.

## Saved state and editing

Version 13 stores `start_x`, `start_y`, `end_x`, `end_y`, `bend_along` and
`bend_across` as typed scalar properties. Missing coordinates use the frame and
straight midpoint. `shape` chooses `curve` or `lines`; `start_head` and `end_head`
are independent booleans. `stroke`, `border_color`, `background_opacity` and
`label_position` control appearance. Caption position ranges from 0 to 10; its
default is 5 with both/neither head, 8 with only the start head and 2 with only
the end head. An explicit value overrides that dependent default until Reset.

New arrows are 150×100, diagonal, double-headed and markup. Versions 1–12 retain
their original horizontal midline and single right head through `legacy_horizontal`
and explicit style properties. The size-relative fallback preserves old component
size overrides. Migration leaves stored markup unchanged and does not rewrite the
input file. Recovery migrates the corresponding clean baseline without concealing
unsaved changes. Older arrow geometry is preserved; its original placeholder
stroke is replaced by the shared style renderer.

Three handles replace ordinary box handles for one selected arrow. Drag previews
update while held; release records one history action. Shift constrains endpoints,
Escape cancels, and double-clicking the middle resets its bend. Groups and component
instances use the same resize domain. Selection uses the visible path with a
15-unit tolerance and a 20-unit round-ended neutral highlight at20% opacity.
Locked selection uses a lighter gray. Native GPU composition fades the complete
stroke after joining its segments, avoiding darker patches at bends; the existing
unsupported-GL/headless fallback retains approximate per-part fading.

Canvas, PNG, SVG and PDF use a shared path and caption. Curves subdivide by geometric
flatness. Strokes have rounded joins/caps; dotted strokes use a 1-unit mark/8-unit
gap and dashed strokes use mark/gap lengths of 3/6 times the stroke width. Heads
are original parametric chevrons: compact round-ended Sketch legs and thinner
square-ended Clean legs. Captions have a light bordered background and
retain inline links. Bounds include heads and captions beyond the control frame.
Opacity fades the path, heads and caption background. Caption text remains fully
readable, including at zero opacity. Path segments use rounded filled surfaces
for consistent native and exported caps; dotted marks keep the normal 4-unit
Sketch or 2-unit Clean width, with 1-unit marks and 8-unit gaps. Path and caption
background use separate opacity scopes so native compositing retains both. Each
head also has its own opacity scope, so its overlap with the line darkens while
the head's own leg joint stays uniform.

## Remaining acceptance

Keyboard quick-draw now supports a translucent gesture preview, directional curved arrows, release insertion and one Undo action. See the [drawing contract](QUICK-DRAW.md) for input boundaries and remaining native checks.
Native comparison now verifies compact Sketch head extent and partial-opacity
overlap, with thin Clean heads retained. Sketch taper/asymmetry remains a bounded
contour difference. Remaining matched-state checks include caption metrics,
remaining inspector appearance and physical drag/scroll. Straight, bent and locked
selection highlights have scoped native acceptance in `arrow-selection-current`. Native middle reset is
retained in the routing evidence. The current native arrow comparison covers
caption opacity, dependent head defaults and explicit position persistence; see
the release checkpoint for candidate-specific results and remaining contour gaps.
Do not treat the headless UI tests as native acceptance. Preserve the specific
collapsed-coordinate fallback rather than reproducing invalid saved data.

Seven geometry regressions and the retained 518-case isolated comparison establish
the domain, with explicit degenerate differences. Integration checks cover saved
properties, dependent defaults, mode/resize/handle history, component edits,
legacy migration, recovery, hit testing and exports. Windowless UI checks exercise
held/coalesced drags at three zoom levels, Shift, Escape and inspector availability.
Fourteen PNG/SVG specimens and two rendered PDFs have visual self-review. Native
and cross-platform acceptance remain separate release gates.
