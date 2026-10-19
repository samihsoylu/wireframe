# Viewport behavior contract

Status: core mathematics, raw input routing and per-surface preferences are implemented with scoped native macOS checks. Smart double-tap dispatch is implemented with application tests; physical gesture delivery/cadence and native cross-platform acceptance remain open. This contract supersedes the former multiplicative button zoom, arbitrary Fit margin and origin-anchored pinch handling.

## View ownership and persistence

Retain independent zoom and scroll values for every editor surface: each official page, alternate and component definition. Identify them by project/session plus stable surface ID, never by page index or display name. New surfaces start at100% and the origin. Switching pages/alternates restores that surface's view. Entering an ordinary group shares the parent surface's view. Version rename/reorder must not discard it; explicit surface replacement/promotion/move needs a defined identity mapping and tests. Presentation has its own transient view and must not overwrite saved editor views.

View changes are local preferences, not document edits or Undo records. Restore them on reopen. Reuse the existing project preference identity/recovery conventions, including Save As and untitled recovery. Invalid/nonfinite settings must not create unsafe transforms; preserve unreadable settings rather than overwriting them. Coalesce writes by the complete surface identity or save the full latest map atomically. A batch affecting multiple versions of one page must persist every changed surface. Bound stored data and avoid synchronous writes on every animation frame.

## Zoom actions

- Button/keyboard steps:10,25,33,50,66,75,100,150,200,250,300%. Between stops, choose the next strictly higher/lower stop. Disable actions at their endpoints.
- With a movable selection, center its document-space bounds in the available view. Without one, retain the old visible document-space center. Round resulting scroll offsets to logical points and floor at zero. Locked-only selection must not become the edit-mode zoom anchor.
- Actual Size sets100% while preserving the old visible center. All Actual Size sets every relevant surface to100% at the origin and clears selection/exits ordinary groups. Include alternates; do not reset unrelated component-library surfaces from a page command.
- Fit uses the full page visual bounds, including visible markup, or the appropriate component bounds. For positive dimensions, scale is min(viewWidth/(boundsWidth+20),viewHeight/(boundsHeight+20)), clamped10–300%. Empty/invalid dimensions use100%. Position the bounds' top-left minus10document units at the view origin, with scroll rounded/floored at zero. Fit is not capped at200% and is not a fixed50-screen-point centered margin. All Fit applies this separately to each relevant surface.
- Smart zoom toggles Actual Size/Fit according to whether the current zoom is exactly100%. Toolbar double-click and the corresponding All shortcuts must have deliberate dispatch without losing state through their first click.

## Wheel and pinch

Use ordered raw input events so each physical wheel/pinch event is applied once. Do not infer the event type from the combined egui zoom_delta value. The checked egui0.36.2 input path moves Command-wheel into zoom_delta while smooth_scroll_delta is zero; the current application consequently routes it through the origin-anchored pinch branch. egui also smooths wheel zoom over later frames. Avoid applying both raw events and this synthesized residual zoom.

On macOS Command-wheel, a positive vertical event supplies+0.06 and a negative one−0.06, independent of magnitude. Use the platform's normal command modifier for the other desktop targets, with native validation still required. A zero vertical event should not unexpectedly zoom. Pinch supplies its signed magnification; current egui-winit converts it to exp(delta), so the raw Zoom event's logarithm recovers that input. Keep multi-touch versus synthesized-wheel paths distinct.

For signed delta d, compute z*(1+d) when d>0, otherwise z/(1-d); clamp to10–300%. Show100% when the computed scale lies inclusively in85–115%. Retain the unsnapped accumulator between events separated by at most500ms, allowing a continuous gesture to move through that range. After a longer gap, restart from displayed zoom. Anchor the document point beneath the pointer: add point*(newZoom-oldZoom) to scroll, round, and floor at zero. Clamp to the actual available scroll extent as appropriate. Ordinary wheel scrolling remains independent.

## Integration and acceptance

Extend the existing pure viewport module and keep input translation, viewport mathematics and preference persistence separate. Reuse existing workspace scroll limits and drag-operation growth semantics. Keep document rendering geometry, current display density, crop masks, undo/redo, export and native dialogs intact.

Required checks include steps/bounds, selection and no-selection centers,100%/Fit/All actions, between-stop scales, wheel/pinch signs and snap timing, invalid settings, independent surfaces, rename/move/promote/Undo, batch persistence, Save As/recovery, presentation isolation, sidebar resize, drag/autoscroll and cache scale/phase invalidation. Run one coherent native scenario workflow after core regression checks. Verify pointer anchoring and retained views on macOS, and preserve explicit Windows/Linux gaps until native runs. Do not present headless input evidence as physical gesture/cadence acceptance.

## Implementation decisions

`src/viewport/zoom.rs` owns scale/anchor mathematics and gesture accumulation. `src/app/viewports.rs` connects stable surface IDs to editor and transient presentation views. Existing rendering transforms remain the canvas interface. Newly allocated IDs start at100%/origin; promotion replaces content while retaining the official surface's existing view, and its newly allocated former-official surface starts at the default. Rename or movement between named versions keeps the alternate ID. Deleted views remain available within the session for Undo.

`viewport-settings.json` uses the existing project/working-copy identity. The complete latest map is saved atomically after500ms idle or at a two-second write interval during continuous changes, and flushed at project/exit boundaries and All actions. Reads and serialized output are bounded to4MiB. Unreadable files are preserved; write failures retain session state and retry at bounded intervals.

The raw input hook captures ordered viewport events and removes Command-wheel from egui's ordinary wheel queue. This prevents mixed Command/plain-wheel frames from producing unwanted pan residuals. Pinch factors are decoded to signed magnification; synthesized zoom residuals are ignored. Shifted numeric All shortcuts also recognize physical digit keys, since their logical keys can be punctuation.

Pinned egui/egui-winit extensions forward native DoubleTapGesture as a distinct SmartZoom event. Ordered viewport handling applies it once inside the canvas, toggling exact100% to Fit and other scales to Actual Size. Application tests cover ordering, bounds, history independence and presentation isolation. The bridge is included in the optimized typography review candidate, where ordinary mouse double-click still opens text editing at unchanged zoom. Physical smart-gesture delivery is not yet verified. The upstream platform event is available on macOS; no equivalent Windows/Linux gesture is claimed. See vendor/README.md for maintained sources and licenses.

## Ordinary-wheel ownership

The raw hook routes ordinary wheel events using their ordered pointer position and the last painted canvas bounds/layer. Canvas-owned events are removed from the shared UI stream and passed to a separate egui InputState with the current viewport size, stable timing, display density and input options. This reuses the pinned dependency's units, modifier mapping and smoothing instead of inventing another curve. Canvas processing runs once per frame even after the pointer leaves; owned residuals request repaint until they settle. Other panels retain their existing ScrollArea behavior.

A phased touch stream keeps its initial canvas/noncanvas owner until End or Cancel, including a terminal event with Command held. Focus loss, a blocked canvas and surface/presentation changes clear canvas residuals. Modal/overlay hit layers prevent new canvas input. A pointer exit alone does not transfer existing residuals. Zoom remains an ordered raw-event path; ordinary smoothing is never reused as zoom input.

Application regressions cover same-frame mixed/reverse routing, later pointer transitions, point/line/page curves, Shift, plain sidebar consumption, focus, surface/modal boundaries, overlay hits and touch completion. All 153 application tests pass. Native macOS Tiles/List scrolling preserves the contact artwork, canvas scrolling works, explicit origin reset restores it and Save is byte-identical. The earlier native displacement is still not tied to an exact captured native event sequence; physical gesture cadence and Windows/Linux delivery remain open.
