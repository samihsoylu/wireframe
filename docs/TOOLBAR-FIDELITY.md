# Toolbar fidelity

## History glyph geometry — W104

Undo/Redo use original vector heads with an 8-point horizontal span and 10-point height, joined to a 7.5-point-radius, one-point arc spanning 270 degrees. This removes the old broad head/arc overlap and matches the measured endpoint. Native density 2 silhouette comparison improves consistently across three thresholds; fine raster coverage is not claimed identical. Button layout, labels/focus and actions remain unchanged.

Eight toolbar tests, all-target warnings-denied Clippy and optimized macOS build pass. Native New Page/Undo/Redo/Undo restores the saved fixture byte-for-byte; 15 artifact assertions preserve prior profile entries and confirm candidate cleanup. No installation, commit or current native Windows/Linux acceptance. Other toolbar glyph details and canvas clarity remain open.

## Magnifier glyph geometry — W89

Zoom In, Zoom Out, Actual Size and Fit use original vector artwork with a one-point lens outline, two-point rounded handle, aligned internal symbols and compact Fit corners. Lens radius accounts for outward stroke expansion. Native captures verify the measured outer size/alignment and retain 30-point pitch; exact edge coverage and the tiny Actual Size lettering are not claimed identical. Other command glyphs and broader visual fidelity remain open.

All eight toolbar tests, Clippy and optimized macOS build pass. Native zoom, Fit and reset checks preserve the fixture byte-for-byte. The review candidate is closed; prior recovery files remain. W88 layout/keyboard evidence applies to unchanged interaction and frames. No installation, commit, independent review or current native Windows/Linux acceptance in this pass.

## Compact action layout — W88

Ordinary icon buttons use20x20point frames; Trash uses24x25. Action items have10-point gaps,10x20dividers and 10-point outer horizontal margins. The centered history/zoom group is190points wide; adding a divider and Trash makes it244. Click frames are separate from original glyph geometry. Glyph size, vertical alignment and the existing Undo path overhang remain follow-up work; the horizontal layout is verified.

Eight affected toolbar tests cover geometry, enabled/pressed behavior, commands, source navigation and non-overlap at860/940/1360/1920widths. Clippy and optimized macOS build pass. Native self-review at1360/941widths confirms30-point zoom pitch, zoom/reset, New Page/Undo, source/Back, Trash open/close and visible keyboard focus with Space toggling. Both fixtures stay byte-identical and prior recovery entries remain. Candidate closed; no installation or commit. No current native Windows/Linux acceptance for this increment.

## Icon state rendering — W86

Normal toolbar ink is white; selected and pressed defaults are #3D9BE8 and #B1D7F5. On macOS an explicit host accent is read without changing preferences, converted through device RGB for the lighter HSV variant (saturation minus 0.25, brightness plus 0.1, clamped), and returned as sRGB. Resolution happens on each toolbar paint. Other platforms and unavailable native resolution use portable defaults. Immediate notification-driven idle refresh and runtime changes to custom system accents remain unaccepted.

Disabled icons receive one 40% opacity operation. Pointer hover does not fill the button; held press lights the icon only while the pointer remains inside. Release outside cancels activation. Keyboard focus retains an outline, and selected Inspector uses the shared toolbar color for its cutout. Labels, tooltips, hit areas and commands are unchanged.

Seven toolbar tests exercise actual rendering and input, including disabled Undo, plain hover and held-outside cancellation. All-target Clippy, formatting and optimized macOS build pass. Native self-review verifies New Page/Undo, selected Inspector on/off and exact saved-fixture restoration. All preexisting recovery entries remain. The temporary candidate is closed. Custom-accent runtime, physical held press, other-platform native and broader glyph/layout acceptance remain open. No installed-app replacement or commit.

## Previous icon state requirements — W85

The toolbar needs separate normal, selected, pressed, disabled and keyboard-focus treatments. Normal icons use white. macOS selection resolves the host accent, with product fallback #3D9BE8; the default pressed tint is #B1D7F5. A custom native accent uses a lighter variation, with appearance/color-space handling verified before acceptance. Other platforms need an explicit portable default. Pressed tint applies to selected and unselected commands. Ordinary hover must not be conflated with pressed state. Disabled appearance and accent-change refresh still need focused verification.

At W85, the hard-coded selected color and filled hover rectangle did not meet this contract. W86 above implements the correction. Preserve accessible names, tooltips, hit geometry, action enablement and visible keyboard focus. Use the shared toolbar surface for the selected inspector icon's cutout. This paragraph records the earlier research requirement. W84's palette/build/native evidence remains applicable to unchanged executable code; current native Windows/Linux acceptance remains open.

## Shared editor palette — W84

Toolbar background is sRGB #394045. Library body is #EEEEEE; category row and active panel headings are #C9CBCC. Non-main panel headings are #E0E1E1. The macOS document window supplies its main-window state directly, which is distinct from keyboard focus while native dialogs are open. Other platforms fall back to input focus until native-window semantics are verified. These are editor surfaces, separate from document colors. Toolbar icon states are unchanged.

Five toolbar and seventeen library regressions pass, along with all-target warnings-denied Clippy and an optimized ARM64 build. Native landing-page review, Fit and Open/Cancel preserve the complete project data; serialization differs by four bytes. All preexisting recovery entries remain. Active appearance was visually reviewed; a dialog-only capture does not establish background-window colors or an inactive transition. Self-review only, no current Windows/Linux native evidence, installed-app replacement or release change.

## Quick Add history and Escape — W66

Quick Add preserves local query Undo/Redo and then falls back to document history when exhausted. Immediate keyboard Undo after insertion now works with the field still focused. Escape clears a nonempty query, then leaves the empty field without changing the source context. Five regressions,278selected tests and native keyboard/menu checks pass. See EDITOR-TOOLBAR.md. Existing text fields and composition ownership are retained.

The automated native-menu Escape observation is inconclusive: the same delivery also leaves the native reference menu open. Physical delivery still needs separate evidence; no menu workaround is introduced. W67 resolved empty-query Return editing and its query-focus recovery; see EDITOR-TOOLBAR.md. W65 removed the duplicated source-workspace Components footer after replacement insertion acceptance. Broader toolbar/library geometry, icon/color/spacing fidelity and other-platform acceptance remain open.

## Mapped interface styles — W62

Back now uses regular platform text at 12 points with measured width, 20-point height, 12-point horizontal margins and a 5-point icon/text gap. The original chevron occupies an assumed 8-point box; exact icon geometry remains open. Mapped panel headings use the shared medium 13-point role. Keyboard routes and canvas/project typography are retained. Native narrow-layout acceptance passes at 941 logical pixels for page/source toolbars, with and without the library. Back commits source notes and returns to pages; original fixture graphs and fresh-process save bytes are retained. All four realistic scenes fit without heading clipping. All 278 selected tests and 20 evidence assertions pass, plus all-target warnings-denied Clippy and optimized build/signature. No installation or commit.

The captured reference width is 920 logical pixels, so exact-size pixel comparison is not claimed. At that checkpoint, duplicate lower Components navigation and immediate post-Quick-Add keyboard Undo were open; W65 and W66 above resolve those bounded cases. Broader spacing/category/icon/color fidelity and physical native-menu Escape remain open. Candidate closed; original Menu Review retained.

## Previous toolbar text style contract — W61

Back uses regular platform text at 12 points in a 20-point-high control. Its measured width includes 12-point horizontal padding on each side, icon width and a 5-point gap. This historical path suggested 14-point category labels; W69 traced the active library path to regular 11 points and supersedes that assumption. Back was implemented at 12 points/20 high in W62.

Native reference page and source layouts are now captured at 920 logical pixels using the system Window → Move & Resize → Left command. Their toolbar commands remain visible. This establishes a comparison target, not narrow product acceptance. Edge dragging did not resize the window. Next apply the Back contract together with mapped medium panel headings and verify actual product layout at a narrow width. See UI-TYPOGRAPHY.md.

## Previous typography and keyboard refinement — W60

The editor's regular interface font now comes from the macOS host, with coherent logical sizing in widgets and shaped input; Quick Add uses 13 points. See UI-TYPOGRAPHY.md for implementation, cache and preservation contracts. The focused search affordance now receives Return. Global canvas text editing runs only when focus is absent or belongs to the canvas; earlier source/page navigation handling stays intact.

Two regressions exercise the actual toolbar search button through global shortcuts and preserve canvas Return editing. Native Shift-Tab→Return→query→Return insertion and Undo pass, alongside saved Unicode editing and fresh-process reopening. The complete affected run passes 277 selected tests and all-target warnings-denied Clippy. This closes W57's observed focused-search Return issue; it does not resolve the separate native-menu Escape observation.

Open at W60: medium/header weights, Back's final size (resolved by W61 above), broader stroke/spacing/colors, source navigator redundancy, native narrow-window captures and other platforms. Current native acceptance is 1360×932 logical pixels. No installation or commit. Overall goal active/incomplete.

## Previous toolbar detail refinement — W57

Trash is grouped after the four zoom actions. Source editing uses a filled Back button and a distinct New Library icon; Presentation is omitted and disabled consistently for menu/shortcut/direct entry. Page Presentation remains available. Quick Add has an original vector search glyph, focusable affordance and outer keyboard-focus outline. The125logical-pixel field keeps existing catalog matching, input, popup, insertion and Undo behavior. Fonts and canvas drawing are unchanged.

Verification:240application tests pass before final focused refinements; all5toolbar tests pass afterward. Bounds/order are checked at860,940,1360,1920logical widths with long titles and Trash. Source/library command visibility, New Library/Undo, Back and Quick Add focus/insertion/Undo pass. All-target Clippy with warnings denied, formatting/diff checks, optimized macOS ARM64 build and strict ad-hoc signature pass. Native self-review at1360x932logical establishes16storage/UI/profile assertions. A final outline-only rebuild separately verifies Shift-Tab focus, visible outline while pointer is elsewhere, and Space activation. This is not independent review or native narrow/platform acceptance.

Both temporary insertions are undone and the named fixture saves exactly its starting graph. The review app is closed; existing working copies and original Menu Review remain. No installation, release replacement or commit. The first candidate's full workflow evidence is retained separately from the final outline-only build identity.

Open: UI font face/weight fidelity, broader stroke/spacing/colors, source navigator redundancy, narrow native captures and other platforms. At W57, Return did not activate the focused search affordance while Space did; W60 above resolves that observed routing issue. Automated Escape while a native menu was open left the menu open and exited a source level; physical delivery/cause remains unverified. These observations are follow-ups, not claimed fixes. Permanent removal's native acceptance remains separate and pending. Overall goal active/incomplete.

Previous W53: moving Back/New Library into the main row removed26logical pixels of redundant captions, with236application tests and12native/storage assertions. W57 supersedes its open Trash grouping, Back styling, search glyph and source Presentation points; remaining typography/layout scope stays open.

## Open user clarity comparison

A user-provided side-by-side toolbar comparison reports softer or less distinct product glyphs. Recheck against a current native candidate at matched display density and window state: stroke coverage, pixel alignment, internal symbols, contrast and disabled/selected rendering. The screenshot alone does not identify the running product build or establish a resampling cause. Preserve this acceptance item independently from catalog-icon thumbnail sizing and canvas text clarity.


## Library canvas origin

The thumbnail strip now reserves a fixed height when switching between empty and overflowing categories. A solid scrollbar occupies its own track rather than overlaying captions. The application-frame regression verifies 120 logical pixels at 940, 1360 and 1920 widths for All and empty Assets, failing on the preceding 111-pixel layout. Native category switching and track navigation pass on the final candidate. Thumb shade/rounding/inset and small border-edge differences remain cosmetic residuals; continuous physical thumb drag is not newly accepted. 23 affected tests, build/lint/signature and unchanged-fixture checks pass. See external editor-library-current/report.md.
