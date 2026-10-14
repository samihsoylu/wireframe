# Recursive project format

The desktop editor now reads versions 1 and 2 and saves version 2. The recursive project remains authoritative; derived renderer views are never saved.

A project owns pages, component definitions, binary assets and contextual settings. Each page has a stable ID, notes, an optional parent and independent alternate content. Content stores ordered recursive nodes. IDs are unique among siblings; a node path, scoped to page/version/definition, identifies descendants. Resource IDs are unique across pages, alternates, assets and definitions. The allocator advances above all stored IDs.

A node has a local origin, lock/markup flags, optional normalized crop and sparse typed properties. Its content is a leaf control with requested dimensions, a group of children or a component reference with descendant overrides. Automatic dimensions are represented explicitly and never confused with measured layout. Derived bounds, rendering caches and selection are not persisted.

Component expansion applies inner overrides before outer overrides. Reset removes the sparse override; source edits then propagate. Expansion returns an owned group that can be retained independently of the source. Obsolete paths produce diagnostics without applying their values to a different node. Cycles are errors. Missing definition IDs remain valid dangling references and are reserved below next_id; zero IDs or references colliding with another resource type are invalid. Resolution produces a disposable150×150 placeholder with crop cleared, without changing the canonical instance or its overrides. Overrides under the missing source are reported as orphaned. Expansion has a control-count budget; validation checks hierarchy depth independently of definition order.

Version-1 migration retains all existing properties, geometry, IDs, groups, links and text. Group children become parent-local. When old logical groups interleave in drawing order, an explicit structural paint-path order preserves their original layering; ordinary recursive order remains the default. Component instances are atomic paint paths; layout maps expanded descendants to their instance ordering position. Migration does not write its input. Component definitions currently reject this legacy ordering extension; converting such a selection to a component needs an explicit normalization operation in the editor.

Validation bounds file size, asset bytes, hierarchy depth, node count and text sizes. It rejects invalid coordinates, dimensions, crops, IDs, parent cycles and resource references. Known catalog property types and states are checked during validation. Specialized schemas remain incomplete; unknown fields are preserved.

ProjectStore reads either format and writes version 2 using the existing atomic file replacement path. Encoding/validation occurs before any file replacement. A failed validation therefore preserves the previous file. The desktop app uses ProjectRepository/ProjectStore for saves and recovery, retaining exclusive recovery ownership.

## Verification

Ten document tests cover migration, recursive geometry, expansion/overrides, assets/alternates, validation and failure preservation. The complete suite now has 55 passing tests, including recursive workspace and live app workflow checks. Clippy passes with warnings denied.

The read-only check_migration example passed against five saved native test documents, including the live editor's derived controls. It verifies order, geometry, text, round trips and unchanged source bytes. A native macOS nested-edit/save/close/reopen check also passed; see RECURSIVE-EDITOR.md. Full rendering fidelity and other-platform runtime checks remain open.
