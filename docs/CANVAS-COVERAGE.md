# Canvas outline coverage

Filled triangle meshes use a geometric union before raster coverage, so overlapping triangles do not darken their shared silhouette. Holes are retained. The canvas and thumbnails share this path; text, document geometry and export paths are unchanged.

## Implementation contract

`render/coverage.rs` validates input, normalizes triangle winding, unions with i_overlay and rasterizes with Zeno at twice physical resolution. Averaging four samples produces premultiplied white coverage, tinted by the current painter. Visible tiles are at most 256 physical pixels with two pixels of padding. Ordinary contours remain complete to avoid tile boundary rounding differences. Coordinates outside the 32768 guard are intersected with the padded tile before rasterization.

`render/coverage/cache.rs` retains at most 8 MiB of prepared-geometry data and 32 MiB of texture pixels, with at most 4,096 entries in each cache. The entry caps separately bound metadata and empty tile records; the byte limits count payloads, not allocator overhead or all transient frame allocations. Geometry hashes are checked against actual points/indices. Monotonic geometry IDs prevent stale tile identity after eviction. Tile identity includes physical scale and fractional origin; integer physical-pixel pans reuse textures. Images are displayed at one texel per pixel with nearest sampling; fractional coverage is already encoded in the pixels.

A failed coverage operation falls back to the existing raw mesh path. Tile preparation completes before painting, avoiding partial tile output followed by a duplicate fallback. Invalid geometry, extreme coordinate transitions and unusually large scenes still require broader acceptance. The current glow backend uploads, paints, then frees evicted textures.

## Verified scope

The full all-target suite passed 765 tests, with zero failures and one ignored helper. After an equivalent lint-only iteration adjustment, nine focused coverage tests passed again. All-target Clippy with warnings denied, formatting, diff checks and optimized macOS build passed. Dependency notices cover the 360 locked packages; original product licensing remains GPL-3.0-only.

An independent triangle/pixel area calculation checked 48 cases/49,152 pixels within one alpha level. Forty scenario scale/phase comparisons against the retained full-image diagnostic also differ by at most one level. Tests include overlap, holes, shared edges, guard behavior, cache identity/invalidation, eviction, clipping and opacity/UV placement.

Native macOS checks cover four realistic pages at 20%, a tablet at 24% with pan away/back, and group entry/exit at 100%. The measured thumbnail field edge improves from 0/112 visible columns to 112/112. Checked regions return exactly to their pre-group captured pixels after exit. Captured opacity residuals remain: the largest sampled residual is 10 RGB levels in both previous and current builds. This does not establish exact compositor fidelity or fix general text softness.

A 120-frame headless CPU sample uploaded no new textures on all 116 warm integer-pan frames. Cold preparation was 2.20–2.97 ms; warm medians were 0.019–0.027 ms. These are CPU diagnostics, not GPU or native frame-cadence measurements.

The isolated review candidate is closed. The existing distribution is unchanged; no installation or commit was made for this checkpoint. Remaining acceptance includes display densities/hardware, coordinate-guard transitions in native scenes, large-document performance, broader nested masks/opacity, held drag, typography and native Windows/Linux.

## Dense outline retention

Entry caps allow up to 4,096 geometries and 4,096 tiles while retaining the existing 8 MiB geometry-data and 32 MiB texture-pixel limits. The former lower entry caps could evict a dense page's many small outlines even when their payloads fit within those byte budgets, causing preparation and uploads to repeat on every frame. This changes retention only; silhouette construction, rasterization, tint, clipping and texture sampling are unchanged.

A regression covers 1,260 distinct small outlines at two scales. Repeated frames retain identical drawing commands without new uploads and stay within both byte budgets. Existing tests continue to cover byte-limit eviction, entry-limit eviction including empty tiles, collision checks, fractional origin/scale invalidation, masking and opacity. This is bounded retained storage, not a total process-memory or frame-time guarantee.

The integrated revision passes 805 tests (zero failures, one ignored helper), formatting, all-target warnings-denied Clippy and an optimized macOS build. A 990-control CPU UI probe reduces warm Tiles updates from 53.57 to 14.95 ms; it excludes native event delivery, GPU presentation, persistence and library. Native Fit/100%/Fit and Save preserve document fields; the sampled Fit canvas matches the prior capture at all 731,646 pixels. Native frame cadence and broader performance remain unproven.
