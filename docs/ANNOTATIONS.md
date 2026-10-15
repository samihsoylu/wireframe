# Annotations

Callouts, sticky notes and tooltips are available through Quick Add and the Markup library. They use the same scene for the canvas, thumbnails, library previews and PNG/SVG export.

Callouts start at 39×39 and grow with text while automatically sized. Resizing fixes both dimensions; Auto-size restores automatic sizing. Fill opacity affects only the background. Text and inline links switch to white on dark opaque fills. Sketch callouts have nine deterministic original outlines; Clean callouts use a blue rounded fill by default.

Sticky notes start at 109×123 and keep their dimensions when text changes. Sketch uses original paper facets and a red tape strip; Clean omits tape. Tape widens for wide notes. Left alignment anchors short text at the left padding. Center and right alignment use a centered content-width box, with right alignment applying within that box. This is intentional family-specific placement, not a change to shared text alignment.

Tooltips support four corner pointers. A pointer appears at 32×28 or larger. Pointer direction preserves the dimensions. Tooltips retain macro text literally; callouts and notes expand `{mockup-name}` to the page name. Names are inserted as literal text, and original source text is preserved in the document. All three support styled text and inline links; callouts and notes also support bullets. Annotation rows outside the label's vertical extent are omitted, and oversized glyphs are not allowed to overflow a narrow line.

Both axes can be resized to a minimum of 10 through the inspector and pointer handles. Smaller saved geometry remains loadable without being rewritten just by selection. Callouts and notes are marked as annotations in the document; tooltips are ordinary content. Component edits use local overrides and participate in Undo/Redo.

`examples/render_annotations.rs` produces a local gallery and supports numeric layout diagnostics. Artwork and fonts are independently distributed; exact contour and font equivalence is not claimed. Native Windows/Linux acceptance and broader editor/recovery work remain open.
