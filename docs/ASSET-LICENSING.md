# Graphics and font permissions

Every asset distributed with Wireframe must have a documented source and permission for its intended use, modification and redistribution. Copyrighted assets are acceptable only under permission that covers those uses. A missing source, unclear permission or unmet license condition blocks inclusion and release; replace or remove the asset until resolved. This includes dependency-supplied graphics, fonts, examples, screenshots and installer artwork.

For every new or changed asset, record its author/source, version or revision, license or written permission, required attribution, modifications, and how the distributed bytes can be identified. Preserve applicable copyright and license texts in the source and delivered packages. Project code's GPL-3.0-only license does not replace third-party asset licenses. Recheck the record when changing sources or versions.

## Current sources

| Asset | Source and permission | Retained evidence |
| --- | --- | --- |
| Application icon and PNG/ICNS/ICO variants | Original project SVG artwork; GPL-3.0-only | [Source and regeneration](../assets/app-icon/README.md) |
| Toolbar graphics and control drawing geometry | Project-authored Rust vector drawing; GPL-3.0-only | [Toolbar source](../src/app/toolbar/icons.rs), [control rendering](../src/render) |
| Icon catalog | Font Awesome Free 6.7.2, Fonticons, Inc.; SVG paths under CC BY 4.0 | [Pinned sources and hashes](../assets/icons/font-awesome-free/PROVENANCE.json), [license](../assets/icons/font-awesome-free/LICENSE.txt), [attribution](../THIRD_PARTY_NOTICES.md#bundled-icons) |
| Comic Neue, four faces | Comic Neue Project Authors, from a pinned Google Fonts revision; OFL-1.1 | [Sources and hashes](../assets/fonts/comic-neue/PROVENANCE.json), [copyright and license](../assets/fonts/comic-neue/OFL.txt) |
| Wireframe Draft, four faces | Modified Comic Neue, renamed and distributed under OFL-1.1 | [Modifications](../assets/fonts/wireframe-draft/MODIFICATIONS.txt), [input/output hashes](../assets/fonts/wireframe-draft/derivation.json), [retained license](../assets/fonts/wireframe-draft/OFL.txt), [rebuild](PROJECT-FONTS.md) |
| Dependency interface/fallback fonts | `epaint_default_fonts` 0.36.2: Ubuntu Light (Ubuntu Font Licence), Noto Emoji (OFL-1.1), emoji icon font (MIT), Hack (MIT and retained Bitstream Vera terms/public-domain contributions) | [Bundled notices](../assets/fonts/fallback-notices.txt), [dependency license files](../licenses/epaint_default_fonts-0.36.2/fonts) |
| Invisible PDF semantic carrier | Original generated rectangular glyphs; GPL-3.0-only | [Purpose and regeneration](../assets/pdf/README.md) |
| Text test fixtures in the source package | Eleven modified Wireframe Draft fonts under OFL-1.1; sixteen original variable-font diagnostic files under GPL-3.0-only | [Fixture sources, modifications and licenses](../tests/fixtures/text/README.md) |
| Three vendored text-rendering screenshots in the source package | Unchanged cosmic-text 0.19.0 upstream artwork, under its MIT OR Apache-2.0 declaration | [Upstream identity and file hashes](../vendor/cosmic-text/UPSTREAM.json), [upstream license declaration](../vendor/cosmic-text/README.md#license), [MIT notice](../vendor/cosmic-text/LICENSE-MIT) |
| Unicode emoji property data | Unicode License V3 | [Pinned sources](../assets/unicode/README.md), [license](../assets/unicode/LICENSE.txt) |

Font Awesome attribution and font notices must accompany applicable exports as well as the application distribution. OFL derivatives retain their license and copyright; modified reserved font names must follow that license. Brand icons do not grant trademark rights or imply endorsement.

## Verification and release gate

The 21 October 2026 source check inventoried all 45 tracked graphics/font files, matched all 12 recorded Comic Neue, derived-font, icon-catalog/category and icon-license hashes plus three vendored screenshot hashes, confirmed the eleven modified fixture fonts retain the original copyright/OFL metadata, and located dependency font notices. The derived drawing fonts retain the original OFL text; the app icon and toolbar have editable project sources. This is a source provenance check, not blanket legal clearance or acceptance of a future binary package. Final delivery must verify that the actual package and its exports retain the required notices.

User-imported images/fonts and installed system fonts used for fallback are separate from the bundled inventory. Their availability does not establish redistribution permission. Do not add them to product examples or release material without checking applicable rights; font embedding and exported outlines also need the relevant permissions.

Before release, reconcile every asset in actual deliverables against this inventory, including dependency additions and generated installer material. Record unresolved permissions and exclude those assets from delivery. See also [Third-party notices](../THIRD_PARTY_NOTICES.md).

The 27 October Sketch contour refinements (Cover Flow joins/weight, Callout rim and brace taper) are original procedural Rust geometry by Wireframe contributors under GPL-3.0-only. The editable source and Git revision identify the modified bytes; no third-party path/image/font asset was introduced. Reference comparisons and cache artwork remain outside product Git and deliverables.
