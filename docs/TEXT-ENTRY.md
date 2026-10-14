# Text entry

Freeform input uses a source-based editor: control text, project/page/component names, notes, Quick Add, icon search, segment URLs, and project-font names/notices. Numeric steppers retain their framework editing behavior. `text_entry::Draft` owns the original UTF-8, selection, and undo history. `text_entry::layout` shapes and rasterizes through COSMIC Text; its stock editing model is not used. `text_entry::widget` connects pointer, keyboard, clipboard, composition, and accessibility events to Draft.

Grapheme ranges map original source to visual cells. Caret affinity distinguishes the two sides of a directional or wrapped boundary. Multiple glyphs belonging to one grapheme share one cell; ligatures spanning multiple graphemes divide their advance. Trimmed wrapping spaces retain zero-width source cells. Selection highlights follow visual cells, while clipboard text retains logical source order. The input layout and raster use the same physical scale and viewport.

The editor supports complete-grapheme deletion, reversible replacement, visual arrow movement, vertical movement, word and line deletion, selection, clipboard, and scrolling. IME preedit is temporary and its commit is one undo transaction. Accessibility text runs preserve source and line endings; selection actions map run positions back to UTF-8. Very long combining sequences are split at scalar boundaries to fit accessibility length limits, while editing still clamps to grapheme boundaries.

`TextInput` supplies single-line and multiline modes, requested width/rows, stable identity, one-time autofocus and placeholders. Single-line mode disables soft wrapping, scrolls horizontally, retains pasted line breaks, and exposes Enter/Up/Down to the containing form. Enter commits names; Quick Add uses vertical arrows and Enter for suggestions. Composition candidate keys stay inside the field. Registered input focus distinguishes text editing from buttons and popups when routing document shortcuts.

The existing UI font is loaded with system font fallback. No project font is installed globally. Layout accepts at most 1 MiB of text, finite bounded dimensions, and raster images up to 8 million pixels. Draft history is bounded separately. Glyph caches and full-document shaping still need performance acceptance for large documents.

## Verification and limits

Model, layout, and widget tests cover mixed directions, joined emoji, combining marks, line endings, source selection, clipboard, undo, wrapping, viewport rasterization, accessibility, and simulated IME composition. Native macOS checks cover click-to-source insertion, whole-grapheme deletion and undo, accessibility replacement, focus traversal, Apply/Cancel, inspector editing, document undo, and save/full-quit/reopen/resave.

Native input-method composition, screen-reader interaction, Windows/Linux runtime, complex bidirectional word navigation, typing-history coalescence, and large-document performance remain unverified. The caret currently does not blink and the field has no scrollbar thumb. Static toolbar/page captions still use the previous text renderer, and long project titles can displace toolbar actions; this display work remains open. Canvas text rendering is separate from input text layout.
