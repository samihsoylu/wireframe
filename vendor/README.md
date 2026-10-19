# Maintained dependency extensions

## Native input

Pinned egui and egui-winit0.36.2 copies add a typed `SmartZoom` event. The native adapter forwards `WindowEvent::DoubleTapGesture` in the existing ordered input queue and requests a repaint. The application handles Actual Size/Fit; ordinary mouse double-clicks and pinch factors retain their original meanings. Upstream MIT/Apache notices remain intact; these additions are GPL-3.0-only. Each directory includes the license and an original-file hash inventory. Cargo patches apply the same event type throughout the locked dependency graph.

Native menus additionally use an opaque `AppCommand` event and a viewport-scoped input queue. The adapter drains native commands before the next window event and before extracting frame input, retaining keyboard/menu ordering. Paste uses the existing window clipboard backend. Weak registration prevents commands from reaching destroyed windows; headless contexts retain in-window menus. These original additions are GPL-3.0-only.

## Confirmed clipboard publication

The original `CopyTextWithResult` output and correlated `ClipboardWriteResult` input let the application defer destructive Cut until the native backend succeeds. These additions are GPL-3.0-only; upstream licenses remain intact. The backend writes and reads back through arboard, or stores and verifies a Wayland selection with bounded settling retries. An unavailable native backend is an error; the in-process fallback is not accepted as system publication. The adapter appends the result to ordered input and requests a redraw. Application commands following Cut wait for this reply, and stale document/target/session results do not delete content.

Ordinary text-field Copy/Cut and existing `CopyText` callers keep their upstream path. The verified native scope must be recorded separately for each platform; a compile check or injected completion event does not prove OS clipboard behavior. Wayland read-back timing and X11/Windows runtime remain separate acceptance requirements.

## Regional raster rendering

These are pinned copies of resvg 0.48.1 and tiny-skia 0.12.0. Upstream source notices and licenses are retained. Wireframe additions are GPL-3.0-only; each directory includes that license and an upstream source/hash inventory.

The ordinary drawing APIs remain available. The added regional APIs keep the full logical canvas and layer coordinate systems while retaining only requested output pixels. This avoids changing curve subdivision, antialiasing or image sampling when an export is split into chunks.

- resvg adds regional SVG-tree traversal and exposes existing gradient/image decoding helpers within the crate. Regional traversal covers the scene generator's paths, text, PNG images, groups and clip paths. Unsupported SVG filters, alpha masks and paint patterns return an error; they are not silently omitted.
- tiny-skia adds regional path/rectangle painting, reuses the existing stroke classification and raster pipeline, and gives shader coordinate seeding a default-zero origin. Existing drawing calls retain zero origin.

Cargo manifests use local paths, retain upstream dependency versions, declare the combined license terms and omit upstream example/test targets not distributed here. `UPSTREAM.json` identifies changed source files and hashes the originals. Keep these changes narrow when updating dependencies and rerun canonical pixel comparisons, including large-canvas boundaries and nested opacity/clipping.

Regional buffers are bounded by the requested output region. Path/scanline state and decoded image/font resources have separate costs. Cancellation is checked between scene nodes and regions; one raster operation or a blocked filesystem call is not interruptible.

## Interface font instances

Pinned cosmic-text 0.19.0 adds immutable named font instances with variation axes and logical point size. Each receives a fresh database ID so cached shapes, raster images and outlines remain separate. The same normalized variation location feeds shaping, metrics and both raster paths. The existing default construction remains available. Instance settings belong to the FontSystem; extracting its database alone does not retain them. Application code keeps that system alive with its layouts.

Pinned epaint 0.36.2 adds opt-in automatic optical sizing and native tracking. Logical point size stays independent of display scale. Project/canvas families leave this option off. The egui font-tweak inspection UI preserves the setting. Upstream notices remain intact; original extensions are GPL-3.0-only. Their directories contain original source hash inventories and both license sets. Upstream benchmark/test targets and supporting test fonts are omitted from these dependency distributions.

## Numeric edit cancellation

The numeric drag-value widget discards its text buffer when Escape ends an edit. The existing two-pass focus-loss notification must not apply a cancelled draft on the following pass. The inspector preserves keyboard ownership until cancellation reaches the field; application tests cover the subsequent idle frame and exact document history. This correction is GPL-3.0-only; upstream notices remain intact.

## Popup arrow-key navigation

Directional focus retains each widget’s paint layer and excludes candidates behind the current layer order. This prevents a popup option from moving focus into an obscured inspector field. Same-order layers remain eligible for nested-menu navigation; ordinary Tab traversal and pointer handling are unchanged. Keyboard activation or Escape restores focus to the combo-box button after its menu closes; pointer dismissal keeps its destination. A full-application inspector regression covers arrow navigation, bounds, selection, unchanged drawing data until activation one-step Undo/Redo, and cancellation preserving selection and Redo. This correction is GPL-3.0-only; upstream notices remain intact.
