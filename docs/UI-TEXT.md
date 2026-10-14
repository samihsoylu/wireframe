# Interface captions

`ui_text` paints bounded, shaped captions for project titles, page tiles and lists, component lists/status, layers, and presentation navigation. It shares the system-fallback text engine used by input fields. Canvas text remains a separate rendering path.

Labels retain their ordinary label role. Selectable captions retain button interaction and selected state. Accessibility receives the complete source name even when its visual preview is shortened. This avoids introducing editable fields merely to display complex scripts.

The preview converts line breaks to spaces, caps layout work at 4096 graphemes and 64 KiB, and reshapes candidate prefixes with an ellipsis. Source values are not changed. A context-local texture cache retains at most 128 entries and 16 MiB of image/source data, evicting least recently used entries. The shared font engine has separate caches; the caption texture bound is not a bound on all font memory.

Project titles reserve toolbar action space and keep the dirty marker separate from truncation. Presentation page captions use a horizontal scroll area. Thumbnail captions remain within the tile.

Tests cover grapheme-safe truncation, original source and noneditable accessibility roles, long input, cache eviction, and toolbar action bounds at 860, 1000 and 1600 points. Native macOS validates mixed-direction/emoji names, thumbnail/list/presentation rendering, interactive presentation navigation and saved source preservation. Pixel-identical typography, native Windows/Linux, screen-reader interaction, and all other dynamic captions are not established. Page-link combo boxes and ancillary dialogs still use framework captions and require follow-up.
