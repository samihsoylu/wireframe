# Embedded images

Add an Image from the library, then use **Import image…** in its inspector. PNG, JPEG and GIF are recognized by their file signatures. Images are stored in the project; reopening does not need the original file. The source menu assigns an existing project image. **Remove image** returns that control to its placeholder and retains the asset for reuse.

Importing a different file with a matching filename or filename stem offers **Use new**, **Use existing**, **Keep both**, or **Cancel**. Use new replaces the shared asset data everywhere while retaining its identity. Keep both creates a distinct asset. Automatic control dimensions follow the image; explicit width/height remain fixed. Source assignment inside a component is local to that instance. Import, assignment, clearing and shared replacement participate in Undo/Redo. Import plus assignment is one product history entry.

## Data and rendering

Asset stores an ID, display name, actual media type and bytes. It never stores an absolute source path. Import identifies encoded bytes independently of the name. Opaque PNG and the first opaque GIF frame become JPEG at quality92; transparent images use PNG. An unchanged transparent PNG or JPEG retains its input bytes. Images above2×1024×1024 pixels are resized with their aspect ratio preserved. JPEG encoding and resampling results can vary from other implementations.

Shared scene data contains positioned raster images. Canvas and thumbnails use a retained texture; SVG embeds a PNG data URL; PNG export renders the same scene. Transparency and ancestor crop clips are preserved. Repeated use of an asset shares decoded data within a scene. Automatic placeholders are77×79. Invalid embedded payloads render an Image unavailable placeholder rather than preventing the rest of the page from rendering.

Reads and decoding are bounded:32MiB encoded input,32768 maximum dimension,64×1024×1024 decoded pixels,256MiB decoder allocation, and128MiB decoded pixels per scene. Resource and serialized project limits are checked before an import transaction is accepted; a failed transaction restores document and allocator state. Existing project limits remain32MiB raw assets and64MiB encoded document. Images which exceed the scene budget also use the unavailable placeholder.

## Verification and remaining work

Five image tests cover transparent canvas/scene data and export pixel colors, SVG embedding, source-independent persistence, shared identity and replacement/history, removal/reassignment, Keep both/Use existing, fixed dimensions, local component source overrides, signature-based PNG/JPEG/GIF import, opaque conversion, large-image reduction, invalid input, transactional rollback and locks. The pre-existing unsupported automatic-layout guard now exercises line charts because automatic Image layout is implemented.

Native macOS: imported160×100 transparent PNG, duplicated/moved it, cleared one source and undid, replaced the shared asset with100×160 opaque PNG, undid/redid, saved, quit, temporarily removed both source paths, restarted and reopened. Both images rendered at100×160; sources were restored unchanged. PNG and SVG exported through native dialogs. Local evidence lives under evidence/images/.

Pending: image-specific crop/rotation UI and geometry, image border/filter styling, DPI scaling and EXIF orientation metadata, multi-frame GIF acceptance, native JPEG/GIF imports, exact resampling/encoding fidelity, resource deletion/management, cross-document image clipboard acceptance, native Windows/Linux checks and large-document/cache profiling. PNG exports currently use the existing white page background. This checkpoint does not complete image fidelity or platform acceptance.
