# Scrolling during a canvas gesture

Moving or resizing controls, or dragging an area selection, scrolls the canvas when the pointer enters its 10-point edge band. Holding the pointer still continues scrolling every 100ms. A pointer farther than80 points outside the visible canvas requests a larger step. Horizontal and vertical scrolling work together at corners.

The viewport moves before the gesture is recalculated, so the control or selection continues to follow the pointer. Movement and resize remain a single Undo operation. Releasing, Escape or losing app focus ends scrolling. Space-drag panning, presentation, image cropping and library insertion use their own interaction paths.

Scrollable bounds include 400 viewport points beyond the content, up to the 10000-document-unit workspace extent and trailing margin. Bounds can grow while a gesture is held but do not shrink until it ends. Existing free pan outside those bounds remains available; edge scrolling moves toward the permitted range gradually rather than jumping.

The scheduler keeps one pending tick per gesture. Pointer movement requests an immediate step and renews that tick. This avoids accumulating overlapping timer callbacks. Canvas content extent currently comes from the top-level layout bounds; exact visual overflow, negative-origin workspace sizing and larger existing documents remain broader viewport work.

Validation covers strict edge thresholds, diagonal motion, zoomed padding and limits, timer cadence, retained bounds, free-pan recovery, stationary move/resize/area selection, 50/100/200% movement, cancellation and one-step Undo/Redo. Native held-pointer timing and Windows/Linux runtime acceptance remain outstanding.
