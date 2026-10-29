// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Pure resource planning followed by a guarded, single-history application.
use super::*;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LibraryImportChoice {
    UseNew,
    UseExisting,
    KeepBoth,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LibraryImportAction {
    Add,
    Reuse,
    Conflict,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibraryImportEntry {
    pub incoming: Id,
    pub name: String,
    pub existing: Option<Id>,
    pub requested: bool,
    pub action: LibraryImportAction,
    /// Some incoming orphan paths need a separate copy of this resource when
    /// their referring resources are imported. Decisions are rechecked together.
    pub has_unmapped_overrides: bool,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LibraryImportCounts {
    pub added: usize,
    pub replaced: usize,
    pub reused: usize,
}
#[derive(Clone)]
pub(in crate::workspace) struct ProjectImportContext {
    pub resources: HashMap<Id, Id>,
    pub surfaces: Vec<(Id, Content)>,
}
#[derive(Clone)]
pub struct LibraryImportPlan {
    context: Option<ProjectImportContext>,
    base: Project,
    payload: LibraryClipboard,
    same_project: bool,
    origin: String,
    entries: Vec<LibraryImportEntry>,
}
pub struct PreparedLibraryImport {
    origin: String,
    base: Project,
    pub(in crate::workspace) after: Project,
    pub(in crate::workspace) counts: LibraryImportCounts,
    newly_orphaned_overrides: usize,
}
impl PreparedLibraryImport {
    /// Newly unresolved override occurrences across editing surfaces. The import
    /// UI must expose this impact before applying a destructive source choice.
    pub fn newly_orphaned_overrides(&self) -> usize {
        self.newly_orphaned_overrides
    }
}
impl Editor {
    pub fn plan_library_import(
        &self,
        clipboard: &Clipboard,
        origin: &str,
    ) -> Result<LibraryImportPlan, String> {
        LibraryImportPlan::with_context(&self.document, clipboard, origin, None)
    }
    pub fn apply_library_import(
        &mut self,
        prepared: PreparedLibraryImport,
        origin: &str,
    ) -> Result<LibraryImportCounts, String> {
        if self.document != prepared.base || origin != prepared.origin {
            return Err(
                "Project changed while import was being prepared; plan the import again".into(),
            );
        }
        if self.document == prepared.after {
            return Ok(prepared.counts);
        }
        self.component_change("Paste component libraries", |editor| {
            editor.document = prepared.after;
            Ok(prepared.counts)
        })
    }
}
impl LibraryImportPlan {
    pub(in crate::workspace) fn with_context(
        base: &Project,
        clipboard: &Clipboard,
        origin: &str,
        context: Option<ProjectImportContext>,
    ) -> Result<LibraryImportPlan, String> {
        if context.is_none() {
            base.validate()?;
        }
        validate_origin(origin)?;
        let payload = clipboard.library_resources()?.clone();
        let same_project = context.is_none() && payload.project == origin;
        let mut claimed = HashSet::new();
        let mut entries: Vec<_> = payload
            .libraries
            .iter()
            .map(|library| {
                let requested = payload.requested.contains(&library.id);
                let existing = if same_project {
                    (!requested)
                        .then(|| base.live_libraries().find(|l| l.id == library.id))
                        .flatten()
                } else {
                    base.live_libraries().find(|l| {
                        !claimed.contains(&l.id)
                            && l.imports
                                .iter()
                                .any(|i| i.project == payload.project && i.library == library.id)
                    })
                }
                .map(|l| l.id);
                if let Some(id) = existing {
                    claimed.insert(id);
                }
                LibraryImportEntry {
                    incoming: library.id,
                    name: library.name.clone(),
                    existing,
                    requested,
                    action: LibraryImportAction::Add,
                    has_unmapped_overrides: false,
                }
            })
            .collect();
        // Reserve all exact origin matches before considering weaker name matches.
        if !same_project {
            for entry in entries.iter_mut().filter(|e| e.existing.is_none()) {
                if let Some(library) = base
                    .live_libraries()
                    .find(|l| !claimed.contains(&l.id) && l.name == entry.name)
                {
                    entry.existing = Some(library.id);
                    claimed.insert(library.id);
                }
            }
        }
        let mut plan = LibraryImportPlan {
            context,
            base: base.clone(),
            payload,
            same_project,
            origin: origin.into(),
            entries,
        };
        let actions: Vec<_> = plan
            .entries
            .iter()
            .map(|e| {
                if e.existing.is_some() && same_project {
                    Apply::Reuse
                } else if e.existing.is_some() {
                    Apply::Replace
                } else {
                    Apply::Add
                }
            })
            .collect();
        let proposed = plan.build(&actions)?;
        for entry in &mut plan.entries {
            let incoming = plan
                .payload
                .libraries
                .iter()
                .find(|l| l.id == entry.incoming)
                .unwrap();
            entry.has_unmapped_overrides = incoming
                .sources
                .iter()
                .any(|s| proposed.unmapped_sources.contains(&s.definition));
            if let Some(id) = entry.existing {
                entry.action = if entry.has_unmapped_overrides {
                    LibraryImportAction::Conflict
                } else if same_project
                    || equivalent_library_content(&plan.base, &proposed.document, id)
                {
                    LibraryImportAction::Reuse
                } else {
                    LibraryImportAction::Conflict
                };
            }
        }
        // If Keep Both could change a dependency identity, its existing parents
        // need an explicit decision too. Never rewrite an otherwise reused parent silently.
        if !same_project {
            let owners: HashMap<_, _> = plan
                .payload
                .libraries
                .iter()
                .flat_map(|l| l.sources.iter().map(move |s| (s.definition, l.id)))
                .collect();
            loop {
                let conflicts: HashSet<_> = plan
                    .entries
                    .iter()
                    .filter(|e| e.action == LibraryImportAction::Conflict)
                    .map(|e| e.incoming)
                    .collect();
                let mut changed = false;
                for entry in &mut plan.entries {
                    if entry.action != LibraryImportAction::Reuse {
                        continue;
                    }
                    let library = plan
                        .payload
                        .libraries
                        .iter()
                        .find(|l| l.id == entry.incoming)
                        .unwrap();
                    let mut refs = HashSet::new();
                    clipboard::references(&library.content.nodes, &mut refs, &mut HashSet::new());
                    for definition in plan
                        .payload
                        .definitions
                        .iter()
                        .filter(|d| library.sources.iter().any(|s| s.definition == d.id))
                    {
                        clipboard::references(
                            &definition.content.nodes,
                            &mut refs,
                            &mut HashSet::new(),
                        );
                    }
                    if refs.iter().any(|id| {
                        owners
                            .get(id)
                            .is_some_and(|owner| *owner != library.id && conflicts.contains(owner))
                    }) {
                        entry.action = LibraryImportAction::Conflict;
                        changed = true;
                    }
                }
                if !changed {
                    break;
                }
            }
        }
        Ok(plan)
    }
    /// Dialog decisions are valid only for the exact destination they reviewed.
    pub fn matches_destination(&self, document: &Project, origin: &str) -> bool {
        self.base == *document && self.origin == origin
    }
    pub fn entries(&self) -> &[LibraryImportEntry] {
        &self.entries
    }
    /// Omitting a conflict choice leaves the plan unresolved. Cancellation drops the plan.
    pub fn prepare(
        &self,
        choices: &BTreeMap<Id, LibraryImportChoice>,
    ) -> Result<PreparedLibraryImport, String> {
        if choices.keys().any(|id| {
            !self
                .entries
                .iter()
                .any(|e| e.incoming == *id && e.action == LibraryImportAction::Conflict)
        }) {
            return Err("Choice does not identify a pending library conflict".into());
        }
        let mut counts = LibraryImportCounts::default();
        let actions = self
            .entries
            .iter()
            .map(|entry| {
                let action = match entry.action {
                    LibraryImportAction::Add => Apply::Add,
                    LibraryImportAction::Reuse => Apply::Reuse,
                    LibraryImportAction::Conflict => match choices
                        .get(&entry.incoming)
                        .ok_or("Choose how to resolve every library conflict")?
                    {
                        LibraryImportChoice::UseNew => Apply::Replace,
                        LibraryImportChoice::UseExisting => Apply::Reuse,
                        LibraryImportChoice::KeepBoth => Apply::Add,
                    },
                };
                match action {
                    Apply::Add => counts.added += 1,
                    Apply::Replace => counts.replaced += 1,
                    Apply::Reuse => counts.reused += 1,
                }
                Ok(action)
            })
            .collect::<Result<Vec<_>, String>>()?;
        let after = if self.context.is_none() && actions.iter().all(|a| *a == Apply::Reuse) {
            self.base.clone()
        } else {
            let built = self.build(&actions)?;
            if !built.unmapped_sources.is_empty() {
                return Err("Some edits refer to missing controls whose identities changed. Keep Both for the related libraries, or Use Existing for the resources containing those edits".into());
            }
            built.document
        };
        after.validate()?;
        for (entry, action) in self.entries.iter().zip(&actions) {
            if *action == Apply::Replace
                && equivalent_library(&self.base, &after, entry.existing.unwrap())
            {
                counts.replaced -= 1;
                counts.reused += 1;
            }
        }
        let newly_orphaned_overrides = if after == self.base {
            0
        } else {
            unresolved_overrides(&after)?
                .difference(&unresolved_overrides(&self.base)?)
                .count()
        };
        Ok(PreparedLibraryImport {
            origin: self.origin.clone(),
            base: self.base.clone(),
            after,
            counts,
            newly_orphaned_overrides,
        })
    }
    fn build(&self, actions: &[Apply]) -> Result<BuiltImport, String> {
        let mut arena = self.base.clone();
        // Scoped node and override identities remain stable; reserve them before
        // allocating resource IDs, including when the clipboard outlives Undo.
        let mut maximum = 0;
        for library in &self.payload.libraries {
            maximum = maximum
                .max(library.id)
                .max(maximum_node_id(&library.content.nodes, 0)?);
        }
        for definition in &self.payload.definitions {
            maximum = maximum
                .max(definition.id)
                .max(maximum_node_id(&definition.content.nodes, 0)?);
        }
        // Existing orphan paths also reserve scoped identities: allocating a new
        // child with one of those numbers would revive an unrelated old edit.
        for nodes in self
            .base
            .pages
            .iter()
            .map(|p| &p.content.nodes)
            .chain(
                self.base
                    .pages
                    .iter()
                    .flat_map(|p| p.alternates.iter().map(|a| &a.content.nodes)),
            )
            .chain(self.base.definitions.iter().map(|d| &d.content.nodes))
            .chain(self.base.libraries.iter().map(|l| &l.content.nodes))
        {
            maximum = maximum.max(maximum_node_id(nodes, 0)?);
        }
        arena.next_id = arena
            .next_id
            .max(maximum.checked_add(1).ok_or("Project ID space exhausted")?);
        let occupied = resource_ids(&self.base);
        let mut hints: HashMap<Id, Option<Id>> = HashMap::new();
        for (entry, action) in self.entries.iter().zip(actions) {
            if *action == Apply::Add {
                continue;
            }
            if let Some(record) = self
                .base
                .libraries
                .iter()
                .find(|l| Some(l.id) == entry.existing)
                .and_then(|l| {
                    l.imports
                        .iter()
                        .find(|i| i.project == self.payload.project && i.library == entry.incoming)
                })
            {
                for (&foreign, &local) in &record.definitions {
                    hints
                        .entry(foreign)
                        .and_modify(|known| {
                            if *known != Some(local) {
                                *known = None;
                            }
                        })
                        .or_insert(Some(local));
                }
            }
        }
        let mut assigned_definitions = HashSet::new();
        let mut map = self
            .context
            .as_ref()
            .map(|c| c.resources.clone())
            .unwrap_or_default();
        for (entry, action) in self.entries.iter().zip(actions) {
            let target = if *action != Apply::Add {
                entry.existing.ok_or("Import target is missing")?
            } else if self.same_project && !occupied.contains(&entry.incoming) {
                entry.incoming
            } else {
                arena.allocate_id()?
            };
            map.insert(entry.incoming, target);
            let incoming = self
                .payload
                .libraries
                .iter()
                .find(|l| l.id == entry.incoming)
                .unwrap();
            let existing = (*action != Apply::Add)
                .then(|| {
                    self.base
                        .libraries
                        .iter()
                        .find(|l| Some(l.id) == entry.existing)
                })
                .flatten();
            let correspondence = existing.and_then(|l| {
                l.imports
                    .iter()
                    .find(|i| i.project == self.payload.project && i.library == incoming.id)
            });
            for source in &incoming.sources {
                let historical = correspondence
                    .and_then(|record| record.definitions.get(&source.definition).copied());
                let current = historical.filter(|id| {
                    existing.is_some_and(|l| l.sources.iter().any(|s| s.definition == *id))
                });
                let by_name = || {
                    existing.and_then(|library| {
                        let name = &self
                            .payload
                            .definitions
                            .iter()
                            .find(|d| d.id == source.definition)
                            .unwrap()
                            .name;
                        let matches: Vec<_> = library
                            .sources
                            .iter()
                            .filter_map(|s| {
                                self.base
                                    .definitions
                                    .iter()
                                    .find(|d| d.id == s.definition && &d.name == name)
                            })
                            .collect();
                        (matches.len() == 1).then(|| matches[0].id)
                    })
                };
                let known = if self.same_project && *action != Apply::Add {
                    Some(source.definition)
                } else {
                    // Source Cut/Paste can retire a recorded identity. Prefer a
                    // current owner before restoring an absent historical target.
                    current.or_else(by_name).or(historical)
                };
                let known = known.or_else(|| {
                    ((*action != Apply::Add || entry.existing.is_none()) && !self.same_project)
                        .then(|| {
                            hints
                                .get(&source.definition)
                                .copied()
                                .flatten()
                                .filter(|id| !occupied.contains(id))
                        })
                        .flatten()
                });
                let known = known.filter(|id| {
                    !assigned_definitions.contains(id)
                        && (!occupied.contains(id)
                            || (self.same_project
                                && self.base.definitions.iter().any(|d| d.id == *id))
                            || existing
                                .is_some_and(|l| l.sources.iter().any(|s| s.definition == *id)))
                });
                let target = if let Some(id) = known {
                    id
                } else if self.same_project
                    && *action == Apply::Add
                    && !occupied.contains(&source.definition)
                {
                    source.definition
                } else {
                    arena.allocate_id()?
                };
                assigned_definitions.insert(target);
                map.insert(source.definition, target);
            }
        }
        let mut new_removed_assets = HashSet::new();
        for id in &self.payload.removed_assets {
            if map.contains_key(id) {
                continue;
            }
            let target = if self.same_project && self.base.removed_assets.contains(id) {
                *id
            } else {
                arena.allocate_id()?
            };
            map.insert(*id, target);
            new_removed_assets.insert(target);
        }
        let mut new_assets = vec![];
        for asset in &self.payload.assets {
            if map.contains_key(&asset.id) {
                continue;
            }
            let id = if let Some(existing) =
                self.base.assets.iter().chain(new_assets.iter()).find(|a| {
                    a.trashed == asset.trashed
                        && a.media_type == asset.media_type
                        && a.bytes == asset.bytes
                        && a.attribution == asset.attribution
                }) {
                existing.id
            } else {
                let mut copy = asset.clone();
                copy.id = arena.allocate_id()?;
                let id = copy.id;
                new_assets.push(copy);
                id
            };
            map.insert(asset.id, id);
        }
        let mut references = HashSet::new();
        for nodes in self
            .payload
            .libraries
            .iter()
            .map(|l| &l.content.nodes)
            .chain(self.payload.definitions.iter().map(|d| &d.content.nodes))
            .chain(
                self.context
                    .iter()
                    .flat_map(|c| c.surfaces.iter().map(|(_, content)| &content.nodes)),
            )
        {
            clipboard::references(nodes, &mut references, &mut HashSet::new());
        }
        let mut missing: Vec<_> = references
            .into_iter()
            .filter(|id| !map.contains_key(id))
            .collect();
        missing.sort_unstable();
        for id in missing {
            let target = if self.same_project
                && (!occupied.contains(&id) || self.base.definitions.iter().any(|d| d.id == id))
            {
                id
            } else if let Some(target) = hints
                .get(&id)
                .copied()
                .flatten()
                .filter(|id| !occupied.contains(id) && !assigned_definitions.contains(id))
            {
                target
            } else {
                arena.allocate_id()?
            };
            assigned_definitions.insert(target);
            map.insert(id, target);
        }
        let pages: HashSet<_> = if self.same_project || self.context.is_some() {
            self.base
                .pages
                .iter()
                .map(|p| p.id)
                .chain(self.base.removed_pages.iter().copied())
                .collect()
        } else {
            HashSet::new()
        };
        let mut names: HashSet<_> = self
            .base
            .libraries
            .iter()
            .map(|l| l.name.to_lowercase())
            .collect();
        let trusted: HashSet<_> = self
            .payload
            .libraries
            .iter()
            .flat_map(|library| {
                library.sources.iter().filter_map(|source| {
                    let established = self.same_project
                        || self.base.libraries.iter().any(|current| {
                            current.imports.iter().any(|record| {
                                record.project == self.payload.project
                                    && record.library == library.id
                                    && record.definitions.get(&source.definition)
                                        == map.get(&source.definition)
                            })
                        });
                    established.then_some(source.definition)
                })
            })
            .collect();
        let reused: HashSet<_> = self
            .entries
            .iter()
            .zip(actions)
            .filter(|(_, action)| **action == Apply::Reuse)
            .flat_map(|(entry, _)| {
                self.payload
                    .libraries
                    .iter()
                    .find(|l| l.id == entry.incoming)
                    .unwrap()
                    .sources
                    .iter()
                    .map(|s| s.definition)
            })
            .collect();
        let correspondence = correspondence::Correspondence::build(
            &self.payload,
            &self.base,
            &map,
            correspondence::MatchContext {
                trusted: &trusted,
                reused: &reused,
                pages: &pages,
                surfaces: self
                    .context
                    .as_ref()
                    .map(|c| c.surfaces.as_slice())
                    .unwrap_or_default(),
            },
            &mut arena,
        )?;
        let mut result = self.base.clone();
        result.next_id = arena.next_id;
        let mut needed_assets = HashSet::new();
        for (entry, action) in self.entries.iter().zip(actions) {
            if *action == Apply::Reuse {
                continue;
            }
            let mut library = self
                .payload
                .libraries
                .iter()
                .find(|l| l.id == entry.incoming)
                .unwrap()
                .clone();
            library.id = map[&library.id];
            if *action == Apply::Replace {
                let previous = result
                    .libraries
                    .iter()
                    .find(|l| l.id == library.id)
                    .ok_or("Import target is missing")?;
                library.name = previous.name.clone();
                library.notes = previous.notes.clone();
                library.imports = previous.imports.clone();
                let removed: HashSet<_> = previous.sources.iter().map(|s| s.definition).collect();
                let retained: HashSet<_> =
                    library.sources.iter().map(|s| map[&s.definition]).collect();
                result
                    .definitions
                    .retain(|d| !removed.contains(&d.id) || retained.contains(&d.id));
            } else {
                library.name = unique_name(&library.name, &mut names);
            }
            let mut bindings = BTreeMap::new();
            for source in &mut library.sources {
                let original = source.definition;
                source.definition = map[&original];
                bindings.insert(original, source.definition);
                let mut definition = self
                    .payload
                    .definitions
                    .iter()
                    .find(|d| d.id == original)
                    .unwrap()
                    .clone();
                definition.id = source.definition;
                text_link_transfer::rewrite(
                    &mut definition.content.nodes,
                    &self.payload.definitions,
                    &mut |id| {
                        let id = map.get(&id).copied().unwrap_or(id);
                        pages.contains(&id).then_some(id)
                    },
                )?;
                correspondence.rewrite(&mut definition.content.nodes, Some(original));
                project_copy::rewrite(&mut definition.content.nodes, &map);
                clipboard::strip_missing_links(&mut definition.content.nodes, &pages);
                clipboard::references(
                    &definition.content.nodes,
                    &mut HashSet::new(),
                    &mut needed_assets,
                );
                if let Some(index) = result
                    .definitions
                    .iter()
                    .position(|d| d.id == definition.id)
                {
                    result.definitions[index] = definition;
                } else {
                    result.definitions.push(definition);
                }
            }
            text_link_transfer::rewrite(
                &mut library.content.nodes,
                &self.payload.definitions,
                &mut |id| {
                    let id = map.get(&id).copied().unwrap_or(id);
                    pages.contains(&id).then_some(id)
                },
            )?;
            correspondence.rewrite(&mut library.content.nodes, None);
            project_copy::rewrite(&mut library.content.nodes, &map);
            clipboard::strip_missing_links(&mut library.content.nodes, &pages);
            clipboard::references(
                &library.content.nodes,
                &mut HashSet::new(),
                &mut needed_assets,
            );
            if !self.same_project {
                let incoming = self
                    .payload
                    .libraries
                    .iter()
                    .find(|l| l.id == entry.incoming)
                    .unwrap();
                let mut refs = HashSet::new();
                clipboard::references(&incoming.content.nodes, &mut refs, &mut HashSet::new());
                for definition in self
                    .payload
                    .definitions
                    .iter()
                    .filter(|d| incoming.sources.iter().any(|s| s.definition == d.id))
                {
                    clipboard::references(
                        &definition.content.nodes,
                        &mut refs,
                        &mut HashSet::new(),
                    );
                }
                for missing in refs
                    .into_iter()
                    .filter(|id| !self.payload.definitions.iter().any(|d| d.id == *id))
                {
                    bindings.insert(missing, map[&missing]);
                }
                if let Some(record) = library
                    .imports
                    .iter_mut()
                    .find(|i| i.project == self.payload.project && i.library == entry.incoming)
                {
                    record.definitions.extend(bindings);
                } else {
                    library.imports.push(LibraryImport {
                        project: self.payload.project.clone(),
                        library: entry.incoming,
                        definitions: bindings,
                    });
                }
            }
            if let Some(index) = result.libraries.iter().position(|l| l.id == library.id) {
                result.libraries[index] = library;
            } else {
                result.libraries.push(library);
            }
        }
        if let Some(context) = &self.context {
            for (id, incoming) in &context.surfaces {
                let content = result
                    .pages
                    .iter_mut()
                    .flat_map(|p| {
                        std::iter::once((p.id, &mut p.content))
                            .chain(p.alternates.iter_mut().map(|a| (a.id, &mut a.content)))
                    })
                    .find(|(target, _)| target == id)
                    .map(|(_, content)| content)
                    .ok_or("Import surface is missing")?;
                *content = incoming.clone();
                text_link_transfer::rewrite(
                    &mut content.nodes,
                    &self.payload.definitions,
                    &mut |id| {
                        let id = map.get(&id).copied().unwrap_or(id);
                        pages.contains(&id).then_some(id)
                    },
                )?;
                correspondence.rewrite(&mut content.nodes, None);
                project_copy::rewrite(&mut content.nodes, &map);
                clipboard::strip_missing_links(&mut content.nodes, &pages);
            }
            // Reuse also establishes qualified correspondence for later imports.
            for (entry, action) in self.entries.iter().zip(actions) {
                if *action != Apply::Reuse {
                    continue;
                }
                let incoming = self
                    .payload
                    .libraries
                    .iter()
                    .find(|l| l.id == entry.incoming)
                    .unwrap();
                let library = result
                    .libraries
                    .iter_mut()
                    .find(|l| l.id == map[&entry.incoming])
                    .unwrap();
                let bindings = incoming
                    .sources
                    .iter()
                    .map(|s| (s.definition, map[&s.definition]))
                    .collect();
                if let Some(record) = library
                    .imports
                    .iter_mut()
                    .find(|i| i.project == self.payload.project && i.library == entry.incoming)
                {
                    record.definitions.extend(bindings);
                } else {
                    library.imports.push(LibraryImport {
                        project: self.payload.project.clone(),
                        library: entry.incoming,
                        definitions: bindings,
                    });
                }
            }
        }
        result.removed_assets.extend(
            new_removed_assets
                .into_iter()
                .filter(|id| needed_assets.contains(id)),
        );
        result.assets.extend(
            new_assets
                .into_iter()
                .filter(|a| needed_assets.contains(&a.id)),
        );
        if result.default_library.is_none() {
            result.default_library = result.libraries.first().map(|l| l.id);
        }
        let allocated = result.next_id;
        result.next_id = self.base.next_id;
        let unmapped_sources = correspondence.unmapped_sources();
        if result == self.base {
            return Ok(BuiltImport {
                document: result,
                unmapped_sources,
            });
        }
        result.next_id = allocated;
        if self.context.is_none() {
            result.validate()?;
        }
        Ok(BuiltImport {
            document: result,
            unmapped_sources,
        })
    }
}
struct BuiltImport {
    document: Project,
    unmapped_sources: HashSet<Id>,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Apply {
    Add,
    Replace,
    Reuse,
}
fn resource_ids(project: &Project) -> HashSet<Id> {
    project
        .pages
        .iter()
        .map(|p| p.id)
        .chain(
            project
                .pages
                .iter()
                .flat_map(|p| p.alternates.iter().map(|a| a.id)),
        )
        .chain(project.versions.iter().map(|v| v.id))
        .chain(project.definitions.iter().map(|d| d.id))
        .chain(project.libraries.iter().map(|l| l.id))
        .chain(project.assets.iter().map(|a| a.id))
        .chain(project.removed_pages.iter().copied())
        .chain(project.removed_assets.iter().copied())
        .collect()
}
pub(in crate::workspace) fn unresolved_overrides(
    project: &Project,
) -> Result<std::collections::BTreeSet<(Id, NodePath, NodePath)>, String> {
    let mut result = std::collections::BTreeSet::new();
    for (id, content) in project
        .pages
        .iter()
        .map(|p| (p.id, &p.content))
        .chain(
            project
                .pages
                .iter()
                .flat_map(|p| p.alternates.iter().map(|a| (a.id, &a.content))),
        )
        .chain(project.definitions.iter().map(|d| (d.id, &d.content)))
    {
        for resolution in project.resolve_all(&content.nodes)? {
            result.extend(
                resolution
                    .orphan_overrides
                    .into_iter()
                    .map(|(instance, path)| (id, instance, path)),
            );
        }
    }
    for library in &project.libraries {
        // Retained graphs still carry edits that must survive import. Inspect
        // their read-only source preview without making them active resources.
        let resolutions = if library.trashed {
            project.resolve_library_preview(library.id)?
        } else {
            project.resolve_library(library.id)?
        };
        for resolution in resolutions {
            result.extend(
                resolution
                    .orphan_overrides
                    .into_iter()
                    .map(|(instance, path)| (library.id, instance, path)),
            );
        }
    }
    Ok(result)
}
/// Conflict comparison excludes local library metadata and source placement.
/// Resource references have already been mapped to current destination identities.
fn equivalent_library_content(before: &Project, after: &Project, id: Id) -> bool {
    fn normalized(library: &ComponentLibrary) -> Option<(Content, Vec<ComponentSource>)> {
        let mut content = library.content.clone();
        // Compare effective ordering, including implicit structural order.
        content.paint_order = content.paint_paths().ok()?;
        // Paths use unique structural root positions, not definition IDs:
        // a definition ID can equal an ordinary library-scoped node ID.
        let root_positions: HashMap<_, _> = content
            .nodes
            .iter()
            .enumerate()
            .map(|(index, node)| (node.id, index as Id))
            .collect();
        let roots: HashMap<_, _> = library
            .sources
            .iter()
            .map(|s| (s.node, s.definition))
            .collect();
        for node in &mut content.nodes {
            if let Some(&definition) = roots.get(&node.id) {
                node.id = definition;
                node.origin = Point::default();
                node.locked = false;
            }
        }
        for path in &mut content.paint_order {
            path[0] = root_positions[&path[0]];
        }
        let mut sources = library.sources.clone();
        for source in &mut sources {
            source.node = source.definition;
        }
        Some((content, sources))
    }
    let a = before.libraries.iter().find(|l| l.id == id).unwrap();
    let b = after.libraries.iter().find(|l| l.id == id).unwrap();
    match (normalized(a), normalized(b)) {
        (Some(a_content), Some(b_content)) => {
            a_content == b_content
                && a.sources.iter().all(|s| {
                    before.definitions.iter().find(|d| d.id == s.definition)
                        == after.definitions.iter().find(|d| d.id == s.definition)
                })
        }
        _ => false,
    }
}
/// Exact replacement accounting must retain fields ignored for conflicts.
fn equivalent_library(before: &Project, after: &Project, id: Id) -> bool {
    let a = before.libraries.iter().find(|l| l.id == id).unwrap();
    let b = after.libraries.iter().find(|l| l.id == id).unwrap();
    // Library labels can differ after collision naming; content decides reuse.
    a.notes == b.notes
        && a.content == b.content
        && a.sources == b.sources
        && a.sources.iter().all(|s| {
            before.definitions.iter().find(|d| d.id == s.definition)
                == after.definitions.iter().find(|d| d.id == s.definition)
        })
}
fn unique_name(name: &str, names: &mut HashSet<String>) -> String {
    let stem = name
        .strip_suffix(')')
        .and_then(|s| s.rsplit_once('('))
        .filter(|(_, n)| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
        .map(|(stem, _)| stem.to_owned())
        .unwrap_or_else(|| format!("{name} "));
    let mut candidate = name.to_owned();
    let mut suffix = 0;
    while !names.insert(candidate.to_lowercase()) {
        suffix += 1;
        candidate = format!("{stem}({suffix})");
    }
    candidate
}
