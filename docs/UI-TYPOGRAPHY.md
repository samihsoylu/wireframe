# Platform UI typography

Status: regular macOS interface fonts accepted in W60; mapped medium headings implemented and accepted in bounded macOS native checks in W62. Other platforms remain open.

UI typography is independent of project/canvas fonts. Use the platform UI font for editor chrome, with coherent face, weight, optical sizing, measurement and rasterization in ordinary widgets, shaped labels and editable fields. Do not change stored project fonts, canvas/export drawing or embedded documents to repair chrome.

The macOS Quick Add contract is the native system font at13points. Panel-title archive defaults use system medium13; navigator defaults include regular12/13. Back now maps to regular 12; library category buttons map to the platform control-content font at 11 points. Remaining title/label styles and runtime overrides still need explicit mapping. These values are local macOS evidence, not a cross-platform font policy inferred for Windows/Linux.

On the current macOS host, system13 selects optical axis17 from SFNS.ttf; medium adds weight510. The file's default optical axis28 is materially different. Load fonts through host APIs without bundling platform-owned font files. Preserve the selected font variation through shaping, painting and caches. Font fallback must remain available for missing scripts.

Current implementation discovers the host regular UI font through CoreText, loads its face/axes once and shares the bytes. The input engine registers immutable per-size instances (bounded to 128) through the local cosmic-text patch; epaint uses opt-in logical optical sizing and tracking. Both paths preserve matching shaping/rasterization coordinates. The existing Ubuntu Light path remains the fallback when discovery is unavailable and on Windows/Linux. Canvas families are captured before the interface font is prepended, and host font bytes never enter documents or distributions.

Acceptance: native macOS13pt Quick Add and matching selected platform styles; regular/medium distinctions; long titles and narrow layouts; multilingual typing, grapheme navigation, selection/clipboard, IME, focus and Undo; existing four realistic sketches with unchanged canvas/export/document fonts. Confirm Windows/Linux font choice and rendering on native platforms. Diagnostic CoreText samples are evidence for font selection and metrics only, not acceptance of the product renderer.

## Validated integration requirements

A standalone macOS prototype confirms that logical point size must reach the shaper as well as the chosen variation axes. The host system font applies size-dependent tracking; leaving point size unset uses a 12-point default and produces wider 13-point text. Do not compensate with a hard-coded per-character adjustment or change raster scale.

The tested 16 styles cover regular/medium at 10, 11, 12, 13, 14, 18, 24 and 36 points. Across 15 phrases and both widget/input engines, maximum advance-width difference from the native font renderer is 0.056 logical points. Regular 13-point input measurements agree exactly on that corpus. These are font-only measurements, not native product acceptance or proof of pixel identity. Distinct immutable instance IDs preserve axes/point-size cache isolation; reverse-order image/outline checks and four display scales pass.

The integrated regular font satisfies the shared-description, immutable-cache and interface-only tracking requirements. Mapped medium headings are implemented and have the bounded W62 acceptance below; unmapped strong-label roles and other-platform font behavior remain open. The current host's optical size clamps from logical size through the font axes; other macOS font generations or size-specific face changes require validation before claiming equal fidelity.

## Integration acceptance

The actual product widget/input engines agree with live CoreText advances within 0.1 logical point over 224 regular-font cases: eight sizes, seven phrases and four display scales. Four canvas font roles at four sizes retain exact advances and project encoding. A portable variable-font fixture verifies independent immutable instances, invalid/duplicate registration, and retained raster/outline results after later instances.

277 selected tests pass; all-target Clippy with warnings denied, formatting, optimized macOS ARM64 build and strict ad-hoc signature pass. Native self-review at 1360×932 logical pixels verifies search-button Shift-Tab/Return, matching/insertion/Undo, saved accent/emoji/Hebrew text with text-local Undo, and desktop/phone/tablet views without obvious clipping. Fresh-process reopen and resave are byte-identical. The original graph, embedded fonts and assets survive the workflow; only the expected file-format upgrade occurs.

These checks do not establish pixel identity, canvas blur resolution, all Unicode/IME behavior, native narrow-window acceptance, or Windows/Linux rendering. Quick Add is explicitly 13 points; other existing sizes remain until their styles are mapped. No platform font is bundled. The temporary candidate is closed; no installation or commit occurred.

## Mapped style requirements — W61

| Role | Typeface and size | Status |
| --- | --- | --- |
| Quick Add | Platform regular, 13 points | Integrated and accepted in W60 |
| Back to wireframes | Platform regular, 12 points | Integrated at regular 12 and accepted in W62 |
| Library categories | Platform control-content font, 11 points | W69 corrects the earlier 14-point mapping; integrated in W70 |
| Panel heading | Platform medium, 13 points | Integrated at medium 13 for mapped titles and accepted in W62 |

Back's control height is 20 points. Horizontal cell padding is 12 points per side and icon/text spacing is 5 points; total width depends on measured label and icon. Keep the control accessible and preserve its keyboard/notes/gesture exit behavior when adjusting its dimensions.

Implement regular/medium as explicit shared interface roles, carrying host-selected face/weight/axes through both widget and shaped-label paths. Do not map every strong label to medium without checking its role or hard-code one host's numeric variation coordinate. The current regular font path remains accepted; no new font or rendering change occurs in W61. Next validate the mapped roles in a single candidate, including native narrow-window and retained realistic scenes.

## Mapped style integration — W62

Regular and medium are explicit interface roles. On macOS, the medium role comes from the host system-font API and retains its selected variation axes through widget and shaped-caption engines. Cached captions distinguish roles. Font discovery permits a unique font-file face for a named variable-instance alias; a collection still requires an exact PostScript face match. No platform-owned font bytes enter documents or the repository.

Wireframes, the source Components heading and current page/source inspector titles now use medium 13. Back uses regular 12, measured width and the mapped height/padding/gap. Other strong labels remain unchanged pending role mapping. Non-macOS fallback is retained; a real medium weight on those platforms is not yet claimed. All 278 selected tests, all-target warnings-denied Clippy, formatting and optimized macOS build/signature pass. The native source/page toolbars and mapped headings remain readable at a 941-logical-pixel captured window, including the four existing realistic scenarios. Back commits Unicode source notes, Undo restores the graph, and fresh-process scenario reopen/resave is byte-identical. Twenty evidence assertions pass. The candidate is closed and old recovery records are preserved. This is bounded macOS self-review, not pixel identity or other-platform acceptance. Immediate post-Quick-Add keyboard Undo remains a separately recorded focus-routing observation; toolbar Undo restores the exact fixture.

## Category-row correction — W69

The active category control uses the platform control-content font at 11 points. The earlier 14-point mapping came from a different control path and is superseded. Back at regular 12 and mapped panel headings at medium 13 are unaffected.

The category strip is 30 logical points high, with 24-point buttons and 4-point gaps. Center the group when space permits; preserve full-size captions as space becomes tight. Native measurement on the current macOS host yields a rounded half-point text advance plus 16 points of horizontal allowance. This is a measured host contract, not a universal native-theme formula. Native inactive/selected rendering and cross-platform font metrics still require product comparison.

Integrate this as a bounded layout change, preserving Shift category unions, keyboard/accessibility activation, visible focus, disabled guards and existing overflow access. Missing catalog content and external catalog links remain separate functional work; do not add inactive placeholder actions. W69 left category rendering unchanged; W70 integrates this contract below.

## Category-row integration — W70

The category strip uses explicit regular 11-point captions, measured half-point widths with 16-point horizontal allowance, 24-point rounded buttons, 4-point gaps and a 30-point gray row. The group centers when it fits; horizontal overflow keeps full-size captions, and focus scrolls an off-screen category into view. Components remains a stable product category, positioned before Text. Thumbnail geometry and insertion behavior are retained.

Selection is exposed as accessible toggle buttons. Keyboard activation, visible focus, disabled guards and Shift unions remain available. The shared activation helper takes Shift from the pointer release event, preserving quick Shift-clicks even when the modifier is released before the next frame. The navigator keeps its existing handling through that same helper.

Colors and rounded shapes use the product palette and remain approximations of native-theme details. Current asset/external catalog gaps remain explicit; no placeholder actions are added. All 287 selected tests, all-target warnings-denied Clippy, formatting/diff and the optimized ARM64 build/signature pass. Native macOS review verifies 1360- and 941-point layouts, empty categories, visible keyboard focus/activation, corrected quick Shift-click unions and fresh-process persistence. Seventeen artifact checks pass; the fixture is byte-identical and old recovery/preferences are preserved. This is bounded macOS self-review; native Windows/Linux and exact theme equivalence remain open.

## Selected presentation list captions

Selected presentation List captions map to the host bold12 role; unselected captions retain regular12. The macOS role is requested through the host system-font API, retaining face/axes through widget and shaped-caption paths and independent caches. No font bytes enter documents or the repository. Other selected/thumbnail styles remain unchanged; non-macOS fallback remains available, without a native weight-fidelity claim.

Thirty affected checks pass, including672 interface metric cases across three roles/eight sizes/seven phrases/four scales, canvas/project-font preservation and narrow grapheme truncation. Native selected Landing and Contact desktop captions remain readable; page navigation, settings and cold-launch List/Review retention pass. External scenario-fidelity-current owns paired and after-change evidence. Exact glyph/color identity and native Windows/Linux acceptance remain unverified.
