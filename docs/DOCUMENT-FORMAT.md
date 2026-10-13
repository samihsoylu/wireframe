# Recursive project format

Version 2 is implemented as an opt-in document/storage API while the current desktop editor continues to read and write version 1. Do not switch application saving until editor integration is complete.

A project owns pages, component definitions, binary assets and contextual settings. Each page has a stable ID, notes, an optional parent and independent alternate content. Content stores ordered recursive nodes. IDs are unique among siblings; a node path, scoped to page/version/definition, identifies descendants. Resource IDs are unique across pages, alternates, assets and definitions. The allocator advances above all stored IDs.

A node has a local origin, lock/markup flags, optional normalized crop and sparse typed properties. Its content is a leaf control with requested dimensions, a group of children or a component reference with descendant overrides. Automatic dimensions are represented explicitly and never confused with measured layout. Derived bounds, rendering caches and selection are not persisted.

Component expansion applies inner overrides before outer overrides. Reset removes the sparse override; source edits then propagate. Expansion returns an owned group that can be retained independently of the source. Obsolete paths produce diagnostics without applying their values to a different node. Cycles and missing definitions are errors. Expansion has a control-count budget; validation checks hierarchy depth independently of definition order.

Version-1 migration retains all existing properties, geometry, IDs, groups, links and text. Group children become parent-local. When old logical groups interleave in drawing order, an explicit leaf-path order preserves their original layering; ordinary recursive order remains the default. Migration does not write its input. Component definitions currently reject this legacy ordering extension; converting such a selection to a component needs an explicit normalization operation in the editor.

Validation bounds file size, asset bytes, hierarchy depth, node count and text sizes. It rejects invalid coordinates, dimensions, crops, IDs, parent cycles and resource references. Catalog-specific enum and property schemas will add semantic validation during catalog integration; the current typed value representation alone does not establish every control's valid states.

ProjectStore reads either format and writes version 2 using the existing atomic file replacement path. Encoding/validation occurs before any file replacement. A failed validation therefore preserves the previous file. The application's recovery ownership and version-1 store remain unchanged until integration.

## Verification

Nine document tests cover full legacy-kind migration and interleaved layering, nested movement/ungrouping, nested override precedence/reset/propagation/detachment, stale paths, cyclic/missing references, expansion growth, assets/alternates/context values, failed save preservation, malformed geometry/order and depth limits. The existing 14 application/workflow tests still pass. `cargo clippy --locked --all-targets -- -D warnings` passes.

The read-only `check_migration` example also passed against five saved native test documents, verifying order, positions, text, round trips and unchanged source bytes. These are document-model and storage checks; no version-2 native editor or rendering behavior is claimed yet.
