# Embedded images

Add an Image from the library, then use **Import image…** in its inspector. PNG, JPEG and GIF are recognized by their file signatures. Images are stored in the project; reopening does not need the original file. The source menu assigns an existing project image. **Remove image** returns that control to its placeholder and retains the asset for reuse.

Importing a different file with a matching filename or filename stem offers **Use new**, **Use existing**, **Keep both**, or **Cancel**. Use new replaces the shared asset data everywhere while retaining its identity. Keep both creates a distinct asset. Automatic control dimensions follow the image; explicit width/height remain fixed. Source assignment inside a component is local to that instance. Import, assignment, clearing and shared replacement participate in Undo/Redo. Import plus assignment is one product history entry.

## Crop and rotation

**Rotate 90°** turns the image counterclockwise around its visible center. Its crop turns with it. **Crop image** opens eight handles on the canvas; drag a handle to adjust an edge or drag inside the selection to move it. The first selection covers80% of each dimension. Apply crop or Enter commits one Undo step. Cancel crop or Escape leaves the document unchanged. Other editing controls are disabled during the preview.

**Remove crop** restores the full image size at the current visible top-left. Auto-size preserves that visible position. At90°/270° with a crop, Auto-size uses the full rotated natural dimensions as the visible size and retains the crop. Transforms never rewrite the stored image bytes. Component children can crop and rotate locally; Reset local changes restores inheritance. An explicit crop removal is distinct from inheriting a source crop. Existing projects without a crop override keep their previous inheritance.

## Storage and drawing

Asset stores an ID, display name, actual media type and bytes. It never stores an absolute source path. Import identifies encoded bytes independently of the name. Opaque PNG and the first opaque GIF frame become JPEG at quality92; transparent images use PNG. An unchanged transparent PNG or JPEG retains its input bytes. Images above2×1024×1024 pixels are resized with their aspect ratio preserved. JPEG encoding and resampling results can vary from other implementations.

Shared scene data contains positioned raster images. Canvas and thumbnails use a retained texture; SVG embeds a PNG data URL; PNG export renders the same scene. Transparency and ancestor crop clips are preserved. Repeated use of an asset shares decoded data within a scene. Automatic placeholders are77×79. Invalid embedded payloads render an Image unavailable placeholder rather than preventing the rest of the page from rendering.

Reads and decoding are bounded:32MiB encoded input,32768 maximum dimension,64×1024×1024 decoded pixels,256MiB decoder allocation, and128MiB decoded pixels per scene. Resource and serialized project limits are checked before an import transaction is accepted; a failed transaction restores document and allocator state. Existing project limits remain32MiB raw assets and64MiB encoded document. Images which exceed the scene budget also use the unavailable placeholder.

## Verification and remaining work

Nine image tests cover transparent canvas/scene data and export pixel colors, SVG embedding, source-independent persistence, shared identity and replacement/history, removal/reassignment, Keep both/Use existing, fixed dimensions, local component source overrides, signature-based PNG/JPEG/GIF import, opaque conversion, large-image reduction, invalid input, transactional rollback and locks. Transform cases cover center-preserving cropped rotation, orientation pixels, visible reset anchors, normalized crop resizing, automatic size, component inherit/clear/region/reset/detach, old patch loading and crop validation. The pre-existing unsupported automatic-layout guard now exercises line charts because automatic Image layout is implemented.

Native macOS: imported160×100 transparent PNG, duplicated/moved it, cleared one source and undid, replaced the shared asset with100×160 opaque PNG, undid/redid, saved, quit, temporarily removed both source paths, restarted and reopened. Both images rendered at100×160; sources were restored unchanged. PNG and SVG exported through native dialogs. Local evidence lives under evidence/images/.

Additional native macOS checks in evidence/image-transforms/: rotated100×160 to160×100 around its center, cropped by dragging the left handle to98×80, removed crop retaining the visible anchor, undid, changed a preview to78×80 and cancelled with saved bytes unchanged. Rotated the committed crop to80×98, saved, quit/restarted/reopened, and exported PNG/SVG. The first UI build exposed a panel-layout regression; it was corrected and the complete native sequence used the corrected build. Component crop behavior currently has automated coverage, not a separate native component sequence.

Pending: image border/filter styling, DPI scaling and EXIF orientation metadata, multi-frame GIF acceptance, native JPEG/GIF imports, exact resampling/encoding fidelity, resource deletion/management, cross-document image clipboard acceptance, native component crop/odd-pixel geometry checks, native Windows/Linux checks and large-document/cache profiling. Only quarter-turn image orientations are exposed. PNG exports currently use the existing white page background. This checkpoint does not complete image fidelity or platform acceptance.
