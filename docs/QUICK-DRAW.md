# Keyboard drawing

Hold **I** for Image, **R** for Rectangle, **T** for Text block, **Y** for Text line
or **A** for Arrow, then drag on the canvas. The held preview is translucent and
uses the shared renderer. Release inserts one selected control with one Undo
entry. A preview does not allocate document IDs or alter saved data.

Modes follow held keys, with priority I, R, T, Y, A. Changing keys during a drag
changes its preview. Releasing every drawing key before the mouse returns to
marquee selection; a mouse release followed by key release still inserts the
control, even when both arrive in one frame. An ordinary marquee can become a
drawing before release. Space/middle-button panning takes precedence.

Arrows follow the drawn direction with one head. Motion samples can produce a
curve; near-straight gestures remain straight. Release updates the endpoint but
does not add an artificial curve sample. Collapsed endpoints retain finite data.
The creation origin snaps to sibling edges, spacing and centers, including the
previous selection. Slow drawing also snaps the moving rectangle; fast movement
suppresses that attraction. Existing canvas edge scrolling applies while held.

Group insertion converts world coordinates to the group's local coordinates.
Locked groups and component instances reject structural insertion. Escape,
focus loss, modal/text entry, or a changed editing surface cancels transient
creation. Text fields and Quick Add keep their keys. Cancelling restores the
selection only while still in the same project and editing context.

The implementation uses live UI fonts for temporary layout and never creates a
new Editor per preview. Canvas artwork and export artwork share the same control
renderer. Editor selection decorations and the empty-canvas prompt are hidden
while drawing.

## Acceptance limits

Windowless event tests cover all five tools at three zoom levels, ordered and
coalesced releases, curvature, mode changes, origin snapping, cancellation,
panning and page changes. Integration checks cover reverse/vertical/collapsed
arrows, grouped insertion, structural restrictions, history and saved decoding.
Shared-scene preview/release renders are a visual self-review, not native input
or editor-window acceptance.

Native checks still need held-key delivery, physical cadence and edge scrolling,
matched preview/minimum-size appearance, and tool-cursor appearance. Drawing
currently uses a crosshair and keeps a held key active until release/cancellation;
there is no inactivity timeout. These differences remain explicitly recorded for
the integrated fidelity pass. No current Windows/Linux execution is claimed.
