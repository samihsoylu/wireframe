# Individual icon catalog

Status: implemented and checked on macOS; visual fidelity and other-platform acceptance remain open.

The independently licensed icon collection is available as individual library entries. Stable typed identities distinguish these entries from ordinary controls, presets, assets and components. Style-qualified captions distinguish regular, solid and brand variants.

Icons contains individual entries. All retains ordinary control recipes, assets and components without adding thousands of icon choices. Quick Add searches the individual catalog alongside All regardless of the selected library category. Category unions deduplicate identities. Preserve current keyword/name search capabilities; exact result ranking remains a separate refinement.

Each entry creates a fresh ordinary icon control with its selected catalog ID and 24-point size. Use automatic geometry, the common destination policy and one-step Undo. Double-click, Quick Add and drag share the creation recipe; save/reopen retains normal icon properties. Ordinary Icon and Icon and Text controls remain reachable through their existing builtin categories.

Previews use the selected artwork through existing licensed rendering. Render visible tiles and bound the icon scene cache. Avoid repeated full-catalog construction in per-tile eligibility checks. Settings changes invalidate affected previews; invalid identities and stale gestures must fail without changing the document or unrelated pending edits.

Acceptance requires distinct style identities, All/Icons/Quick Add composition, combined categories, keyword search, insertion/Undo/save/reopen, preview/commit agreement, bounded caches and native interaction checks. Existing artwork coverage is incomplete; exposing the current catalog does not close that gap. Native Windows and Linux acceptance remains outstanding.

Implementation uses one insertion transaction with finite-coordinate, identity and destination validation before committing pending edits. The library and Quick Add use the same recipe as drag previews. A 128-entry LRU bounds builtin/icon scenes; the existing texture cache has its own limits. This is not a total process-memory bound. Category descriptors are initialized once, and gesture validation resolves a single icon without constructing the full icon catalog.

The selected 306 tests pass, including category/search, style persistence, 24-point geometry, save/reopen, nested and locked destinations, pending-edit failure recovery, preview agreement/eviction and Quick Add/drag history. Native macOS checks verify library double-click, keyword Quick Add, automated drag release, immediate Undo and fresh-process saved style/geometry. Optimized ARM64 build, all-target warnings-denied Clippy, formatting and signature checks pass.

The strip allocates only visible tiles while preserving its full horizontal scroll width. This fixes the native accessibility failure caused by thousands of offscreen widgets. A focused regression bounds accessibility nodes and verifies visible labels after layout.

Individual icon thumbnails now use a dedicated presentation transform that enlarges the canonical 24-point scene to a centered 46-point view box. The same transform supplies the floating preview; ordinary controls retain their scale cap. Exact off-center source placement remains a fidelity follow-up. Physical held cadence, full artwork coverage, exact search ranking and native Windows/Linux checks remain open.

The dedicated presentation target is established: fit individual icon artwork to a centered 46-point square in the 68 by 46 provider area, allowing enlargement. The floating preview also has a 46-point square view box, independent of canvas zoom; insertion still creates the ordinary 24-point control. This must remain distinct from ordinary controls' proportional-down thumbnail treatment. Tile and floating geometry are implemented together. The physical insertion observation below supplies practical placement evidence; exact pointer-offset geometry remains unmeasured, and the canonical cached scene remains 24 points so insertion and save geometry do not change. Do not infer saved dimensions from the larger preview.

The shared catalog-artwork painter now avoids extra texture resampling at ordinary display sizes; see [Icon rendering](ICON-RENDERING.md). This rendering correction does not change the pending thumbnail sizing or drag-placement contract.

A physical catalog drag now has saved-output evidence: the user inserted an individual icon at 24×24 and reports a visible size transition from the larger grabbed preview to size S on release. It lands within their intended area, with small adjustments afterward. This supports practical placement and the separate preview/insert size contract. It does not establish an exact pointer offset because press/release coordinates and held frames were not recorded. No additional physical repetition is currently requested; retain that measurement limit when implementing the known 46-point preview target.

The individual-icon anchor uses the full 68×46 provider region, mapping horizontal offset into the 46-point floating box. Downward canvas Y is retained as an explicit interim interaction choice; precise off-center equivalence is not claimed. The preview follows the pointer in screen points independently of canvas zoom. Release converts the anchored point into document coordinates, clamps to the canvas edge and creates one 24-point control. Exact native anchor and physical held cadence still need acceptance.

The sizing increment passes 42 focused drag, preview/cache and library tests plus all-target Clippy. An optimized native macOS candidate verifies the larger strip and drag → one 24×24 icon → Save → Undo → Save; the complete copied fixture is byte-identical after Undo. The candidate is closed and no installed release was replaced. These checks do not establish exact off-center source parity, physical held cadence, broader artwork equivalence or other-platform runtime acceptance.
