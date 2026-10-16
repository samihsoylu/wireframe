# Wireframe — current state

## Goal and authority

Build a complete, usable offline Rust wireframing product for macOS, Windows and Linux, with close visual and behavioral fidelity to the established specification. The overall goal remains active. The initial investigation/readiness gate was met on 2026-10-13, and the user authorized automatic implementation progression. Continue targeted investigation when fidelity questions arise; do not treat a partial control inventory or passing foundation tests as product completion.

Work directly as Engineer. Preserve user files and explicit deferrals. Use synthetic fixtures for native experiments. Keep reference-specific names, extracted implementation/assets and investigation narratives outside this repository and its Git history. Original code is GPL-3.0-only from first commit 87f11c1; preserve dependency notices. Existing-format import remains deferred.

## Current checkpoint — Save As interruption and archive recovery

The macOS working-copy suite passes26 tests, including24 Save As process-termination combinations across named/unnamed sources, new/existing destinations and six handoff boundaries. The prior eight interruption cases remain covered; one child helper is intentionally ignored outside its parent tests. Source work survives until the destination is acknowledged; named source files stay unchanged; previous destination edits remain recoverable from immutable archives. Nine background-session integration tests and all-target Clippy also pass. Production saving behavior is unchanged.

Native macOS Save As replaced a disposable destination with retained local edits, archiving its prior working record byte-for-byte. Recover work displayed the previous copy; Restore opened unnamed work, and saving it separately preserved both the archive and the destination. All75 pre-existing profile files and the automatic-save preference are unchanged;7 synthetic files were added. Candidate closed, installed app unchanged, task VM remains stopped.

The interruption tests drive storage APIs in the session's handoff order; they do not pause every possible worker interleaving or kill inside atomic writes/archive renames. Power loss and native Windows/Linux crash behavior remain open. Scoped recovery acceptance is recorded in docs/RECOVERY.md. Next return to nested group/component crop, lock and resize fidelity. Broader canvas softness, held drag cadence, typography/PDF, platform/runtime and release gaps remain open. Engineer self-review only; the full goal remains active.

## Existing capabilities and limits

The 86-entry catalog, typed inspector, shared canvas/PNG/SVG rendering, recursive groups/components, rich text and project fonts, image transforms, page navigation, history and offline project storage are implemented. Catalog availability does not establish full visual or behavioral acceptance.

Save, Save As, automatic/manual named saving, New/Open/close and local recovery use the ordered background session. Retained working copies and destination archives are available through Recover work. Native checks cover automatic saving, manual Quit/restart recovery, Save As, missing-file close refusal, explicit recreation and immediate automatic-Quit flush. Shutdown cannot cancel an application exit after a save failure; recovery depends on durable local work. Filesystem stalls, final external-write races, path-alias limitations and archive cleanup UI remain unresolved. Scoped macOS process-termination checks pass as described above; broader crash and power-loss acceptance remain open.

## Next work and unresolved acceptance

Keep the four integrated scenarios as repeatable acceptance fixtures. Keep the five-page table-based order variant alongside the original labels/fields version for integrated table acceptance. Continue control-specific palette/contrast, typography and constrained-wrapping comparisons while preserving distinct regular, bold, italic and bold-italic roles. Continue device geometry and remaining drawing style details against the established specification. Continue canvas/export and native interaction checks without treating catalog coverage as complete fidelity.

1. Broaden display-hardware acceptance for canvas pixel alignment; retain the demonstrated extra-filtering correction and distinguish remaining font/capture differences. Continue targeted shared-interaction investigation: native held library-drag cadence, display-pixel alignment and library autoscroll. Six screen points is the current onset threshold, not an exact timing acceptance result. Linux sampled held preview is verified; retain macOS/reference cadence and broader hardware gaps. Do not infer held verification from post-release captures.
2. Complete remaining group/component crop, lock and resize combinations, clipping, snapping and alignment behavior. Cropped-group ungrouping and group crop creation/adjustment/removal UI have scoped native acceptance; broader nested component combinations remain. Alternate versions, page hierarchy and presentation target/history behavior need broader acceptance.
3. Refine typography, state styling and geometric/visual edge cases against the established specification. Keep original artwork and fonts properly licensed. Legacy shape/rule coverage, image metadata and placeholder/filter behavior remain open where identified in family documents. Explicit stored Text Input heights remain fixed, while interactive insertion/resizing uses intrinsic auto height; reconcile that loading contract separately before any compatibility claim.
4. Complete PDF typography and remaining native failure/performance acceptance, image scale/options and broader font handling. Preserve shared canvas/export geometry.
5. Package and run native macOS, Windows and Linux acceptance and release checks. Large-document profiling, revision-based cache invalidation, forced failures and recovery remain outstanding. No full cross-platform or complete-fidelity claim.

## Durable specifications

- docs/IMPLEMENTATION-PLAN.md: full scope and acceptance discipline.
- docs/LIBRARY-INSERTION.md and docs/CANVAS-SCROLLING.md: pointer behavior and current verification limits.
- docs/SAVING.md, docs/PROJECT-SESSION.md, docs/WORKING-COPIES.md and docs/RECOVERY.md: desktop persistence and failure boundaries.
- docs/DOCUMENT-FORMAT.md and docs/RECURSIVE-EDITOR.md: recursive data and live editing/storage boundaries.
- docs/CATALOG-AND-LAYOUT.md and control-family documents: rendering and property requirements.
- docs/RICH-TEXT.md, docs/PROJECT-FONTS.md and docs/TEXT-ENTRY.md: text and fonts.
- docs/OUTLINES.md: stable corners, rectangle variants, shared geometry and current fidelity limits.
- docs/PROJECT-COLORS.md: project palette, contrast, history and new-project defaults.
- docs/PDF-EXPORT.md: page selection, sizing, notes, text/link fidelity, architecture boundaries and scoped integrated acceptance.
- docs/IMAGES.md and docs/COMPONENTS.md: image and component behavior and remaining gaps.

Keep product-only fixtures/evidence local unless selected for distribution. Update this checkpoint and commit completed product work. No unattended supervision beyond active goal execution is implied. The overall goal remains active.
