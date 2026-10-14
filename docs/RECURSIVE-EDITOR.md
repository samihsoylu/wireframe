# Recursive editor checkpoint

The desktop app now edits the version-2 Project directly. The workspace module owns transactions, history, page/context selection, structural clipboard and tree transformations. Rendering receives disposable control projections; saving and recovery always serialize the original recursive project, including properties/resources that the current inspector does not expose.

Groups have their own identity and parent-local children. Enter, double-click or Edit group contents selects a nested editing context. Escape and Exit group move to its parent. Grouping preserves existing paint order, including nonadjacent selections. Ungrouping preserves world coordinates and repairs sibling identity collisions. Movement, cancellation and undo/redo retain descendants. Resizing uses catalog axes for leaf sizes. Full-enclosure marquee excludes locked nodes; hit testing follows visible leaf order and clips.

Clipboard data includes selected subtrees and referenced definitions/assets. Matching dependencies already present in the target are reused; imported resources receive new IDs. Root node IDs change while scoped descendant/override paths remain stable. Component instances are atomic entries in explicit paint order, with their expanded descendants kept together. Missing page links are cleared when importing clipboard data.

ProjectRepository separates app workflow from disk storage. ProjectStore reads both file versions and writes version 2 atomically. Opening preflights layout before replacing active work. Failed saves leave the document dirty and retain its contents. Recovery uses the same version-2 path and existing exclusive ownership lock.

Canvas, thumbnails and SVG/PNG exports use a shared scene with inherited clips. Basic rich-text controls and tables now use positioned glyphs and structured cells; other controls retain the initial primitives. See RICH-TEXT.md for verified coverage and limitations. Valid documents requiring unsupported automatic layout cannot yet open; the source and active project remain intact.

Native macOS verification used a copy of a saved two-control fixture. The app created two nested group levels, edited one child, moved it ten units, undid/redid, saved, closed and reopened. Re-entering the hierarchy showed the persisted X=338 coordinate; the file retained four leaves inside three group nodes. Screenshots, accessibility snapshots and the saved fixture are retained locally.

Automated verification totals 91 passing tests. New checks cover hierarchy/context round trips, transactional movement/cancel, lock/enclosure selection, nonadjacent grouping, failed edit rollback, clipboard dependencies, resize-axis handling, crop preservation, scoped layering/hit testing and atomic component paint order. Raster crop checks inspect the output dimensions and pixels inside/outside the mask. An injected disk-full failure verifies the app retains dirty state. Five saved version-1 fixtures retain geometry, order, text and source bytes through migration and editor projection. Clippy is clean with warnings denied.

Remaining work includes local component override/reset editing and resizing, group resize rules around instances and crops, richer inspectors, asset/version workflows, full rendering/export fidelity, large-project performance and Windows/Linux native validation. This checkpoint establishes usable nested editing on macOS, not completion of the product.

Typed text/table properties and six-state choice rendering are integrated; see INSPECTOR-AND-STATES.md for the current field boundary and native evidence.

## Component workspaces

Components now have a sidebar and source workspace. Select one control or group and choose Make component; use the sidebar + to insert another instance, click its name or Edit component source to edit shared content, and Back to wireframe to return. Source edits use existing nested editing, typed properties, clipboard and history. Detach component materializes current effective values into an ordinary group. Validation rolls back cyclic insertion; locked conversion/detach and interleaved group conversion are rejected without changing the document. Existing local overrides remain effective, but editing/resetting them in the UI is still pending.

Native macOS verification created a component from a form group, inserted a second instance, renamed its source to Contact row and changed its input text to Email address; both instances updated. Detach, Undo and Redo passed. A later source change to Contact email affected only the linked instance. Save, quit and reopen preserved the linked instance and detached group's different values. Evidence: evidence/components/. Three regression tests additionally cover source grouping/movement/clipboard, cycle rollback, locks, paint order, overrides, history and storage. The refreshed development bundle is running the saved synthetic fixture; this is scoped self-verification, not independent review.
