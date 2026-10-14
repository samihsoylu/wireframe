# Phone layout implementation checkpoint

The original device layout engine is implemented in `src/phone.rs`. Integration with the library, inspector and shared renderer is still in progress. The running development app continues to use the existing Phone appearance; this checkpoint does not accept the new control.

`Options` describes the model, orientation, resizing mode, background pattern, transparency and status bar. `layout` takes fixed outer dimensions, appearance and a text-measurement callback, then returns the fitted frame, screen area, ordered fills, status content, detail placements and proximity guides. Artwork is kept separate from this geometry. The callback receives the actual status font size and returns carrier/time metrics, so the shared font backend can determine narrow-label visibility.

Four models have distinct intrinsic sizes and frame sections. Automatic dimensions use the intrinsic size in the selected orientation. Fixed dimensions remain fixed when model/orientation changes. Fit mode centers a proportional frame; stretch mode adapts its insets and detail visibility. The model default is Classic/Fit, while new insertion will explicitly choose Notched/Stretch. Existing saved dimensions must be preserved.

Fill transparency leaves pattern bars and frame artwork opaque. Status visibility uses strict model/style size gates. Tiny fitted notched frames can retain time while losing icons. `guides` stores local left/top/right/bottom coordinates, not a width/height rectangle. Rotated detail bounds retain their rotation anchor; `visible_bounds` produces their axis-aligned rectangle.

Five focused tests cover centered fit, orientation, strict status boundaries, transparent patterns, small fitted status content and stored defaults. App wiring, original contours, render/export pixel checks, history/storage/component integration and native acceptance remain pending.
