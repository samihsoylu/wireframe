# Project colors

Choose **Font and colors…** in the toolbar to edit selection and link colors. Use the swatch picker or enter six hexadecimal digits, with an optional `#`. **Apply colors** changes both colors together and closes the dialog. Invalid entries cannot be applied. Cancel leaves the project and its defaults unchanged.

The selection color controls selected rows and buttons, focused field borders and other selection accents. The link color controls ordinary links. Selected content uses white automatic text and links on dark backgrounds. At weighted RGB brightness 153 and above, normal ink and the configured link color are used. Selected menu/dropdown links use black on light selections. Explicit text colors and inline color markup are preserved; disabled content remains muted. Existing opacity and control-family rules still apply.

Changes affect all project pages and components, the canvas, page thumbnails, library previews, presentation and PNG/SVG/PDF output. Undo/Redo changes the project palette as one operation and never rewrites individual controls.

Applying colors also saves defaults for new projects in the local `appearance-settings.json` file alongside the application's other settings. Undo restores project colors but leaves the new-project defaults as last applied. A new project starts clean with those defaults. Opening a saved project uses its own colors; older projects use the built-in defaults. No system appearance setting is changed.

If saving local defaults fails, the document change remains undoable and the app reports the failure. Unreadable settings are reported and left unchanged until a new value is explicitly applied. The defaults file has a 4 KiB read limit.

## Acceptance scope

Regression coverage includes older documents, custom-color round trips, rejected malformed colors, selected control contrast at the 152/153 boundary, explicit/disabled text, multiple pages, undo/redo, separate default persistence, write failures and library-preview invalidation. Native desktop interaction and export acceptance are recorded in SESSION-SPEC.md. Native Windows/Linux verification remains pending.
