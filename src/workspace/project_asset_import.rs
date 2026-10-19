// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Image decisions share the whole-project import's isolated resource mapping.
use super::*;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssetImportChoice {
    UseNew,
    UseExisting,
    KeepBoth,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssetImportAction {
    Add,
    Reuse,
    Conflict,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssetImportEntry {
    pub incoming: Id,
    pub name: String,
    pub existing: Option<Id>,
    pub action: AssetImportAction,
    pub incoming_trashed: bool,
    pub existing_trashed: Option<bool>,
}
pub(super) struct AssetReview<'a> {
    pub entries: &'a [AssetImportEntry],
    pub choices: &'a BTreeMap<Id, AssetImportChoice>,
}
pub(super) fn entries(base: &Project, source: &Project, origin: &str) -> Vec<AssetImportEntry> {
    let mut claimed = HashSet::new();
    let mut entries: Vec<_> = source
        .assets
        .iter()
        .map(|asset| {
            let existing = base
                .assets
                .iter()
                .find(|target| {
                    !claimed.contains(&target.id)
                        && base.asset_imports.get(&target.id).is_some_and(|records| {
                            records
                                .iter()
                                .any(|r| r.project == origin && r.asset == asset.id)
                        })
                })
                .map(|a| a.id);
            if let Some(id) = existing {
                claimed.insert(id);
            }
            AssetImportEntry {
                incoming: asset.id,
                name: asset.name.clone(),
                existing,
                action: AssetImportAction::Add,
                incoming_trashed: asset.trashed,
                existing_trashed: None,
            }
        })
        .collect();
    for entry in entries.iter_mut().filter(|e| e.existing.is_none()) {
        if let Some(asset) = base
            .assets
            .iter()
            .find(|a| !claimed.contains(&a.id) && a.name == entry.name)
        {
            entry.existing = Some(asset.id);
            claimed.insert(asset.id);
        }
    }
    for (entry, incoming) in entries.iter_mut().zip(&source.assets) {
        if let Some(existing) = entry.existing {
            let existing = base.assets.iter().find(|a| a.id == existing).unwrap();
            entry.existing_trashed = Some(existing.trashed);
            entry.action = if identical(existing, incoming) {
                AssetImportAction::Reuse
            } else {
                AssetImportAction::Conflict
            };
        }
    }
    entries
}
fn identical(a: &Asset, b: &Asset) -> bool {
    a.trashed == b.trashed && a.media_type == b.media_type && a.bytes == b.bytes
}
pub(super) fn remember(project: &mut Project, target: Id, origin: &str, incoming: Id) {
    let records = project.asset_imports.entry(target).or_default();
    if !records
        .iter()
        .any(|r| r.project == origin && r.asset == incoming)
    {
        records.push(AssetImport {
            project: origin.into(),
            asset: incoming,
        });
    }
}
pub(super) fn stage(
    after: &mut Project,
    source: &Project,
    origin: &str,
    review: Option<AssetReview<'_>>,
    counts: &mut ProjectCopyCounts,
) -> Result<HashMap<Id, Id>, String> {
    if let Some(review) = &review {
        for id in review.choices.keys() {
            if !review
                .entries
                .iter()
                .any(|e| e.incoming == *id && e.action == AssetImportAction::Conflict)
            {
                return Err("Choice does not identify an image conflict".into());
            }
        }
        if review.entries.iter().any(|e| {
            e.action == AssetImportAction::Conflict && !review.choices.contains_key(&e.incoming)
        }) {
            return Err("Choose how to resolve every image conflict".into());
        }
    }
    let mut names: HashSet<_> = after.assets.iter().map(|a| a.name.to_lowercase()).collect();
    let mut map = HashMap::new();
    for id in &source.removed_assets {
        let target = after.allocate_id()?;
        after.removed_assets.insert(target);
        map.insert(*id, target);
    }
    for asset in &source.assets {
        let decision = review.as_ref().map(|review| {
            let entry = review
                .entries
                .iter()
                .find(|e| e.incoming == asset.id)
                .unwrap();
            let choice = match entry.action {
                AssetImportAction::Add => AssetImportChoice::KeepBoth,
                AssetImportAction::Reuse => AssetImportChoice::UseExisting,
                AssetImportAction::Conflict => review.choices[&asset.id],
            };
            (entry.existing, choice)
        });
        let target = if let Some((Some(id), choice)) =
            decision.filter(|(_, c)| *c != AssetImportChoice::KeepBoth)
        {
            if choice == AssetImportChoice::UseNew {
                let current = after.assets.iter_mut().find(|a| a.id == id).unwrap();
                current.trashed = asset.trashed;
                current.bytes = asset.bytes.clone();
                current.media_type = asset.media_type.clone();
                counts.assets_replaced += 1;
            } else {
                counts.assets_reused += 1;
            }
            id
        } else if review.is_none()
            && let Some(existing) = after.assets.iter().find(|a| identical(a, asset))
        {
            // Retain the original explicit-copy API's byte deduplication policy.
            counts.assets_reused += 1;
            existing.id
        } else {
            let mut copy = asset.clone();
            copy.id = after.allocate_id()?;
            if review.is_some() {
                copy.name = project_copy::unique_name(&copy.name, &mut names);
            }
            let id = copy.id;
            after.assets.push(copy);
            counts.assets_added += 1;
            id
        };
        remember(after, target, origin, asset.id);
        map.insert(asset.id, target);
    }
    Ok(map)
}
