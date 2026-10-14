# Built-in icons

Lists support one optional icon per raw text row, including the header. Expand **Row icons** in the inspector, choose an icon, search its name or keywords, and filter by Solid, Regular or Brands. No icon and the remove button clear one assignment; Reset restores the default or inherited component property. Search and filter clicks keep the picker open. Assignment and removal are undoable.

The offline catalog contains 2,060 Font Awesome Free 6.7.2 variants. Artwork comes directly from the official public upstream at the immutable commit recorded in assets/icons/font-awesome-free/PROVENANCE.json. scripts/import_icons.py verifies both input checksums and regenerates the compact, sorted catalog. Paths and dimensions are unchanged; metadata is compacted and only free variants are selected. Artwork retains CC BY 4.0 attribution and the upstream license; original application code remains GPL-3.0-only. THIRD_PARTY_NOTICES.md includes the attribution, and exported SVG metadata and PNG Attribution iTXt carry it when the scene contains icons.

A List with any icon reserves one shared column: text starts font size+16 units from the left, and automatic width gains that amount. Each icon occupies a font-size square at x+8 and follows the row text's vertical origin. Paths fit proportionally and center within that square. Icons use the row's base ink; inline disabled text does not dim the icon. Control and ancestor clipping apply. Fixed control dimensions remain fixed. Removing the final icon removes the shared column on automatic controls.

Canvas, thumbnails, library previews and exports consume the same icon scene entries. SVG contains vector paths; PNG rasterizes those paths. Canvas textures use bounded resolution, at most 128 cache entries and 16 MiB per context, with only visible icons painted. The picker virtualizes result rows. These bounds do not replace large-document profiling.

Row assignments use typed nullable values, not embedded path data. Text edits follow the same raw-label occurrence mapping as row links, including transaction baselines, duplicate labels and same-count replacements. Component-local changes leave source definitions untouched; Reset resumes inheritance. Syntactically valid unknown IDs survive storage and display an outlined placeholder, allowing later catalogs to resolve them. Invalid IDs and oversized arrays fail validation.

## Verification and remaining work

Four integration tests cover all 2,060 icon rasters/search/aspect ratios, row mapping/history/validation/storage, shared geometry/colors/clipping/vector and PNG attribution, and component-local reset/copy behavior. The complete suite has 130 passing tests. Formatting and Clippy with warnings denied pass.

Native macOS acceptance covers camera search, Regular heart filtering, assignment/Undo/Redo, Reset shrinking an automatic List from 142 to 102 units and Undo restoring 142, text-row reordering with assignments retained, save/quit/restart/reopen, and PNG/SVG exports. The reopened fixture has Camera on Title, Asterisk on Bravo, Cloud on Alpha and Asterisk on Disabled; Undo/Redo are empty after restart. Export metadata and all three distinct icon path sets were checked, and the PNG was visually inspected. Local evidence: evidence/list-icons/; fixture generator: examples/render_list_icons.rs.

Custom image icons, icon properties on other control families, broader artwork coverage, typography/contour fidelity and Windows/Linux native acceptance remain open. No independent review is claimed.
