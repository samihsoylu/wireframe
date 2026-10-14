# Alignment while editing

Moving and resizing controls attracts to nearby sibling edges, centers and spacing guides. Attraction uses five document units at every zoom level. Center alignment takes priority while moving; the active edge takes priority while resizing. Overlapping guides resolve in sibling order. Containers use their shared content layout for inward spacing.

Hold Command on macOS or Control on Windows/Linux to bypass attraction. Shift constrains movement to one axis or preserves resize proportions. Alt also bypasses resize attraction. Fast movement temporarily suppresses attraction using recent pointer-event distances. Snap to 10 remains a separate explicit grid mode.

Multiple selected controls move with a shared delta. Locked controls do not move but remain alignment targets. Nested editing uses the current group's siblings in global coordinates. Guides are transient; they are never saved or exported. Releasing commits one Undo step. Escape or focus loss cancels the gesture. Pointer presses are retained across render frames so quick completed drags are not discarded.

Phone and Smartphone retain the ordinary 16-unit exterior spacing targets in addition to their interior content guides. Smartphone adds interior guides only in Fit; Tablet uses ordinary outer targets. Interior guides remain attached to the visible frame when another sibling moves to its left.

Library insertion has its own pointer-following preview and is separate from existing-control alignment. Additional control-specific boundaries, arrow endpoints, keyboard-only guides, Alt-drag duplication, remaining autoscroll cases and hidden-markup filtering remain follow-up work. Exact native guide appearance and Windows/Linux interaction acceptance are unverified.

Checks cover geometry priority and thresholds, target order, center resize limits, event-distance suppression, locked neighbors, group coordinates, window content boundaries, multi-selection, zoom, modifier changes, focus cancellation, Undo, and complete or split-frame native event delivery.
