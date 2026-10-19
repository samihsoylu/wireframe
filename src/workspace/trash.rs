// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Retained page transitions. History replays the operation against the current
//! graph because restoring a parent does not undo its children's adoption.
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RetainedResource {
    Page(Id),
    Library(Id),
    Asset(Id),
}
impl RetainedResource {
    pub fn id(self) -> Id {
        match self {
            Self::Page(id) | Self::Library(id) | Self::Asset(id) => id,
        }
    }
}

#[derive(Clone)]
pub(super) struct PageHistory {
    ids: Vec<Id>,
    libraries: Vec<Id>,
    assets: Vec<Id>,
    trash: bool,
    placeholder: Option<Page>,
    allocator_before_placeholder: Option<Id>,
}

impl PageHistory {
    pub(super) fn assets(assets: Vec<Id>, trash: bool) -> Self {
        Self {
            ids: vec![],
            libraries: vec![],
            assets,
            trash,
            placeholder: None,
            allocator_before_placeholder: None,
        }
    }
    pub(super) fn remove_deleted_targets(
        &mut self,
        removed: &std::collections::BTreeSet<Id>,
        libraries: &HashSet<Id>,
        assets: &std::collections::BTreeSet<Id>,
    ) -> bool {
        self.ids.retain(|id| !removed.contains(id));
        self.libraries.retain(|id| !libraries.contains(id));
        self.assets.retain(|id| !assets.contains(id));
        if self
            .placeholder
            .as_ref()
            .is_some_and(|p| removed.contains(&p.id))
        {
            self.placeholder = None;
            self.allocator_before_placeholder = None;
        }
        self.ids.is_empty() && self.libraries.is_empty() && self.assets.is_empty()
    }
}

impl Editor {
    pub fn retained_resources(&self) -> impl Iterator<Item = (RetainedResource, &str)> {
        self.document
            .pages
            .iter()
            .filter(|p| p.trashed)
            .map(|p| (RetainedResource::Page(p.id), p.name.as_str()))
            .chain(
                self.document
                    .assets
                    .iter()
                    .filter(|a| a.trashed)
                    .map(|a| (RetainedResource::Asset(a.id), a.name.as_str())),
            )
            .chain(
                self.document
                    .libraries
                    .iter()
                    .filter(|l| l.trashed)
                    .map(|l| (RetainedResource::Library(l.id), l.name.as_str())),
            )
    }

    /// Restore the complete selection in one history entry, including page
    /// adoption rules. Reject stale or mistyped resources before changing data.
    pub fn restore_resources(&mut self, resources: &[RetainedResource]) -> Result<(), String> {
        let selected: HashSet<_> = resources.iter().copied().collect();
        let available: HashSet<_> = self.retained_resources().map(|(r, _)| r).collect();
        if selected.is_empty()
            || selected.len() != resources.len()
            || !selected.is_subset(&available)
        {
            return Err("Select distinct resources still in Trash".into());
        }
        let mut history = PageHistory {
            ids: self
                .document
                .pages
                .iter()
                .filter(|p| selected.contains(&RetainedResource::Page(p.id)))
                .map(|p| p.id)
                .collect(),
            libraries: self
                .document
                .libraries
                .iter()
                .filter(|l| selected.contains(&RetainedResource::Library(l.id)))
                .map(|l| l.id)
                .collect(),
            assets: self
                .document
                .assets
                .iter()
                .filter(|a| selected.contains(&RetainedResource::Asset(a.id)))
                .map(|a| a.id)
                .collect(),
            trash: false,
            placeholder: None,
            allocator_before_placeholder: None,
        };
        let document = self.prepare_retained_transition(&mut history, false)?;
        self.commit();
        let before = self.snapshot();
        self.apply_prepared_retained_transition(document);
        self.record_with_page_history("Restore from Trash".into(), before, Some(history));
        Ok(())
    }
    /// The caller supplies explicit selection scope, including descendants only
    /// when the navigator has selected them. Input order and duplicates do not
    /// change the canonical bottom-to-top removal order.
    pub fn trash_pages(&mut self, ids: &[Id]) -> Result<(), String> {
        self.change_retained_pages(ids, true)
    }

    pub fn restore_pages(&mut self, ids: &[Id]) -> Result<(), String> {
        self.change_retained_pages(ids, false)
    }

    fn change_retained_pages(&mut self, ids: &[Id], trash: bool) -> Result<(), String> {
        if ids.is_empty() {
            return Ok(());
        }
        // Do not partially apply a stale selection.
        let requested: HashSet<_> = ids.iter().copied().collect();
        let ids: Vec<_> = self
            .document
            .pages
            .iter()
            .filter(|p| requested.contains(&p.id) && p.trashed != trash)
            .map(|p| p.id)
            .collect();
        if ids.len() != requested.len() {
            return Err("One or more pages are no longer available for this action".into());
        }
        let mut history = PageHistory {
            ids,
            libraries: vec![],
            assets: vec![],
            trash,
            placeholder: None,
            allocator_before_placeholder: None,
        };
        let document = self.prepare_retained_transition(&mut history, false)?;
        self.commit();
        let before = self.snapshot();
        self.apply_prepared_retained_transition(document);
        self.record_with_page_history(
            if trash {
                "Move to Trash"
            } else {
                "Restore from Trash"
            }
            .into(),
            before,
            Some(history),
        );
        Ok(())
    }

    pub(super) fn replay_page_history(&mut self, index: usize, undo: bool) -> bool {
        let Some(mut history) = self.history[index].page_history.clone() else {
            let change = &self.history[index];
            let mut target = if undo { &change.before } else { &change.after }.clone();
            if let Err(error) = reviews::reconcile_history(
                &self.document,
                &change.before.document,
                &change.after.document,
                &mut target.document,
                undo,
            ) {
                self.error = Some(error);
                return false;
            }
            // An ordinary edit must not also reverse adoption left behind by a
            // later Trash/Undo. Keep parent relationships that this edit never
            // changed, provided the target snapshot still contains the parent.
            let existing: HashSet<_> = target.document.pages.iter().map(|p| p.id).collect();
            for page in &mut target.document.pages {
                let before = change
                    .before
                    .document
                    .pages
                    .iter()
                    .find(|p| p.id == page.id);
                let after = change.after.document.pages.iter().find(|p| p.id == page.id);
                let current = self.document.pages.iter().find(|p| p.id == page.id);
                if let (Some(before), Some(after), Some(current)) = (before, after, current)
                    && before.parent == after.parent
                    && current
                        .parent
                        .is_none_or(|parent| existing.contains(&parent))
                {
                    page.parent = current.parent;
                }
            }
            if let Err(error) = target.document.encode() {
                self.error = Some(error);
                return false;
            }
            self.restore(target);
            return true;
        };
        if let Err(error) = self.apply_retained_transition(&mut history, undo) {
            self.error = Some(error);
            return false;
        }
        self.history[index].page_history = Some(history);
        true
    }

    pub(super) fn apply_retained_transition(
        &mut self,
        history: &mut PageHistory,
        undo: bool,
    ) -> Result<(), String> {
        let document = self.prepare_retained_transition(history, undo)?;
        self.apply_prepared_retained_transition(document);
        Ok(())
    }

    // Prepare the full graph, including placeholder allocation and live-page
    // limits, before committing an unrelated draft or discarding its Redo branch.
    fn prepare_retained_transition(
        &self,
        history: &mut PageHistory,
        undo: bool,
    ) -> Result<Project, String> {
        let mut document = self.document.clone();
        let trash = history.trash != undo;
        let ids: HashSet<_> = history.ids.iter().copied().collect();
        if trash && !ids.is_empty() {
            retain_pages(&mut document, &ids);
            if document.live_pages().next().is_none() {
                history.allocator_before_placeholder = Some(document.next_id);
                let reusable = history.placeholder.as_ref().filter(|page| {
                    let mut candidate = document.clone();
                    candidate.next_id = candidate.next_id.max(page.id + 1);
                    candidate.pages.push((*page).clone());
                    candidate.validate().is_ok()
                });
                let page = match reusable {
                    Some(page) => page.clone(),
                    None => Page {
                        id: document.allocate_id()?,
                        trashed: false,
                        name: "Wireframe 1".into(),
                        notes: String::new(),
                        parent: None,
                        content: Content::default(),
                        alternates: vec![],
                    },
                };
                document.next_id = document.next_id.max(page.id + 1);
                history.placeholder = Some(page.clone());
                document.pages.push(page);
            }
        } else if !ids.is_empty() {
            restore_retained_pages(&mut document, &ids);
            if undo && history.trash {
                let removed =
                    remove_untouched_placeholder(&mut document, history.placeholder.as_ref());
                if removed
                    && let (Some(page), Some(previous)) =
                        (&history.placeholder, history.allocator_before_placeholder)
                    && document.next_id == page.id + 1
                {
                    let next = document.next_id;
                    document.next_id = previous;
                    if document.validate().is_err() {
                        document.next_id = next;
                    }
                }
            }
        }
        for library in &mut document.libraries {
            if history.libraries.contains(&library.id) {
                library.trashed = trash;
            }
        }
        for asset in &mut document.assets {
            if history.assets.contains(&asset.id) {
                asset.trashed = trash;
            }
        }
        // Validate before changing either editor data or view. Limits and graph
        // failures must not advance the history cursor or partially restore.
        document.encode()?;
        Ok(document)
    }

    fn apply_prepared_retained_transition(&mut self, document: Project) {
        if document.live_page(self.page).is_none() {
            let index = self
                .document
                .pages
                .iter()
                .position(|p| p.id == self.page)
                .unwrap_or(0);
            // Removal proceeds from the bottom of the list. Keep the next
            // surviving page after the removed current page; only fall back
            // backward at the end. Undo preserves a surviving current page.
            // Use the old position because Undo can remove a generated blank.
            self.page = document
                .pages
                .iter()
                .skip(index)
                .find(|p| !p.trashed)
                .or_else(|| document.pages.iter().rev().find(|p| !p.trashed))
                .unwrap_or_else(|| document.first_live_page())
                .id;
            self.definition = None;
            self.library = None;
            self.context.clear();
            self.selection.clear();
        }
        self.document = document;
        self.reconcile();
    }
}

pub(super) fn retain_pages(document: &mut Project, ids: &HashSet<Id>) {
    let order: Vec<_> = document.live_pages().map(|p| p.id).collect();
    let positions: HashMap<_, _> = order.iter().enumerate().map(|(i, &id)| (id, i)).collect();
    let mut parents: HashMap<_, _> = document
        .live_pages()
        .map(|p| {
            (
                p.id,
                document
                    .live_parent(p)
                    .filter(|parent| positions[parent] < positions[&p.id]),
            )
        })
        .collect();
    // Removing descendants first prevents the removal of a selected ancestor
    // from rewriting a selected descendant's retained parent.
    for (index, &id) in order.iter().enumerate().rev() {
        if !ids.contains(&id) {
            continue;
        }
        let depths = page_depths(&parents);
        let depth = depths[&id];
        let adopter = order[..index]
            .iter()
            .enumerate()
            .filter_map(|(i, id)| {
                depths
                    .get(id)
                    .filter(|&&d| d <= depth)
                    .map(|&d| (d, i, *id))
            })
            .max()
            .map(|(_, _, id)| id);
        for parent in parents.values_mut() {
            if *parent == Some(id) {
                *parent = adopter;
            }
        }
        parents.remove(&id);
    }
    for page in &mut document.pages {
        if let Some(parent) = parents.get(&page.id) {
            page.parent = *parent;
        }
        if ids.contains(&page.id) {
            page.trashed = true;
        }
    }
}

fn page_depths(parents: &HashMap<Id, Option<Id>>) -> HashMap<Id, usize> {
    let mut depths = HashMap::new();
    for &id in parents.keys() {
        let mut path = vec![];
        let mut next = Some(id);
        while let Some(id) = next {
            if depths.contains_key(&id) {
                break;
            }
            path.push(id);
            next = parents[&id];
        }
        let base = next.map_or(0, |id| depths[&id] + 1);
        for (offset, id) in path.into_iter().rev().enumerate() {
            depths.insert(id, base + offset);
        }
    }
    depths
}

fn restore_retained_pages(document: &mut Project, ids: &HashSet<Id>) {
    // Normalize the already-live tree before adding restored resources. A child
    // whose parent is hidden remains visually at the root; restoring that parent
    // later must not silently reattach the child.
    let live: HashSet<_> = document.live_pages().map(|p| p.id).collect();
    let positions: HashMap<_, _> = document
        .pages
        .iter()
        .enumerate()
        .map(|(i, p)| (p.id, i))
        .collect();
    for page in &mut document.pages {
        if !page.trashed && page.parent.is_some_and(|parent| !live.contains(&parent)) {
            page.parent = None;
        }
        if page.trashed && ids.contains(&page.id) {
            page.trashed = false;
            if page.parent.is_some_and(|parent| {
                (live.contains(&parent) || ids.contains(&parent))
                    && positions[&parent] >= positions[&page.id]
            }) {
                page.parent = None;
            }
        }
    }
}

fn remove_untouched_placeholder(document: &mut Project, placeholder: Option<&Page>) -> bool {
    let Some(placeholder) = placeholder else {
        return false;
    };
    let Some(index) = document.pages.iter().position(|p| p == placeholder) else {
        return false;
    };
    if document.live_pages().count() < 2 {
        return false;
    }
    let page = document.pages.remove(index);
    // A later reference to the blank page makes it meaningful even if its own
    // contents are unchanged. Preserve it instead of creating dangling links.
    if document.validate().is_err() {
        document.pages.insert(index, page);
        return false;
    }
    true
}
