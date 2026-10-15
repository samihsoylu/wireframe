# Project fonts

New projects use **Wireframe Draft**, a four-style drawing font derived from Comic Neue under the SIL Open Font License. Its font files are saved inside each new project, so later font updates do not change that document.

Choose **Project font…** in the toolbar. Select the regular, bold, italic and bold italic TTF or OTF files, then choose **Apply font**. The name can be edited before applying. Optional font notices are saved with the project and included in its SVG and PDF exports.

Each project has one font family. All four faces are required, so each style has an explicit face. Font files may be chosen separately, including the same file for more than one role when desired. Files must contain a single outline font face and be no larger than 4 MB each; collections are unsupported. Invalid files are rejected before they enter the project.

Font changes affect automatic control sizes, wrapped text, library previews, page thumbnails, presentation, canvas drawing and PNG/SVG/PDF export. Fixed control dimensions remain fixed; text wrapping and item visibility can change. Font changes are undoable. **Use Wireframe Draft** selects the current bundled drawing font. **Use Comic Neue** selects the earlier bundled family. Both changes can be undone.

Fonts are embedded in the `.wireframe` file as base64 data. Opening the project on another computer does not require installing its chosen font. Existing projects without a font resource retain bundled Comic Neue. Normal project size limits still apply. Imported fonts remain user project data; the application does not install them or change system font preferences.

## Rendering and validation

The document holds an immutable validated resource, shared by history snapshots and cached scenes. Both the canvas parser and export parser validate each face on import or document decoding. Measurement and the desktop atlas use the same face assignments. SVG embeds the four faces with an internal family alias, and PNG registers those explicit roles independently of font metadata. This supports user-assigned roles even when a file's internal weight/style differs. The resource survives saving, recovery, reopening and undo/redo.

Glyphs absent from the selected face use the existing bundled canvas fallback fonts. SVG viewers may use their own fallback fonts for missing glyphs; exact fallback, emoji, complex-script shaping and arbitrary-font parity are not yet accepted. Imported glyph coverage and the four selected styles are the portability scope of this feature. Native Windows/Linux verification remains pending.

## Rebuilding the bundled drawing font

The original Comic Neue sources and their license remain in `assets/fonts/comic-neue`. The four derived faces, retained OFL, modification notice and derivation manifest are in `assets/fonts/wireframe-draft`. No system fonts are installed.

Use an isolated Python environment with `scripts/font-requirements.txt`, then run `python scripts/build_fonts.py --check` to verify every generated file byte for byte. Omit `--check` to regenerate the family. This optional asset build is separate from the normal Rust build, which embeds the committed font files.

The builder preserves accented base-letter outlines through shared components, gives detached bold marks clearance and keeps attached marks in place. Four distinct faces retain the source character coverage. Font metadata, clipping bounds, canonical accent shaping and constrained wrapping are checked during acceptance. Native Windows/Linux text rendering remains a separate acceptance requirement.
