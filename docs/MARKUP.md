# Braces and markup strokes

Horizontal brace, Vertical brace, Red cross and Scratch out are available in the Markup library and Quick Add. All four start as markup and resize independently in either direction. Editing enforces a ten-unit minimum; smaller saved dimensions remain intact and render within their frame. These controls do not offer Auto-size or a whole-control page link.

| Control | Initial size | Editable content |
| --- | --- | --- |
| Horizontal brace | 200 × 80 | Multiline rich text; top or bottom direction |
| Vertical brace | 180 × 140 | Multiline rich text; left or right direction |
| Red cross | 240 × 104 | Geometry only |
| Scratch out | 205 × 107 | Color and opacity |

Brace artwork occupies 15 units across its short axis, with a 20-unit text offset. Direction determines text alignment. Text appears only when both available text dimensions are positive. Font size, bold, italic, underline, text color, inline links and page-name expansion use the shared text engine. Changing text does not resize a brace. Text color also colors its brace.

Red cross uses fixed colors: `#771822` in Sketch and `#cf2b28` in Clean. Scratch out defaults to black in Sketch and `#333333` in Clean; its opacity ranges from zero to one. The static marks do not open a text editor.

The artwork is original procedural geometry. Scratch out uses one continuous ribbon with non-overlapping turns, preserving uniform transparency. Canvas, PNG and SVG consume the same drawing data. Original contours, text metrics and rasterization remain fidelity work.

`examples/render_markup.rs` generates a 40-control gallery in both styles, including narrow, tiny, reversed, empty, formatted and translucent examples. It also accepts neutral JSON layout cases with `--geometry`. `tests/markup.rs` checks placement, strict text thresholds, history, component-local dimensions, storage, formatting, clipping and mesh validity. Shared app tests cover inspector support, resize minimums, tiny selection and keyboard editing eligibility.

## Classification commands

Treat as Markup is available from Edit, the canvas context menu and Command/Ctrl+Shift+K. A selection containing any plain control becomes markup; an entirely marked selection is cleared. Position locks remain intact and do not block this appearance change. The action changes all selected controls in one Undo step. Component-instance classification remains local; source classification keeps its owning source semantics. Text editors and modal dialogs retain command ownership.

## Editor visibility

View > Markup and Command/Ctrl+K toggle a per-project view preference, defaulting to visible. The preference is stored with canvas views, outside project data and document Undo. Existing view files migrate on a subsequent change; unreadable files remain preserved. Save As, project switching and recovery reuse the existing preference ownership rules.

Hidden markup is excluded from editor artwork, hit testing, direct/marquee/Select All selection, snapping and Fit. Entered-group scene caches and text previews retain the visibility setting. Hiding prunes invisible selections; leaving a hidden group cannot select that group. Source flags and instance-local overrides use resolved appearance. Newly inserted markup becomes visible, including sampled paste and history restoration; classification and ordinary regrouping do not toggle the view preference.

## Geometry and export scope

Movable group selection handles, hit areas, marquee containment and pointer resizing retain the full group frame when markup is hidden. Snap targets use separately calculated bounds excluding marked descendants, intersected with existing masks. Nested groups and resolved component overrides preserve their stored geometry; a group containing only marked descendants has zero extent for these snap calculations.

PNG and SVG exports inherit editor visibility. Image workers capture that preference with the document and dynamic drawing values; an explicit snapshot override can include or exclude markup independently. PDF options capture the current editor preference when the export dialog opens and carry it into the worker. Hiding markup also removes its link annotations. Group/selection scope filtering retains original masks and placement. Canonical full scenes and navigator thumbnails continue including markup; presentation keeps its own setting.

The inspector displays bounds excluding hidden markup. Position-field changes move the complete group without adopting the smaller displayed width/height. Size-field changes target the full group: for example, a380-unit group with80 visible units becomes160 units overall and34 visible units after setting Width to160. Pointer resizing retains its full-frame behavior. Component-local edits preserve the definition and locked geometry stays read-only.

Group crop preview omits hidden markup and uses the visible user bounds. For an existing crop, the preview's full frame is reconstructed from those visible bounds and the stored fractions. Commit stores crop fractions against the complete group while retaining the chosen corner; this can make the saved result differ from the preview. Preview and background do not reveal marked descendants. Escape preserves data and Redo. A group with no visible crop extent rejects crop entry without changing the document.

When markup is hidden, Remove crop reconstructs the uncropped origin from the visible bounds and existing crop fractions before clearing the crop. Leading marked children can therefore make the complete group move. Children, component definitions and unrelated properties remain unchanged; Undo restores the exact prior document. Empty visible masks remain removable even though entering a new crop preview is rejected.

Arrangement commands measure visible bounds while moving complete controls. Edge and spacing commands retain the visible origin offset when writing the full-group origin; center commands use their corresponding center offset. This can produce surprising placement when hidden children lie before the visible children. All ten actions and history are covered by `tests/markup_arrangement.rs`; native Left and Center have saved comparison evidence.

Locked selection outlines include hidden descendants. They use a dashed individual frame and a lock badge whose screen size stays constant with zoom. Locked controls are excluded from the movable aggregate outline and resize handles. Clicking a badge selects and unlocks only that control, preserving sibling and ancestor locks. Undo/Redo keeps the target selected. Dragging away cancels activation; Shift/Command-click does not unlock. Badge hit areas remain aligned with the drawn full frame even when markup is hidden. These two interaction choices intentionally avoid invisible hit areas and accidental drag activation. Fill and contour styling are not pixel-identical.

The visibility feature is not fully accepted. External `editor-markup-arrangement-current/report.md` records native inspector/crop/alignment/locked-frame/SVG/cold-reopen checks on the combined candidate. Paired native nonzero-offset gestures and broader insertion/source/alternate variants remain open. The earlier inspector and geometry reports retain their unchanged evidence. The protected drag-review candidate predates these source changes.

Native PNG/PDF export now has paired shown/hidden acceptance on a three-rectangle fixture containing one marked group child. Transparent 1× PNG and content-sized PDF without margins measure 681 × 40 with markup shown and 382 × 40 hidden; surviving controls retain their relative placement. The hidden PNG is pixel-exact against the corresponding region of the shown product PNG. Both project drawings remain unchanged. External `editor-markup-exports-current/report.md` records 21 artifact assertions and the unchanged executable identity. Native hidden-link annotations, masks, alternate pages and broader export settings remain unverified by this checkpoint; cross-application rasterization is not pixel-identical.

External `editor-unlock-current/report.md` records native badge click, drag cancellation, Shift-click, exact saved history and hidden-markup hit-area checks. Component-local and inherited-lock behavior, input cancellation and document replacement are covered by automated tests; broad native variants remain unverified.

The later current `65d0292` native Mac PDF checkpoint verifies one Clean rectangular group-crop with official/Wide alternate pages: hidden markup artwork and URI annotations disappear, clipped ordinary links remain exact, and an omitted official target resolves to its included alternate. Eight rendered pages, ten exact annotations, five pixel comparisons and complete unchanged project/empty history pass. All original data/settings and package identity are preserved. Private `appearance-finishing-current/delivery/pdf-hidden-crop-alternates/REPORT.md` records this named scope; broader soft/irregular/image/nested masks, reader interaction and platform variants remain open. No source/build/package change.

External `editor-markup-offset-crop-current/report.md` records the crop-removal correction,25 domain/11 application crop checks and16 native artifact assertions. Default crop/removal matches the paired saved coordinates; native product corner-drag, nonzero-offset commit/removal and exact history pass. The corresponding physical comparison gesture remains pending because automated input did not move that preview. No broad native component, nested-mask or other-platform crop acceptance is claimed.
