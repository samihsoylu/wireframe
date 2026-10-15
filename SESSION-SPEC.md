# Wireframe — current state

## Goal and authority

Build a complete, usable offline Rust wireframing product for macOS, Windows and Linux, with close visual and behavioral fidelity to the established specification. The overall goal remains active. The initial investigation/readiness gate was met on 2026-10-13, and the user authorized automatic implementation progression. Continue targeted investigation when fidelity questions arise; do not treat a partial control inventory or passing foundation tests as product completion.

Work directly as Engineer. Preserve user files and explicit deferrals. Use synthetic fixtures for native experiments. Keep reference-specific names, extracted implementation/assets and investigation narratives outside this repository and its Git history. Original code is GPL-3.0-only from first commit 87f11c1; preserve dependency notices. Existing-format import remains deferred.

## Current checkpoint — 2026-10-15

Ungroup now accepts cropped unlocked groups. Dissolving the wrapper reveals hidden content without moving the children; descendant crop and lock state remain intact. Mixed selections retain other controls and locked groups. Sibling ID collisions, interleaved paint order and empty-group paint units are handled, and Undo restores crop, hierarchy, context and selection. See docs/RECURSIVE-EDITOR.md.

Native restart testing also exposed a stale “Saving…” label after saving an unchanged document. Completion receipts now update saved/newer-edits status even when the worker does not emit a separate Saved event. This does not change the saved snapshot or discard newer edits.

The full all-target run passed 532 tests. The final all-app run passes 71, including the additional status regression, covering 533 current tests across runs. Three cropped-group regressions and the status regression failed before their fixes. Final all-target Clippy with warnings denied, formatting, diff checks and self-review pass. Rendered SVG/PNG matches the expected uncropped document; this is scoped internal rendering evidence. Native macOS checks cover cropped Ungroup, retained selection, Undo/Redo, saved geometry and process restart with byte-identical resave. Final build confirms clean-save acknowledgement and a second cropped-group dissolution/Undo. No independent review is claimed.

Final signed development bundle UUID BFC2B407-0A30-3EF9-95C7-37144AAC047A. Canonical executable identity and ad-hoc signature verify. The app runs the saved Cropped groups fixture, Sketch style at 100%, Library hidden, three top-level controls selected. No background builds or tests remain after this checkpoint. Completed history remains in Git and the relevant specifications.

Library activation, full-size transient previews and save/decision gesture boundaries remain in place. Native held smoothness, exact onset and library autoscroll still require stronger observation. Group crop creation/removal UI and broader crop/resize/lock combinations are the next editing task. The overall goal remains active.

## Existing capabilities and limits

The 86-entry catalog, typed inspector, shared canvas/PNG/SVG rendering, recursive groups/components, rich text and project fonts, image transforms, page navigation, history and offline project storage are implemented. Catalog availability does not establish full visual or behavioral acceptance.

Save, Save As, automatic/manual named saving, New/Open/close and local recovery use the ordered background session. Retained working copies and destination archives are available through Recover work. Native checks cover automatic saving, manual Quit/restart recovery, Save As, missing-file close refusal, explicit recreation and immediate automatic-Quit flush. Shutdown cannot cancel an application exit after a save failure; recovery depends on durable local work. Filesystem stalls, final external-write races, path-alias limitations and archive cleanup UI remain unresolved. Forced-crash and power-loss acceptance are not established.

## Next work and unresolved acceptance

1. Continue targeted shared-interaction investigation: native held library-drag cadence, display-pixel alignment and library autoscroll. Six screen points is the current onset threshold, not an exact timing acceptance result. Do not repeat completed preview foundations or claim native held verification from post-release captures.
2. Complete remaining group/component crop, lock and resize combinations, clipping, snapping and alignment behavior. Cropped-group ungrouping is verified; group crop creation/removal UI still needs implementation and native acceptance. Alternate versions, page hierarchy and presentation target/history behavior need broader acceptance.
3. Refine typography, state styling and geometric/visual edge cases against the established specification. Keep original artwork and fonts properly licensed. Legacy shape/rule coverage, image metadata and placeholder/filter behavior remain open where identified in family documents.
4. Complete PDF export, image scale/options, links, notes and font handling. Preserve shared canvas/export geometry.
5. Package and run native macOS, Windows and Linux acceptance and release checks. Large-document profiling, revision-based cache invalidation, forced failures and recovery remain outstanding. No full cross-platform or complete-fidelity claim.

## Durable specifications

- docs/IMPLEMENTATION-PLAN.md: full scope and acceptance discipline.
- docs/LIBRARY-INSERTION.md and docs/CANVAS-SCROLLING.md: pointer behavior and current verification limits.
- docs/SAVING.md, docs/PROJECT-SESSION.md, docs/WORKING-COPIES.md and docs/RECOVERY.md: desktop persistence and failure boundaries.
- docs/DOCUMENT-FORMAT.md and docs/RECURSIVE-EDITOR.md: recursive data and live editing/storage boundaries.
- docs/CATALOG-AND-LAYOUT.md and control-family documents: rendering and property requirements.
- docs/RICH-TEXT.md, docs/PROJECT-FONTS.md and docs/TEXT-ENTRY.md: text and fonts.
- docs/IMAGES.md and docs/COMPONENTS.md: image and component behavior and remaining gaps.

Keep product-only fixtures/evidence local unless selected for distribution. Update this checkpoint and commit completed product work. No unattended supervision beyond active goal execution is implied. The overall goal remains active.
