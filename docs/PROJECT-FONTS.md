# Project fonts

Choose **Project font…** in the toolbar. Select the regular, bold, italic and bold italic TTF or OTF files, then choose **Apply font**. The name can be edited before applying. Optional font notices are saved with the project and included in its SVG exports.

Each project has one font family. All four faces are required, so each style has an explicit face. Font files may be chosen separately, including the same file for more than one role when desired. Files must contain a single outline font face and be no larger than 4 MB each; collections are unsupported. Invalid files are rejected before they enter the project.

Font changes affect automatic control sizes, wrapped text, library previews, page thumbnails, presentation, canvas drawing and PNG/SVG export. Fixed control dimensions remain fixed; text wrapping and item visibility can change. Font changes are undoable. **Use bundled Comic Neue** restores the default and can also be undone.

Fonts are embedded in the `.wireframe` file as base64 data. Opening the project on another computer does not require installing its chosen font. Existing projects without a font resource retain bundled Comic Neue. Normal project size limits still apply. Imported fonts remain user project data; the application does not install them or change system font preferences.

## Rendering and validation

The document holds an immutable validated resource, shared by history snapshots and cached scenes. Both the canvas parser and export parser validate each face on import or document decoding. Measurement and the desktop atlas use the same face assignments. SVG embeds the four faces with an internal family alias, and PNG registers those explicit roles independently of font metadata. This supports user-assigned roles even when a file's internal weight/style differs. The resource survives saving, recovery, reopening and undo/redo.

Glyphs absent from the selected face use the existing bundled canvas fallback fonts. SVG viewers may use their own fallback fonts for missing glyphs; exact fallback, emoji, complex-script shaping and arbitrary-font parity are not yet accepted. Imported glyph coverage and the four selected styles are the portability scope of this feature. Native Windows/Linux verification remains pending.
