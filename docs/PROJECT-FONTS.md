# Project fonts

New projects use **Wireframe Draft**, a four-style drawing font derived from Comic Neue under the SIL Open Font License. Its font files are saved inside each new project, so later font updates do not change that document.

Choose **Font and colors…** in the **File** menu. Select the regular, bold, italic and bold italic TTF or OTF files, then choose **Apply font**. The name can be edited before applying. Optional font notices are saved with the project and included in its SVG and PDF exports.

Each project has one font family. All four faces are required, so each style has an explicit face. Font files may be chosen separately, including the same file for more than one role when desired. Files must contain a single outline font face and be no larger than 4 MB each; collections are unsupported. Invalid files are rejected before they enter the project.

Font changes affect automatic control sizes, wrapped text, library previews, page thumbnails, presentation, canvas drawing and PNG/SVG/PDF export. Fixed control dimensions remain fixed; text wrapping and item visibility can change. Font changes are undoable. **Use Wireframe Draft** selects the current bundled drawing font. **Use Comic Neue** selects the earlier bundled family. Both changes can be undone.

Fonts are embedded in the `.wireframe` file as base64 data. Opening the project on another computer does not require installing its chosen font. Existing projects without a font resource retain bundled Comic Neue. Normal project size limits still apply. Imported fonts remain user project data; the application does not install them or change system font preferences.

## Rendering and validation

The document holds an immutable validated resource, shared by history snapshots and cached scenes. Both the canvas parser and export parser validate each face on import or document decoding. Measurement and the desktop atlas use the same face assignments. SVG embeds the four faces with an internal family alias, and PNG registers those explicit roles independently of font metadata. This supports user-assigned roles even when a file's internal weight/style differs. The resource survives saving, recovery, reopening and undo/redo.

Glyphs absent from the selected face use the selected platform backend's installed fallback fonts. Positioned outlines and bitmap ink are retained in SVG/PNG, so those exports do not ask a viewer to select a fallback font again. Documents retain their four selected primary faces; system fallback availability can still differ across machines. See TEXT-BACKENDS.md for architecture and limits. Arbitrary-font parity and native Windows/Linux acceptance remain open.

## Rebuilding the bundled drawing font

The original Comic Neue sources and their license remain in `assets/fonts/comic-neue`. The four derived faces, retained OFL, modification notice and derivation manifest are in `assets/fonts/wireframe-draft`. No system fonts are installed.

Use an isolated Python environment with `scripts/font-requirements.txt`, then run `python scripts/build_fonts.py --check` to verify every generated file byte for byte. Omit `--check` to regenerate the family. This optional asset build is separate from the normal Rust build, which embeds the committed font files.

The builder preserves accented base-letter outlines through shared components, gives detached bold marks clearance and keeps attached marks in place. Four distinct faces retain the source character coverage. Revision 1.004 gives plus and multiplication a consistent lowercase-height design and refines letter, digit, punctuation and word-space advances for each role. Ordinary and nonbreaking spaces retain equal widths. The math outlines come from the retained Comic Neue Regular/Italic sources, with a separate bold expansion. Other outlines and inherited positioning remain unchanged. Revision 1.003 added uppercase/lowercase G with breve (U+011E/U+011F), using the existing licensed base letters and case-appropriate accents. It retains the revision 1.002 correction for lowercase t with cedilla (U+0163) and uppercase T with cedilla (U+0162). A composition rule handles canonically equivalent breve sequences, including an intervening below-letter mark, before the inherited accent-context rules. Other above-letter accents remain order-sensitive. Pair positioning, combining-mark anchors and applicable context coverage are inherited from the corresponding same-case letters. Existing glyph outlines and advances are unchanged. This revision applies when creating a project or explicitly choosing the current drawing font; opening or saving an existing project preserves its embedded font bytes. Font metadata, clipping bounds, canonical accent shaping and constrained wrapping are checked during acceptance. Native Windows/Linux text rendering remains a separate acceptance requirement.

## Scoped revision 1.003 acceptance

CoreText and portable shaping checks cover both breve forms in four roles, adjacent letters, additional marks and accent ordering. Native macOS checks cover a contact-name edit, Undo/Redo, Save, process restart/reopen and matching PNG/PDF exports. The older project fixture remains byte-identical after native Save, retaining its earlier embedded fonts. Broader font metrics, physical-display appearance and native Windows/Linux rendering remain separate acceptance work.

Revision 1.004 adds native and portable checks for operator size and ordinary/nonbreaking-space raster equality across four styles and three sizes. The generator reproduces all outputs byte-for-byte. A native-created project embeds the new four faces, while saving the prior-revision scenario leaves its complete file unchanged. Remaining narrow-field wrapping differences stay under review; this update does not imply exact typeface parity or native Windows/Linux acceptance.

## Canvas pixel alignment

Positioned glyphs are rasterized at canvas zoom times display pixels per point. The texture origin is rounded to a physical-pixel boundary, matching the GUI text convention and preventing a second smoothing pass from fractional texture placement. The resulting display adjustment is at most half a physical pixel per axis. Font shaping, control layout, caret/hit geometry, raster density and export positions are unchanged. Panning reuses the cached raster; no new glyph shaping is introduced.

Headless paint tests cover five display densities and six zoom factors, including fractional scales, with one texel per output pixel, bounded translation and cache reuse. Hardware verification remains separate from this simulated scale matrix. Other font/capture differences are not ruled out by fixing texture placement.
