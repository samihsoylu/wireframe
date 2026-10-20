# Finishing checklist

Current checkpoint: 21 October 2026, bounded divider correction; catalog appearance fixes remain open. Unchanged behavior evidence retains its named runtime baseline. This is the short work queue; [the release contract](RELEASE-CHECKLIST.md) retains the full requirements. Windows native testing is on hold. A recorded check applies to its named candidate and behavior, not every possible variant.

## Immediate priorities

- [x] **Q1 — Catalog labels and order:** all 94 built-in choices reconciled; saved identities, old search terms and category memberships retained. All 27 requested labels observed across the live strip.
- [ ] **Q2 — Interface and control appearance:** comparison recorded for wide/narrow editors and all 27 default choices in both styles. The minimum client width is corrected to 920 points. Close V01–V05 below; retain the existing local icon geometry checks.
- [ ] **Q3 — Android Alert tile:** close the specific insertion/recipe/save/Undo gap.
- [ ] **Q4 — Remaining control variants:** reconcile later evidence for Dropdown icons/scrollbars, Date picker destinations and brace links before scheduling new checks.
- [ ] **Q5 — Divider and dragging:** establish the remaining hit-area/persistence/off-center placement behavior without repeating the accepted smooth-preview check.
  Physical divider resizing is demonstrated by the user; native accessibility checks confirm 130–400-point clamping and saved limits. Same-process/fixed-geometry reopening changes 400→344 and then remains stable; native window resizing also changes the saved height. Reconcile these sizing differences before claiming parity. The product's internal grab area is corrected from six to five points, with 17 affected inspector checks and native ±2-point accepted/±3-point ignored grabs; other panel targets and drawing history are preserved. Physical reference-edge acceptance remains open. Automated checks cover 24 normal/icon off-center drops through zoom, pan, nested groups and edge clamping, file round-trip/Undo/Redo, plus visible-divider double-click preservation. These checks do not replace native off-center or modified-gesture acceptance.
- [ ] **Q6 — Four workflows:** refresh only affected routes, then verify one current landing/contact/phone/tablet workflow candidate.
- [ ] **Q7 — Release requirements:** classify every remaining requirement as evidenced, open or explicitly deferred; keep material discrepancies open.
- [ ] **Q8 — Delivery:** refresh macOS/Linux packages and remaining installer/update checks after behavior/appearance closure.
**Held — Q9 Windows:** unverified; excluded from the active work queue until requested. Seven work packages remain active; they are not eight test-suite reruns.

## Visible controls: existing evidence

All 27 listed choices are implemented (26 control families plus the Android Alert preset). 26 have recorded scoped native behavior checks; the separate preset insertion result remains to establish. The current appearance comparison covers all 27 default choices in Sketch and Clean (54 pairs). It identifies differences; it does not establish complete visual equivalence or every variant.

| Expected visible choice | Catalog name | Existing scoped behavior | Next scope |
|---|---|---|---|
| Accordion | Accordion | Recorded native behavior; source/tests | Q2 |
| Alert Box | Alert Box | Recorded native behavior; source/tests | Q2 |
| Alert Box (Android) | Alert Box (Android) | Recipe tests; native family behavior | Q3 |
| App Bar | App Bar | Recorded native behavior; source/tests | Q2 |
| Arrow | Arrow | Recorded native behavior; source/tests | Q5 |
| Breadcrumbs | Breadcrumbs | Recorded native behavior; source/tests | Q2 |
| Browser Window | Browser Window | Recorded native behavior; source/tests | Q2 |
| Button | Button | Recorded native behavior; source/tests | Q2 |
| Button Bar | Button Bar | Recorded native behavior; source/tests | Q2 |
| Calendar | Calendar | Recorded native behavior; source/tests | Q2 |
| Callout | Callout | Recorded native behavior; source/tests | Q2 |
| Chart: Bar | Chart: Bar | Recorded native behavior; source/tests | Q2 |
| Chart: Column | Chart: Column | Recorded native behavior; source/tests | Q2 |
| Chart: Line | Chart: Line | Recorded native behavior; source/tests | Q2 |
| Chart: Pie | Chart: Pie | Recorded native behavior; source/tests | Q2 |
| Checkbox | Checkbox | Recorded native behavior; source/tests | Q2 |
| Checkbox Group | Checkbox Group | Recorded native behavior; source/tests | Q2 |
| Circle Button | Circle Button | Recorded native behavior; source/tests | Q2 |
| Color Picker | Color Picker | Recorded native behavior; source/tests | Q2 |
| ComboBox | ComboBox | Recorded native behavior; source/tests | Q4 |
| Comment | Comment | Recorded native behavior; source/tests | Q2 |
| Cover Flow | Cover Flow | Recorded native behavior; source/tests | Q2 |
| Data Grid | Data Grid | Recorded native behavior; source/tests | Q2 |
| Date Chooser | Date Chooser | Recorded native behavior; source/tests | Q2 |
| Date Picker | Date Picker | Recorded native behavior; source/tests | Q4 |
| Field Set | Field Set | Recorded native behavior; source/tests | Q2 |
| H.Curly Brace | H.Curly Brace | Recorded native behavior; source/tests | Q4 |

## Current appearance fixes — Q2

The wide pair uses the same 1728-point width and 2x density. Narrow native frames measure 920 and 921 points; the remaining one-point frame difference is explicit. Both show unselected controls at 100%. Default fonts differ. Native image exports provide lossless detail; overview sheets are scaled and cannot establish sharpness by themselves.

| Item | Observed difference | Closure check |
|---|---|---|
| V01 — Shell layout | Category group is left of the full window center; the last category needs scrolling at narrow widths. The native title bar uses a different appearance. | Check centered categories, compact labels, reachable final category and title-bar treatment in the same two window sizes. |
| V02 — Starter content | Accordion, Alert choices, Browser Window, ComboBox and Data Grid use different starter content; Alert and grid initial sizes consequently differ. Library Calendar preview content also differs. | Reconcile neutral starter structures and preview rules; refresh only the affected specimens and strip captions. |
| V03 — Typography and clarity | Font shape, weight, text widths and raster appearance differ. | Isolate font metrics and canvas texture scaling using original-size native crops; verify the affected text and icons at 100%. |
| V04 — Sketch contours | Cover Flow, Callout, brace and several container/icon contours differ in weight, shape or shadow. | Correct original drawing geometry and refresh the affected Sketch crops. |
| V05 — Small glyphs | Circle Button plus, browser commands and date icon have visible silhouette/weight differences. | Compare original-size crops after changing only the affected original artwork. |

The default Arrow is straight in both products once the comparison fixture includes creation properties. Both calendars have the same cramped final default row; this is not a newly confirmed defect. Initial Arrow/Circle fixture errors are excluded from conclusions. The separate Android preset behavior task remains Q3. Appearance fixes do not reopen established editing/history workflows unless their implementation changes.

## Check ownership

Every new check must name its unanswered question, reused evidence, reason another observation is needed, candidate identity and closure condition. A completed check is repeated only after a specific relevant change or a demonstrated failure. Documentation-only updates do not trigger runtime suites. Historical text widths and visual differences are remeasured in the shared current comparison rather than treated as current defects automatically.

Detailed evidence is retained privately. The complete product remains in progress; this list adds no new feature or platform deferrals.
