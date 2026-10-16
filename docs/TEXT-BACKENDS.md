# Positioned text backends

Canvas controls, measurement, wrapping and exports share positioned glyphs. macOS uses CoreText. Windows and Linux use cosmic-text with system font discovery and skrifa outlines. The interface retains original UTF-8 source ranges, glyph positions, advances, direction, actual font data and detached outline/bitmap ink. Exports consume that result instead of reshaping strings with a second engine.

Document font files keep their explicit four roles. Native macOS shaping automatically adjusts an available optical-size axis to the requested size; the portable adapter currently keeps the declared optical-size default. That cross-platform difference remains open for variable-font parity. The portable adapter caches up to eight primary font identities and excludes earlier document fonts from system fallback. Emoji presentation is selected per grapheme using pinned Unicode 17 data. Missing coverage returns an explicit error; it must not silently substitute another document's font.

Each shaped span retains bounded font/ink data. Before image decoding, the portable adapter validates bitmap dimensions, compressed PNG output size, color layers and outline bounds. Ordinary outline glyphs bypass the color-image rasterizer. Shaped and raster caches each retain at most128 entries and32MiB under their recorded accounting. Shared font faces are charged once within each cache. Detached tables transfer into packed bytes once rather than retaining both representations. These are retained-cache limits, not an application-wide or complete export-memory guarantee.

SVG and PNG retain detached ink and original source labels. PDF embeds the actual resolved fonts; ink-only clusters receive transparent semantic text using an original rectangular carrier. It does not change visible glyphs. Some PDF readers still reorder joined emoji in pages containing right-to-left text; ordinary combining/bidi extraction also needs broader acceptance. Correct visible output is not proof of exact copy/search behavior.

Font fixtures and their generators cover bitmap budgets, primary isolation, cache reuse and semantic-cluster handling. See `tests/fixtures/text/README.md`, `assets/pdf/README.md` and `assets/unicode/README.md` for provenance and regeneration. Linux verification requires installed script and color-emoji fonts; a useful test environment includes Noto core, CJK and color emoji.

Native Windows/Linux desktop behavior and packaging, arbitrary font families and full multilingual export acceptance remain separate checks. The native macOS route remains in place.
