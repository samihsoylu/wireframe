# PDF export

## Required product behavior

Export all pages, the current page, or an explicitly chosen set in navigator order. Include alternate versions consistently with their page identity and exported display names. Offer content-sized pages and shrink-to-paper pages. Keep PNG and SVG export available.

Content-sized pages use the visible content bounds with optional 10-point margins. Add 26 points above the artwork when names or page numbers are enabled. Included nonempty notes reserve their measured text height plus a 10-point gap below the artwork. Measure notes at the page width minus horizontal margins. The artwork retains its natural size.

Paper choices are A3, A4, A5, Letter, Legal and Tabloid. Orientation is portrait, landscape or best fit per page; best fit follows the artwork's aspect ratio. Use 10-point margins. Reserve header and notes height before computing a uniform scale. Cap scale at 1, so small artwork is never enlarged. Place artwork at the top-left of the available content area. Headers and notes keep their own text sizes rather than inheriting artwork scaling.

Names, page numbers, notes and link hints are independent options. Internal links resolve against the pages actually exported: prefer the official page, then an included alternate. Omit annotations and hints for targets absent from the export. Keep supported web links. Transform link rectangles with the artwork and preserve clipping and overlap exclusions. Screen link hints use a translucent region; paper hints use a small arrow at the appropriate link boundary.

The PDF must contain vector artwork and extractable text wherever supported, embedded images/fonts as needed, and real link annotations. It must remain usable offline. Do not substitute one page-sized bitmap for the document. Preserve the shared renderer's glyph positions, crops, masks, opacity and image transforms. Exact text extraction, mixed scripts and fallback behavior need explicit acceptance; visible text alone does not prove search/copy correctness.

## Architecture and current status

File > Export PDF is available. Page selection and layout, scene conversion, PDF encoding and destination writing are separate modules. Reuse the positioned Scene rather than building another control renderer. Retain existing PNG/SVG behavior until their explicit options are integrated and verified.

A backend feasibility probe can reuse primary-font glyph IDs, cluster ranges and positions with an embedded font. Shaped runs now retain primary font bytes or the actual native fallback face tables, face index and non-default variation coordinates. Detached collection faces use index zero. Font serialization is lazy, cache accounting reserves its eventual bytes, and live spans share equivalent native fallback faces through bounded weak references. Bitmap runs and unavailable/transformed faces retain an explicit ink-only reason. The macOS adapter has scoped primary, Hebrew, Arabic, Chinese and color-emoji verification; general cross-platform and synthetic-style fidelity remain unverified. Never apply a primary font to glyph IDs from a fallback face. Keep exact rendered ink available for unsupported glyphs while explicitly testing text extraction and visual output.

The Scene encoder is implemented with Krilla 0.8.2. It accepts positioned pages and resolved links, emits vector primitives/icons, embedded images, clips/masks and transparency, and reuses retained native glyph positions and font sources. Legacy text uses the SVG resolver and its positioned glyphs. Font resolvers are reused across spans. Failed drawing operations unwind PDF graphics state before returning an error. This layer does not mutate the project or write destinations.

All 86 catalog controls encode in both styles. A two-page visual check covers full-size and half-size transforms, negative scene origins, images, masks, transparency, rotated icons, native and legacy text and internal/web annotations. Six fonts embed and two small raster images remain; pages are not flattened. PDFKit preserves the tested mixed-script logical order, while other readers reorder directional runs. Outline-only runs and bitmap emoji preserve visible ink but are not reliably searchable. These checks do not establish catalog-wide visual parity or general variable-font acceptance.

The export dialog has one option state. Cancelling the native destination panel retains those options; reopening a completed export starts with defaults. Explicit selection is emitted in navigator order. A background worker owns an immutable project snapshot and uses the existing atomic destination writer after encoding succeeds. Success is reported only after writing completes. Closing waits for an active export; export failure cancels the deferred close. The worker has no cancellation control. Layout errors preserve any prior destination. Excessive notes are rejected with guidance to choose larger/content-sized pages or omit notes. No silent truncation is used.

Generic whole-control links are included in the shared Scene, split around specialized links and occluded by later controls. Paper hint arrows retain the original link bounds so a split annotation does not duplicate its hint. Legacy PDF text uses point-sized text operators with a compensating transform; this preserves glyph placement and prevents tested readers from concatenating header/note lines.

## Acceptance

- Content-sized pages, with and without margins, and paper pages with both large and small artwork.
- Independent names/numbers/notes/hints; note measurement, wrapping, excessive notes and empty content with finite valid page dimensions.
- Current/all/set selection, page order, alternates, internal target omission and real URI/page annotations.
- Shared geometry for nested crops, masks, opacity, images and rotated content; link rectangles follow the same transform.
- Extractable primary text, combining marks, ligatures, RTL/mixed scripts, fallback fonts and color glyphs. Compare rendered pages and extracted content; document normalization or viewer-specific limits.
- Native export/save/error flow, reopen in a PDF reader, offline operation, destination preservation and macOS/Windows/Linux acceptance.

Scoped macOS acceptance covers all/current export, option retention after save-panel cancellation, content sizing and A4 best fit, native file writing, reopening in Preview and following an internal page link. The final native A4 output matches the tested planner output byte for byte. Both pages were rendered and inspected. Five planner/worker tests cover selection/alternates, all paper orientations, empty pages, overflow refusal, immutable snapshots, destination preservation and split link hints. A subsequent native A5 overflow check reports a clear error and leaves an existing PDF byte-identical. Failure status and cancellation of deferred quit have a focused application test. The complete suite passes 567 tests.

Remaining acceptance: native close-during-export flow, sustained editing during a large export, native Windows/Linux, broader typography/variable fonts, mixed-direction extraction and searchable bitmap emoji. Headers currently use the bundled proportional face without a separate bold face; exact header typography remains open. Initial cancellation/current-page native checks used the same UI before the subsequent text-encoding and hint corrections; A4 export and reader checks used the corrected rendering bundle; the final status-only follow-up was rebuilt and used for the native failure check. No general cross-platform, catalog-wide visual or performance acceptance is claimed.
