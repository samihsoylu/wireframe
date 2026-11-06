// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Exact compact JSON accounting, with payload lengths tied to history snapshots.
use crate::document::{Id, Project};
use std::collections::HashMap;

/// Only lengths are retained. The owning Change keeps the corresponding Project
/// immutable; permanent removal clears this metadata before changing snapshots.
#[derive(Clone, Default)]
pub(super) struct AssetLengths(HashMap<Id, usize>);

pub(super) struct Size {
    pub(super) bytes: usize,
    pub(super) assets: AssetLengths,
    #[cfg(test)]
    payload_encodes: usize,
}

/// Preserve normal recording's historical zero-on-serde-error policy. An error
/// must not leave metadata that a later record could reuse.
pub(super) fn normal(result: Result<Size, serde_json::Error>) -> (usize, Option<AssetLengths>) {
    match result {
        Ok(size) => (size.bytes, Some(size.assets)),
        Err(_) => (0, None),
    }
}

pub(super) fn measure(
    project: &Project,
    reuse: Option<(&Project, &AssetLengths)>,
) -> Result<Size, serde_json::Error> {
    // Preserve the original path for projects without embedded assets.
    if project.assets.is_empty() {
        return Ok(Size {
            bytes: serde_json::to_vec(project)?.len(),
            assets: AssetLengths::default(),
            #[cfg(test)]
            payload_encodes: 0,
        });
    }
    let previous: HashMap<_, _> = reuse
        .into_iter()
        .flat_map(|(previous, lengths)| {
            previous.assets.iter().filter_map(move |asset| {
                lengths
                    .0
                    .get(&asset.id)
                    .map(|&length| (asset.id, (asset.bytes.as_slice(), length)))
            })
        })
        .collect();
    let mut projection = project.clone();
    for asset in &mut projection.assets {
        asset.bytes.clear();
    }
    let mut bytes = serde_json::to_vec(&projection)?.len();
    let empty = serde_json::to_vec(&[] as &[u8])?.len();
    let mut assets = AssetLengths::default();
    #[cfg(test)]
    let mut payload_encodes = 0;
    for asset in &project.assets {
        let length = match previous.get(&asset.id) {
            Some(&(prior_bytes, length)) if prior_bytes == asset.bytes.as_slice() => length,
            _ => {
                #[cfg(test)]
                {
                    payload_encodes += 1;
                }
                serde_json::to_vec(&asset.bytes)?.len()
            }
        };
        // Asset.bytes is an always-present Vec<u8> in derived serialization.
        // All surrounding fields and escaping still come from actual serde.
        bytes += length - empty;
        assets.0.insert(asset.id, length);
    }
    Ok(Size {
        bytes,
        assets,
        #[cfg(test)]
        payload_encodes,
    })
}

#[cfg(test)]
mod tests {
    use super::{AssetLengths, Size, measure, normal};
    use crate::{
        document::{Asset, Node, Point, Project, Size as ControlSize, Value},
        workspace::Editor,
    };

    fn project(payloads: &[&[u8]]) -> Project {
        let mut p = Project {
            font: None,
            ..Project::default()
        };
        for bytes in payloads {
            let id = p.allocate_id().unwrap();
            p.assets.push(Asset {
                id,
                trashed: false,
                attribution: None,
                name: format!("image-{id}"),
                media_type: "image/png".into(),
                bytes: bytes.to_vec(),
            });
        }
        p
    }

    fn oracle(p: &Project) -> usize {
        serde_json::to_vec(p).unwrap().len()
    }

    fn exact(p: &Project, reuse: Option<(&Project, &AssetLengths)>) -> Size {
        let original = p.clone();
        let size = measure(p, reuse).unwrap();
        assert_eq!(size.bytes, oracle(p));
        assert_eq!(p, &original);
        assert_eq!(size.assets.0.len(), p.assets.len());
        for asset in &p.assets {
            assert_eq!(
                size.assets.0[&asset.id],
                serde_json::to_vec(&asset.bytes).unwrap().len()
            );
        }
        size
    }

    fn exact_history(e: &Editor) {
        for change in &e.history {
            assert_eq!(
                change.bytes,
                oracle(&change.before.document) + oracle(&change.after.document)
            );
            for (snapshot, memo) in [
                (&change.before, &change.before_asset_lengths),
                (&change.after, &change.after_asset_lengths),
            ] {
                let memo = memo.as_ref().unwrap();
                exact(&snapshot.document, Some((&snapshot.document, memo)));
                for asset in &snapshot.document.assets {
                    assert_eq!(
                        memo.0[&asset.id],
                        serde_json::to_vec(&asset.bytes).unwrap().len()
                    );
                }
                assert_eq!(memo.0.len(), snapshot.document.assets.len());
            }
        }
    }

    #[test]
    fn cold_and_reused_sizes_match_serde_for_empty_payloads_all_bytes_and_shared_references() {
        let values: Vec<u8> = (0..=255).collect();
        for p in [
            project(&[]),
            project(&[&[]]),
            project(&[&values, &[0, 9, 10, 99, 100, 255]]),
        ] {
            let first = exact(&p, None);
            assert_eq!(first.payload_encodes, p.assets.len());
            let hit = exact(&p, Some((&p, &first.assets)));
            assert_eq!(hit.payload_encodes, 0);
        }
        let mut p = project(&[&values]);
        for x in 0..4 {
            let id = p.allocate_id().unwrap();
            let mut n = Node::control(
                id,
                "image",
                Point::new(x as f32 * 10., 0.),
                ControlSize::default(),
            );
            n.properties
                .insert("image".into(), Value::Asset(p.assets[0].id));
            p.pages[0].content.nodes.push(n);
        }
        let sized = exact(&p, None);
        assert_eq!(sized.payload_encodes, 1); // One stored asset, four references.
    }

    #[test]
    fn equal_length_mutation_add_delete_and_reorder_cannot_reuse_stale_payload_lengths() {
        let old = project(&[&[0, 9], &[100, 255]]);
        let memo = exact(&old, None).assets;
        let mut p = old.clone();
        p.assets[0].bytes[0] = 255; // Same ID/length, different JSON byte length.
        let changed = exact(&p, Some((&old, &memo)));
        assert_eq!(changed.payload_encodes, 1);
        assert_eq!(changed.bytes, oracle(&old) + 2);
        p.assets.reverse();
        assert_eq!(exact(&p, Some((&old, &memo))).payload_encodes, 1);
        p.assets.retain(|a| a.id != old.assets[0].id);
        assert_eq!(exact(&p, Some((&old, &memo))).payload_encodes, 0);
        let id = p.allocate_id().unwrap();
        let mut duplicate = p.assets[0].clone();
        duplicate.id = id;
        p.assets.push(duplicate);
        assert_eq!(exact(&p, Some((&old, &memo))).payload_encodes, 1);
        // Metadata absence is a cold miss, never a guessed length.
        assert_eq!(
            exact(&p, Some((&old, &AssetLengths::default()))).payload_encodes,
            2
        );
    }

    #[test]
    fn unicode_escaping_metadata_and_trash_flags_are_counted_fresh() {
        let old = project(&[&[0, 255]]);
        let memo = exact(&old, None).assets;
        let mut p = old.clone();
        p.title = "é 🦀 \"quote\" \\ slash\nline\t".into();
        p.pages[0].notes = "汉字 and a null:\0".into();
        p.assets[0].name = "日本語 \"photo\"".into();
        p.assets[0].media_type = "custom\\type".into();
        p.assets[0].trashed = true;
        let size = exact(&p, Some((&old, &memo)));
        assert_eq!(size.payload_encodes, 0);
        assert_ne!(size.bytes, oracle(&old));
    }

    #[test]
    fn recorded_sizes_stay_exact_through_undo_redo_noop_and_public_payload_changes() {
        let mut e = Editor::new(project(&[&[0, 9], &[100, 255]]));
        e.edit("First", |e| e.document.title = "First".into());
        exact_history(&e);
        let first = e.document.clone();
        e.edit("Replace", |e| e.document.assets[0].bytes[0] = 255);
        let second = e.document.clone();
        exact_history(&e);
        e.undo();
        assert_eq!(e.document, first);
        e.redo();
        assert_eq!(e.document, second);
        e.undo();
        let count = e.history.len();
        let redo = e.redo_name().map(str::to_owned);
        e.edit("Noop", |_| {});
        assert_eq!(e.history.len(), count);
        assert_eq!(e.redo_name(), redo.as_deref());
        e.document.assets[0].bytes[1] = 255; // Mutation outside editor methods.
        e.edit("Branch", |e| e.document.title = "Branch".into());
        assert_eq!(e.redo_name(), None);
        exact_history(&e);
        while e.cursor > 0 {
            e.undo();
        }
        e.edit("At zero", |e| e.document.title = "Zero".into());
        assert_eq!(e.history.len(), 1);
        exact_history(&e);
    }

    // Exercise the production pruning function with synthetic accounting totals,
    // independently of serializer equivalence above. No giant payload fixtures.
    fn budget_fixture(costs: &[usize]) -> Editor {
        let mut e = Editor::new(project(&[]));
        e.edit("Seed", |e| e.document.title = "Seed".into());
        let seed = e.history[0].clone();
        e.history = costs
            .iter()
            .enumerate()
            .map(|(i, &bytes)| {
                let mut c = seed.clone();
                c.bytes = bytes;
                c.name = i.to_string();
                c
            })
            .collect();
        e.cursor = e.history.len();
        e
    }

    #[test]
    fn history_entry_limit_keeps_exactly_one_hundred_latest_entries() {
        let mut at = budget_fixture(&[1; 100]);
        at.trim_recorded_history();
        assert_eq!((at.history.len(), at.cursor), (100, 100));
        let mut over = budget_fixture(&[1; 101]);
        over.trim_recorded_history();
        assert_eq!((over.history.len(), over.cursor), (100, 100));
        assert_eq!(over.history[0].name, "1");
        assert_eq!(over.history.last().unwrap().name, "100");
    }

    #[test]
    fn history_byte_boundary_is_exact_and_newest_oversized_entry_survives() {
        const LIMIT: usize = 64 * 1024 * 1024;
        let mut at = budget_fixture(&[LIMIT - 1, 1]);
        at.trim_recorded_history();
        assert_eq!((at.history.len(), at.cursor), (2, 2));
        let mut over = budget_fixture(&[LIMIT, 1]);
        over.trim_recorded_history();
        assert_eq!((over.history.len(), over.cursor), (1, 1));
        assert_eq!(over.history[0].name, "1");
        for costs in [vec![1, LIMIT + 1], vec![LIMIT + 1]] {
            let mut e = budget_fixture(&costs);
            e.trim_recorded_history();
            assert_eq!((e.history.len(), e.cursor), (1, 1));
            assert_eq!(e.history[0].bytes, LIMIT + 1);
        }
    }

    #[test]
    fn permanent_removal_rebuilds_metadata_and_cannot_restore_deleted_payloads() {
        let mut e = Editor::new(project(&[&[0, 255], &[10, 100]]));
        let removed = e.document.assets[0].id;
        e.edit("Title", |e| e.document.title = "Title".into());
        e.trash_assets(&[removed]).unwrap();
        e.begin("Pending");
        e.document.pages[0].notes = "Pending note".into();
        e.permanently_remove_resources(&[crate::workspace::RetainedResource::Asset(removed)])
            .unwrap();
        exact_history(&e);
        for change in &e.history {
            assert!(
                !change
                    .before_asset_lengths
                    .as_ref()
                    .unwrap()
                    .0
                    .contains_key(&removed)
            );
            assert!(
                !change
                    .after_asset_lengths
                    .as_ref()
                    .unwrap()
                    .0
                    .contains_key(&removed)
            );
        }
        for redo in [false, true] {
            for _ in 0..6 {
                if redo {
                    e.redo();
                } else {
                    e.undo();
                }
                assert!(e.document.assets.iter().all(|a| a.id != removed));
                assert!(e.document.removed_assets.contains(&removed));
                exact_history(&e);
            }
        }
    }

    #[test]
    fn permanent_projection_error_preserves_pending_and_original_history() {
        let mut e = Editor::new(project(&[&[0, 255]]));
        let id = e.document.assets[0].id;
        e.edit("Title", |e| e.document.title = "Title".into());
        e.trash_assets(&[id]).unwrap();
        // Force an existing historical encode preflight to fail, without a
        // giant fixture or changing the current valid document.
        e.history[0].before.document.title = "x".repeat(10_001);
        e.begin("Pending");
        e.document.pages[0].notes = "Keep pending".into();
        let document = e.document.clone();
        let history = e.history.clone();
        let cursor = e.cursor;
        let pending = e.pending.as_ref().unwrap().1.document.clone();
        assert!(
            e.permanently_remove_resources(&[crate::workspace::RetainedResource::Asset(id)])
                .is_err()
        );
        assert_eq!(e.document, document);
        assert_eq!(e.cursor, cursor);
        assert_eq!(e.pending.as_ref().unwrap().1.document, pending);
        assert_eq!(e.history.len(), history.len());
        for (a, b) in e.history.iter().zip(&history) {
            assert_eq!(a.before.document, b.before.document);
            assert_eq!(a.after.document, b.after.document);
            assert_eq!(a.bytes, b.bytes);
            assert_eq!(
                a.before_asset_lengths.as_ref().unwrap().0,
                b.before_asset_lengths.as_ref().unwrap().0
            );
            assert_eq!(
                a.after_asset_lengths.as_ref().unwrap().0,
                b.after_asset_lengths.as_ref().unwrap().0
            );
        }
    }

    #[test]
    fn normal_serde_error_preserves_zero_accounting_without_reusable_metadata() {
        let error = serde_json::from_slice::<serde_json::Value>(b"[").unwrap_err();
        let (bytes, memo) = normal(Err(error));
        assert_eq!(bytes, 0);
        assert!(memo.is_none());
        let p = project(&[&[0, 255]]);
        let (bytes, memo) = normal(measure(&p, None));
        assert_eq!(bytes, oracle(&p));
        assert!(memo.is_some());
    }
}
