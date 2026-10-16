# Project fonts

New projects use **Wireframe Draft**, a four-style drawing font derived from Comic Neue under the SIL Open Font License. Its font files are saved inside each new project, so later font updates do not change that document.

Choose **Font and colors…** in the toolbar. Select the regular, bold, italic and bold italic TTF or OTF files, then choose **Apply font**. The name can be edited before applying. Optional font notices are saved with the project and included in its SVG and PDF exports.

Each project has one font family. All four faces are required, so each style has an explicit face. Font files may be chosen separately, including the same file for more than one role when desired. Files must contain a single outline font face and be no larger than 4 MB each; collections are unsupported. Invalid files are rejected before they enter the project.

Font changes affect automatic control sizes, wrapped text, library previews, page thumbnails, presentation, canvas drawing and PNG/SVG/PDF export. Fixed control dimensions remain fixed; text wrapping and item visibility can change. Font changes are undoable. **Use Wireframe Draft** selects the current bundled drawing font. **Use Comic Neue** selects the earlier bundled family. Both changes can be undone.

Fonts are embedded in the `.wireframe` file as base64 data. Opening the project on another computer does not require installing its chosen font. Existing projects without a font resource retain bundled Comic Neue. Normal project size limits still apply. Imported fonts remain user project data; the application does not install them or change system font preferences.

## Rendering and validation

The document holds an immutable validated resource, shared by history snapshots and cached scenes. Both the canvas parser and export parser validate each face on import or document decoding. Measurement and the desktop atlas use the same face assignments. SVG embeds the four faces with an internal family alias, and PNG registers those explicit roles independently of font metadata. This supports user-assigned roles even when a file's internal weight/style differs. The resource survives saving, recovery, reopening and undo/redo.

Glyphs absent from the selected face use the selected platform backend's installed fallback fonts. Positioned outlines and bitmap ink are retained in SVG/PNG, so those exports do not ask a viewer to select a fallback font again. Documents retain their four selected primary faces; system fallback availability can still differ across machines. See TEXT-BACKENDS.md for architecture and limits. Arbitrary-font parity and native Windows/Linux acceptance remain open.

## Rebuilding the bundled drawing font

The original Comic Neue sources and their license remain in `assets/fonts/comic-neue`. The four derived faces, retained OFL, modification notice and derivation manifest are in `assets/fonts/wireframe-draft`. No system fonts are installed.

Use an isolated Python environment with `scripts/font-requirements.txt`, then run `python scripts/build_fonts.py --check` to verify every generated file byte for byte. Omit `--check` to regenerate the family. This optional asset build is separate from the normal Rust build, which embeds the committed font files.

The builder preserves accented base-letter outlines through shared components, gives detached bold marks clearance and keeps attached marks in place. Four distinct faces retain the source character coverage. Font metadata, clipping bounds, canonical accent shaping and constrained wrapping are checked during acceptance. Native Windows/Linux text rendering remains a separate acceptance requirement.

## Canvas pixel alignment

Positioned glyphs are rasterized at canvas zoom times display pixels per point. The texture origin is rounded to a physical-pixel boundary, matching the GUI text convention and preventing a second smoothing pass from fractional texture placement. The resulting display adjustment is at most half a physical pixel per axis. Font shaping, control layout, caret/hit geometry, raster density and export positions are unchanged. Panning reuses the cached raster; no new glyph shaping is introduced.

Headless paint tests cover five display densities and six zoom factors, including fractional scales, with one texel per output pixel, bounded translation and cache reuse. Hardware verification remains separate from this simulated scale matrix. Other font/capture differences are not ruled out by fixing texture placement.
