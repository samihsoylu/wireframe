# Icons

Lists support one optional icon per raw text row, including the header. Expand **Row icons** in the inspector, choose an icon, search its name or keywords, and filter by Solid, Regular or Brands. No icon and the remove button clear one assignment; Reset restores the default or inherited component property. Search and filter clicks keep the picker open. Assignment and removal are undoable.

The offline catalog contains 2,060 Font Awesome Free 6.7.2 variants. Artwork comes directly from the official public upstream at the immutable commit recorded in assets/icons/font-awesome-free/PROVENANCE.json. scripts/import_icons.py verifies both input checksums and regenerates the compact, sorted catalog. Paths and dimensions are unchanged; metadata is compacted and only free variants are selected. Artwork retains CC BY 4.0 attribution and the upstream license; original application code remains GPL-3.0-only. THIRD_PARTY_NOTICES.md includes the attribution, and exported SVG metadata and PNG Attribution iTXt carry it when the scene contains built-in catalog icons.

A List with any icon reserves one shared column: text starts font size+16 units from the left, and automatic width gains that amount. Each icon occupies a font-size square at x+8 and follows the row text's vertical origin. Paths fit proportionally and center within that square. Icons use the row's base ink; inline disabled text does not dim the icon. Control and ancestor clipping apply. Fixed control dimensions remain fixed. Removing the final icon removes the shared column on automatic controls.

Canvas, thumbnails, library previews and exports consume the same icon scene entries. SVG contains vector paths; PNG rasterizes those paths. Canvas textures use bounded resolution, at most 128 cache entries and 16 MiB per context, with only visible icons painted. The picker virtualizes result rows. These bounds do not replace large-document profiling.

Row assignments use typed nullable values, not embedded path data. Text edits follow the same raw-label occurrence mapping as row links, including transaction baselines, duplicate labels and same-count replacements. Component-local changes leave source definitions untouched; Reset resumes inheritance. Syntactically valid unknown IDs survive storage and display an outlined placeholder, allowing later catalogs to resolve them. Invalid IDs and oversized arrays fail validation.

## Verification and remaining work

Four integration tests cover all 2,060 icon rasters/search/aspect ratios, row mapping/history/validation/storage, shared geometry/colors/clipping/vector and PNG attribution, and component-local reset/copy behavior. The complete suite now has 141 passing tests, including four custom-image tests below. Formatting and Clippy with warnings denied pass.

Native macOS acceptance covers camera search, Regular heart filtering, assignment/Undo/Redo, Reset shrinking an automatic List from 142 to 102 units and Undo restoring 142, text-row reordering with assignments retained, save/quit/restart/reopen, and PNG/SVG exports. The reopened fixture has Camera on Title, Asterisk on Bravo, Cloud on Alpha and Asterisk on Disabled; Undo/Redo are empty after restart. Export metadata and all three distinct icon path sets were checked, and the PNG was visually inspected. Local evidence: evidence/list-icons/; fixture generator: examples/render_list_icons.rs.

Icon properties beyond the currently implemented families, broader artwork coverage, typography/contour fidelity and Windows/Linux native acceptance remain open. No independent review is claimed.

## Custom project images

Choose **Project images** in the row icon picker to search embedded images or **Import image…** to add a PNG, JPEG or GIF. Previews show source colors. Same-name conflicts offer Use new, Use existing, Keep both and Cancel. Use new replaces the shared image everywhere in the project. Import plus row assignment is one undoable operation; original bytes remain embedded and need no source file after import.

Custom icons use the same shared text column and row clipping as catalog icons. Their source rectangle stretches into the font-size square. A linear-light color matrix combines source RGB with alpha and the row's base ink; alpha stays separate. This differs from the ordinary Image control's sketch effect, crop and rotation. Custom icons inherit no Image-control appearance properties.

Canvas, thumbnails and exports share tinted raster entries. SVG embeds PNG data with square stretching; PNG composites the same entries. Identical asset/color variants share decoded data within a scene. Normal images and icon variants share a 128 MiB decoded RGBA budget; unreadable or over-budget custom icons draw a placeholder. Picker previews retain at most32 textures of at most64 pixels per side. Large-document profiling remains open.

Four integration tests cover color/alpha/extent, atomic import/conflict/history/validation, component-local ownership/reset/cross-document copy, and shared geometry/cache/clipping/PNG/SVG. Native macOS selected a project image, undid/redid, imported another image, replaced its shared bytes and undid back to the saved baseline exactly. Save/quit/restart/reopen retained Palette.png on Title and Imported.png on Alpha with empty history. PNG and SVG exports succeeded; all five SVG images are embedded, and the three large payloads match the raw tinted rasters. Native PNG is681×374 and was visually inspected. Evidence: evidence/custom-icons/; generator: examples/render_custom_icons.rs.

Tint rounding, raster interpolation, color profiles, other scales and broader rotated-image sampling remain fidelity work. Native component editing, Windows/Linux runtime and independent review were not performed.


## Button and Dropdown icons

The Icon picker assigns a catalog icon or embedded project image. No icon removes it; Reset restores the default or inherited value. Sizes are16,24,32,48,64 and128 units, independently of text size. Rotation is counterclockwise in90-degree steps and does not change the square footprint. Catalog artwork keeps its aspect ratio; custom images stretch to the square after tinting and rotation. All assignments, size/rotation changes and imports participate in history, component overrides, clipboard resource remapping and storage.

Buttons center the icon/text group by default, support left/right alignment and have an independent Menu arrow toggle. Automatic size includes the icon and spacing; fixed dimensions stay fixed. Insufficient width or height hides the optional icon. Menu arrows use10,12 or24 units according to font-size thresholds28 and60. Dropdown icons enlarge the title row and move an open menu downward; narrow dropdowns clip rather than hide the icon. Normal, focused and disabled states supply the icon color. The normal clean dropdown palette is gray.

Four integration tests cover every size and rotation, automatic/fixed geometry, clipping/visibility, states, menu arrows, history/reset/storage/validation, component ownership, cross-document resource collisions, atomic import and shared exports. Native macOS verified a Button size change16→32 (58×24→74×32),90° rotation, Menu arrow (93×32), Reset (58×24) and Undo. Project-image selection/Undo/Redo and a focused Dropdown size32→64 (103×32→135×64), Undo and Redo were exercised. Save/full quit/restart/reopen retained the assignments and dimensions with empty history. Native PNG/SVG exports retain attribution and three embedded rotated custom images; the838×477 PNG was visually inspected. Evidence: evidence/optional-icons/; generator: examples/render_optional_icons.rs.

Native direct import into these two families and native component-local edits were not separately exercised; automated tests cover those paths and the existing shared picker import flow has List acceptance. Exact typography, selection palette, contours and resampling remain visual acceptance work. The complete suite has141 passing tests; formatting and all-target Clippy with warnings denied pass. No Windows/Linux native check or independent review is claimed.


## Standalone icons and labels

The Icons library category adds Icon and Icon and label. Both use the shared picker, independent size and counterclockwise quarter-turn rotation, project-image import, tinting and exports. A missing standalone icon has a24-unit empty frame; a missing label icon has a48-unit frame. An original rounded outline provides the empty frame and optional border. The border insets artwork4units without changing outer size. Removing the source returns to the fallback size even if a previous size property remains stored.

Icon dimensions come from the selected source size; width/height fields and resize handles cannot resize it. Icon labels allow horizontal resizing only. Bottom placement centers the icon over the text with a2-unit gap; left/right placement uses a3-unit gap. Height derives from icon and text, including an empty text line. An icon that cannot fit the control's width is hidden. Disabled labels use the shared disabled ink. Rich text, text links and whole-control page links use the existing scene/editor paths.

Three integration tests cover sizes/rotation/removal, width-only geometry, boundary visibility, border inset, disabled color, rich web links, attributed SVG/PNG, atomic import/history, component-local styling/reset and copying embedded resources. The full suite has141 passing tests, formatting and all-target Clippy with warnings denied pass. Native macOS verified library insertion/Undo,24→48 square sizing, removal→24/Undo, Bottom→Right/Undo/Redo (48×63→68×48), width47→48 visibility, save/full quit/restart/reopen and PNG/SVG export. Reopened history is empty and saved bytes are unchanged. Both SVG image instances embed identical rotated80×120 payloads at40×40 and48×48 drawing sizes; original source bytes are unchanged. The868×531 PNG was visually inspected. Evidence: evidence/standalone-icons/; example: render_standalone_icons.

Exact typography, outline contours, color-profile/interpolation coverage and broader macros/inline-color/link behavior remain fidelity work. Native direct import and component editing were not repeated for these families; automated tests cover their targets. Windows/Linux native and independent review were not performed. Overall product scope remains open.


## Circular buttons

Circle button is available in Buttons and Forms. New circles are48×48 with an independently sourced plus icon at24. Width and height can be changed; Auto size restores48×48 regardless of text or icon size. The circle stays centered in non-square bounds. Fill defaults to gray, with automatic light/dark content contrast. Selected changes fill, focused changes outline, and disabled changes outline and content ink. The border uses1unit in clean style and3 in sketch; turning it off retains the corresponding inset.

Optional text goes below, left or right. Both icon and label must fit strictly inside the diameter's square bounds: an icon equal to the diameter is hidden. A hidden label recenters the icon. Empty text reserves no line. Rich multiline text and visible links use the shared renderer; hidden labels do not leave interactive link regions. Image sources, rotation, ownership, history, component overrides and exports reuse the existing icon path. Exact typography, broader artwork coverage and all-platform native acceptance remain open.
