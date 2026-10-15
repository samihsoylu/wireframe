# PDF export

## Required product behavior

Export all pages, the current page, or an explicitly chosen set in navigator order. Include alternate versions consistently with their page identity and exported display names. Offer content-sized pages and shrink-to-paper pages. Keep PNG and SVG export available.

Content-sized pages use the visible content bounds with optional 10-point margins. Add 26 points above the artwork when names or page numbers are enabled. Included nonempty notes reserve their measured text height plus a 10-point gap below the artwork. Measure notes at the page width minus horizontal margins. The artwork retains its natural size.

Paper choices are A3, A4, A5, Letter, Legal and Tabloid. Orientation is portrait, landscape or best fit per page; best fit follows the artwork's aspect ratio. Use 10-point margins. Reserve header and notes height before computing a uniform scale. Cap scale at 1, so small artwork is never enlarged. Place artwork at the top-left of the available content area. Headers and notes keep their own text sizes rather than inheriting artwork scaling.

Names, page numbers, notes and link hints are independent options. Internal links resolve against the pages actually exported: prefer the official page, then an included alternate. Omit annotations and hints for targets absent from the export. Keep supported web links. Transform link rectangles with the artwork and preserve clipping and overlap exclusions. Screen link hints use a translucent region; paper hints use a small arrow at the appropriate link boundary.

The PDF must contain vector artwork and extractable text wherever supported, embedded images/fonts as needed, and real link annotations. It must remain usable offline. Do not substitute one page-sized bitmap for the document. Preserve the shared renderer's glyph positions, crops, masks, opacity and image transforms. Exact text extraction, mixed scripts and fallback behavior need explicit acceptance; visible text alone does not prove search/copy correctness.

## Architecture and current status

PDF export is not yet available in the app. Keep page selection and layout, scene conversion, PDF encoding and destination writing separate. Reuse the positioned Scene rather than building another control renderer. Retain existing PNG/SVG behavior until their explicit options are integrated and verified.

A backend feasibility probe can reuse primary-font glyph IDs, cluster ranges and positions with an embedded font. Shaped runs now retain primary font bytes or the actual native fallback face tables, face index and non-default variation coordinates. Detached collection faces use index zero. Font serialization is lazy, cache accounting reserves its eventual bytes, and live spans share equivalent native fallback faces through bounded weak references. Bitmap runs and unavailable/transformed faces retain an explicit ink-only reason. The macOS adapter has scoped primary, Hebrew, Arabic, Chinese and color-emoji verification; general cross-platform and synthetic-style fidelity remain unverified. Never apply a primary font to glyph IDs from a fallback face. Keep exact rendered ink available for unsupported glyphs while explicitly testing text extraction and visual output.

The export dialog must have one authoritative option state, preserve selection/order and report output success only after the complete file is written. Use the existing atomic destination writer. Failures must leave the project and any prior destination intact. Long exports must not freeze editing or report success early; capture a coherent document snapshot.

## Acceptance

- Content-sized pages, with and without margins, and paper pages with both large and small artwork.
- Independent names/numbers/notes/hints; note measurement, wrapping, excessive notes and empty content with finite valid page dimensions.
- Current/all/set selection, page order, alternates, internal target omission and real URI/page annotations.
- Shared geometry for nested crops, masks, opacity, images and rotated content; link rectangles follow the same transform.
- Extractable primary text, combining marks, ligatures, RTL/mixed scripts, fallback fonts and color glyphs. Compare rendered pages and extracted content; document normalization or viewer-specific limits.
- Native export/save/error flow, reopen in a PDF reader, offline operation, destination preservation and macOS/Windows/Linux acceptance.

A passing backend probe does not satisfy the integrated export requirement. The integrated export implementation and its acceptance checks remain outstanding. Font-source tests and rendered backend probes establish only their stated scope. Mixed-direction extraction order and searchable bitmap emoji remain unresolved.
