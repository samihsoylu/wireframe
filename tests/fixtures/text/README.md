# Text rendering fixtures

Synthetic bitmap, color-layer, oversized-input and fallback-isolation fonts for rendering tests. Regenerate with `python scripts/build_text_fixtures.py` using the pinned font tooling in `scripts/font-requirements.txt`.

These modify the bundled Wireframe Draft face, derived from Comic Neue. Glyph outlines retain the SIL Open Font License in `assets/fonts/wireframe-draft/OFL.txt` and its copyright/modification notices. Added test tables contain generated colored pixels, extreme dimensions, layer lists and a noncharacter mapping. They are not intended as user fonts. The generator is GPL-3.0-only.

The `variable/` fixtures use entirely original GPL-3.0-only geometric outlines with weight, width and optical-size axes. Regenerate them with `python scripts/build_variable_fixtures.py`. Static instances provide known geometry for the default-axis comparisons.
