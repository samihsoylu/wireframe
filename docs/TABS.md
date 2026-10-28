# Tab controls

Tabs and Vertical tabs are in the Layout library. Horizontal labels use commas; `\,` keeps a comma inside a label. Vertical labels use newlines. Empty labels retain their slots. Rich text is supported, including multiline labels inside horizontal tabs.

Both dimensions begin automatic and can be resized independently. There is no Auto-size action. Horizontal tabs can be on the top or bottom and aligned left, center or right. Vertical tabs can be on either side. Automatic vertical height includes space above and below the labels; a single or empty label retains a minimum content area.

The inspector offers selected tab, fill opacity, border and optional scrollbar. No border keeps the dividing line but removes the content-area fill. Selected text changes contrast for opaque dark fills. Scroll position moves the diagram's thumb; it does not scroll other controls.

Expand **Edit tab links** to assign page, Back or web destinations. Tab links work in Present mode. Horizontal tabs whose right edge does not fit are omitted; vertical tabs whose bottom edge does not fit are omitted. Their targets and selection remain saved. Narrow vertical tabs clip their labels. Visible hit regions obey control and ancestor cropping. Inline link markup is decorative; the tab's assigned target controls navigation. Web destinations are retained as SVG anchors.

Label edits preserve links by label and occurrence, with a positional fallback for unmatched labels at unchanged count. Selection follows its label, then its previous position when the count stays the same. A continuous edit uses its initial metadata, so temporary empty text does not lose targets. Undo/Redo, component overrides/reset, clipboard remapping and local save/reopen share the existing editor model.

Canvas, thumbnails, PNG and SVG use the shared scene and original outline geometry. Typography, exact contours and broader platform validation remain under development. The gallery generator is `examples/render_tabs.rs`; focused tests are in `tests/tabs.rs`.

A named native macOS shared-selection check in extracted `97e7a99` combines a two-item Tab Bar with three-item Button/Link Bars. Choosing the third combined row leaves the short Tab node unchanged while updating eligible controls. Full saved graphs, shared index Reset, one-step Undo/Redo, current-row no-op preserving Redo and cold Save/empty history pass. All labels, destinations and fixed Tab geometry remain exact; original data/settings are restored. Private `appearance-finishing-current/delivery/mixed-row-selection/REPORT.md` records the scope and corrected sample-markup attempt. Other nesting/crop/order/platform and paired visual variants remain open; no runtime/package change, Windows held.
