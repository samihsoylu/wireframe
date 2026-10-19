# Presentation navigation

Implemented in source, with scoped native macOS acceptance. Presentation uses a stable page/version destination and a two-way history cursor. Toolbar controls, keyboard navigation and page links share the controller.

## Current scenario checkpoint

The current presentation navigator uses full-width list rows, independent list/thumbnail controls and shared page preview drawing. Its view mode is separate from the editor and survives presentation re-entry within the current application window. Scoped native cold-launch retention of List/Review now passes; broader cross-project/global persistence remains unverified. The selected page menu retains versions, Back/Forward and settings; right-click also opens those actions. Notes/Comments have aligned headers and backgrounds; the local reviewer menu remains available. All icon-only controls have accessible names.

Navigator Prev/Next actions use compact padding and bounded captions so portable font metrics cannot expand the requested panel width. Full accessible names remain available when a narrow caption is truncated. The corrected 35-case width/density matrix and original drag-resize/history check pass on macOS and Linux; this is application-event evidence, not native visual acceptance of the latest correction.

Paired native observations establish view-mode independence and retained Review preset. Final product native replay verifies thumbnail page selection, menu Back, re-entry, editor destination and unchanged editor mode. Both fixtures remain byte-identical. Navigator X continues to hide its pane; J restores it and Escape exits. Existing paired preset/keyboard/history checks and earlier pointer-link evidence remain scoped as recorded.

The layout candidate passes 399 application tests with one preexisting opt-in graphics test ignored. The final chevron-only drawing correction passes all 24 affected presentation tests, native replay, build, all-target Clippy and local signature verification. Exact current candidate and captures: external native-scenarios-current/presentation-chrome/report.md and bundle.json. Typography, selected-row emphasis/color, precise canvas contours, comments-composer appearance, native held-pan and broader platform/package acceptance remain open. No pixel-equivalence or independent-review claim.

## Current interface implementation

Full-screen entry now opens settings with Review/User Test/Custom, all six aid controls, zoom action, Advanced centering and layered Escape. Navigator, editable Notes, markup filtering, link hints and original arrow/finger/hand drawings are connected. Settings persist per project outside document Undo. The Comments panel now connects stored discussions, replies and local review callouts through the [review workflow](REVIEW-COMMENTS.md). Native appearance, input and integrated reopen remain open.

The native macOS candidate verifies preset switching, Notes shortcut text and Cancel, phone shared/individual centering, visible large arrow, tablet navigation and alternate/history access. The compact navigator fits its default 200-point width. Earlier broad application checks passed 281 tests; final focused route/interface checks pass 22, with 36 neighboring rendering checks, all-target Clippy and an optimized build. This is scoped self-review, not independent review or pixel-equivalent acceptance. Native held-pan/finger transitions, precise artwork, toolbar/canvas clarity, accessibility and current Windows/Linux GUI checks remain open. Evidence is retained outside the product repository in the presentation-interface checkpoint.

## Gesture verification

Timed application checks reproduce and correct two interrupted-gesture defects: a held press must not restart after settings dismiss, and its origin must not cross page navigation. Start only from a fresh primary canvas press; cancel when framing changes. Tests cover the 150 ms threshold, truncated integer deltas, release and unchanged document/Undo. Final affected suite passes 25 checks, with all-target Clippy and an optimized build.

Native macOS checks confirm the link finger, arrow off links, cursor suppression over settings, link activation and editor return. The fixture remains byte-identical. Automated drag did not visibly pan; held cadence and the hand remain unaccepted. The temporary link-hint correction below supersedes the initial persistent outlines. No independent or current Windows/Linux GUI acceptance is implied.

## Temporary link hints

The Link Hints aid enables temporary feedback. At rest links are unmarked. Alt/Option reveals filled link areas; release fades opacity over 400 ms. A blank primary canvas press flashes hints without altering the document. Initial white feedback uses alpha 0.3 for 40 ms, then transitions over 40 ms toward the primary or broken-target alert color at alpha 0.8. These timings are code-derived; native screen cadence remains unaccepted.

Disabling the aid, settings/text input, focus loss or reframing cancels feedback. Split hit annotations sharing a visible hint paint once. Held feedback stops requesting animation frames after settling. Missing page targets use the alert color. Link activation remains independent of the aid.

Thirty presentation checks pass, including timed state and application render-output checks, with all-target Clippy and an optimized ARM64 build. Native macOS confirms entry, the unmarked resting canvas, aid-off link activation and editor return; copied fixture bytes and prior profile entries are preserved. A subsequent passive diagnostic captures native Alt delivery, held blue fills, release fading, blank-click flashing and disabled-aid suppression. Six framebuffer captures and 181 presentation frames close the product modifier-delivery gap; earlier window captures had missed that state. Logging/readback overhead means this does not establish uninstrumented cadence or reference timing parity. Product code remains unchanged; eighteen preservation/behavior assertions and framebuffer pixel checks pass. Hover tooltips, exact platform animation interruption behavior, nested/cropped compositing and current Windows/Linux GUI acceptance remain open. Evidence is retained externally in the presentation-link-hints and presentation-modifiers checkpoints.

## User behavior

Presentation starts at the current page and selected version. A new presentation session starts a new route history. Leaving presentation keeps the last viewed page/version selected in the editor.

Previous/Next page follows document page order and restores each destination page's remembered version. Previous/Next including versions walks Official followed by the page's ordered alternates; crossing a page boundary selects the previous page's last version or the next page's Official version. At the document boundaries, navigation does nothing. Provide the Option/Alt+Up/Down shortcuts for ordinary traversal and Shift+Option/Alt+Up/Down for traversal including versions.

Back and Forward restore the exact page and version recorded in the route, including changes between versions on the same page. Add a route entry only for a successful change of destination. Navigating somewhere new after Back discards the previous forward route. At either history boundary, do nothing. Option/Alt+Left/Right navigate history; plain Left/Right are compatibility aliases when presentation canvas input owns the keys. Editable text and open dialogs retain their own keyboard input. Back/Forward and settings are available through the navigator overflow menu; the adjacent compact dropdown contains versions.

Page links continue to use the destination page's remembered version. Back links use the same controller as history navigation. Missing pages must not mutate the document or create successful route entries. Preserve the established recovery behavior for stale history: skip deleted pages, and fall back to Official if a recorded alternate has been deleted. Reordering pages must not retarget stored history; resolve by stable identity.

## Implementation and acceptance

Use an explicit page/version destination and history cursor, separate from document Undo and persisted view preferences. Route toolbar navigation, shortcuts and links through the same destination resolver. Keep the change local to presentation navigation; preserve the existing geometry and hit-testing pipeline.

Regression checks must exercise same-page version history, forward truncation, session reset, both traversal modes across page boundaries, stable identities after reorder, deleted pages/versions, invalid targets, keyboard focus boundaries and unchanged document/Undo state. Verify native landing → contact → phone navigation plus Back/Forward and alternate traversal on one coherent candidate. Existing Back-only tests are insufficient. Native pointer activation in all nested/cropped compositions remains a separate acceptance requirement.

## Current version-route checkpoint

The unchanged Open Review 2 debug candidate passes native pointer navigation from Landing Focused to Contact Focused, alternate traversal to Contact Official, exact-version Back/Forward, and canvas Back links through same-page and cross-page history. A new navigator route to Phone clears Forward history: the shortcut is a no-op and the menu action is disabled. Returning through Back restores Contact Focused and Landing Focused. Editor return leaves Undo/Redo empty; native Save and cold reopen preserve the project byte-for-byte and both remembered alternate choices. Evidence: external `version-route-current/report.md`.

The preceding 425-test application run remains applicable because no runtime inputs changed. Retained comparison evidence establishes the reference navigation controller and native history behavior; this pass did not run a fresh paired reference session. The fixture was generated and then opened/saved through the native app, not created through native editing. Nested/cropped hit regions, physical cadence, broader accessibility and platform/package acceptance remain separate. Defensive stale-page/version handling is application-tested; ordinary document deletion exits presentation and starts a new history on re-entry.

## Retained navigation checkpoint

The application suite passed 121 tests before the final menu correction. After a native-discovered Escape/menu regression was fixed, all 13 presentation-related tests and all-target Clippy passed. The final optimized candidate passed native remembered-version traversal, exact page/version Back and Forward, forward truncation, ordered version traversal across page boundaries, toolbar history, direct version selection, landing-to-contact pointer navigation, editor return and fresh-session reset. Escape closes an open Versions menu without leaving presentation.

The fixture remained byte-identical. Source hashes match the tested candidate and the existing distribution executable is unchanged. No installation or commit occurred. This is scoped macOS acceptance; nested/cropped hotspot combinations, native Windows/Linux and broader accessibility remain open.

## Presentation fidelity requirements

User review requires faithful toolbar entry and full-screen presentation. Cover Review, User Test and Custom modes; Navigator (J), Notes (N), Comments (C), Link Hints (L), Big Arrow Cursor (B), Markup (M), actual-size zoom (Z) and Advanced settings. The large arrow is a transient pointer, never document artwork. Establish preset defaults, toggle persistence, focus handling, settings dismissal, navigation and restoration on exit before implementation. Retain the already verified page/version route behavior. Current navigation acceptance does not cover these settings or visual equivalence.

## Settings contract

The full-screen entry opens presentation settings. Review enables Navigator, Notes, Comments, Link Hints, Big Arrow Cursor and Markup, using fit zoom. User Test disables all six and uses actual-size zoom. The zoom control labels its destination action: actual size when fit is active, and fit when actual size is active. Default presentation panel dimensions are 200 points for the navigator, 300 for the right panel, and 300 for the notes region. Native layout can constrain these values.

An individual option or panel-size change selects Custom and saves the settings immediately. Within one presentation controller, Custom recalls the latest individual configuration across preset changes. Selecting the active preset is a no-op. A first Custom selection retains current values. Preset changes retain panel dimensions and centering exclusions. Centering changes save without changing the preset or cached Custom configuration. Persist the current configuration per project outside document Undo; a saved Custom configuration initializes the next session. Do not invent a separately persisted historical Custom snapshot after another preset has replaced it.

Use J/N/C/L/B/M for the six aids, Z for fit/actual, R for Review, U for User Test and S for settings. Settings close with Enter, S or Escape; Escape outside settings exits presentation. Native checks cover R/U/S/B and the combined J/N/C/L/M/Z sequence, Custom recall, and the two-step Escape flow. Focus boundaries around text fields remain an explicit verification requirement.

Advanced opens project centering rules. Included pages and alternates contribute their largest width and height independently, using visible artwork bounds for the current markup policy. Exclusions are page/version-specific. An excluded current page centers and fits against its own extent. Fit only shrinks, with a 5% floor and a 100% cap; actual size stays 100%. Small included pages share the common project extent, avoiding per-page recentering. Preserve markup offsets and editor view restoration. Exclusion changes and mixed-size native navigation still require visual acceptance.

The large arrow is a transient native pointer layer. Its original product equivalent must follow the pointer without becoming document artwork or intercepting page links, and must disappear over settings/appropriate non-canvas areas. The native contract is established; exact product contours, motion and modal behavior remain subject to scoped acceptance. The aid toggle alone is insufficient. Comments and local review callouts are now implemented; their native acceptance remains open. Navigation acceptance alone does not establish complete presentation parity.

Evidence: 19 executed source-method checks (12 settings, seven centering) with synthetic host/page adapters, native preset/shortcut/session observations and decoded native layout archives. These establish the scoped behavior above; they do not prove current product implementation or every reference path. No new product executable/build in this contract pass.

## Cursor and text focus contract

Use a transient canvas pointer with three original drawings: arrow (100×231 points), open hand (66×67), and pointing finger (65×84). Keep the hotspot at the top center. The arrow rotates toward the scaled page center; finger/hand stay upright. Update immediately with pointer movement, without implicit position animation. Hide over non-canvas regions and modal settings; restore the ordinary system cursor appropriately. The layer must retain link activation and canvas panning.

Tracked links select the finger. A held canvas press establishes a pan start after 150 ms; subsequent movement uses deltas from the original press, and release cancels that state/timer. A tracked link activates through the shared route controller. Preserve the existing stable page/version destination and history rules. Exact original artwork contours, live cursor transitions and panning still need native product acceptance.

Notes editing exposes Save and Cancel. While its text editor owns focus, J/N/C/L/B/M/Z/R/U/S must enter ordinary text rather than changing presentation settings. Native individual-key observation confirms that full sequence; cancelling restores the note and leaves Review, all aids and fit unchanged. No Comments posting or Notes Save acceptance is implied by that probe.

## Settings and framing foundation

The shared presentation module now implements presets, controller-local Custom recall, twelve current settings, validated per-project atomic preference storage and shared page/version framing. It preserves malformed/oversized preferences and retries failed writes. Centering choices are captured with a Custom snapshot; changing centering alone does not update that snapshot. A later Custom recall may therefore restore its earlier exclusions.

Framing uses a separate result type because presentation supports 5% fit while editor zoom starts at 10%. It handles independent shared width/height maxima, excluded current surfaces, markup offsets, actual size and empty geometry. Fourteen focused tests and all-target warnings-denied Clippy pass. These core checks preceded the application integration described at the top of this document.

## Native panel gesture correction

Native review gesture acceptance found and corrected dropped quick presentation-panel resizing. A press can arrive separately from a coalesced movement/release; the app now retains the visible resize handle until release, honoring focus, modal, cancellation, threshold and ownership guards. All26 presentation tests pass, including regressions failing before correction, plus final all-target warnings-denied lint, format, debug build and signature. Native sidebar width300→400, notes height300→400 and navigator width200→260 persist and survive presentation re-entry. Editor callout drag saves only on comment commit; draft Save/Cancel and presentation drag Cancel preserve exact canonical data. Eight saved-state assertions pass. External `native-review-gestures-current/report.md` pins the corrected macOS ARM64 executable `bc5ea8d2314df732dd7a53f2841843da55713ba0f3175bd2d5dad6c3c6f5c301` and both failed intermediate candidates. Both checkpoint app processes are closed; other owners and pending confirmations remain untouched. This establishes sampled native gesture outcomes, not physical held cadence, reference callout parity, every pane, cold-process preference retention or other-platform input. Windows remains held; macOS/Linux distribution refresh remains required. No commit, installation or publication.

