// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Read-only check of saved version-1 projects against the version-2 migration.
use wireframe::{document, model};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    for path in std::env::args().skip(1) {
        let bytes = std::fs::read(&path)?;
        let old: model::Document = serde_json::from_slice(&bytes)?;
        let new = document::decode(&bytes)?;
        assert_eq!(document::decode(&new.encode()?)?, new);
        assert_eq!(old.pages.len(), new.pages.len());
        let mut count = 0;
        for (old_page, new_page) in old.pages.iter().zip(&new.pages) {
            let leaves = new_page.content.leaves()?;
            assert_eq!(old_page.controls.len(), leaves.len());
            for (old, node) in old_page.controls.iter().zip(leaves) {
                assert_eq!(old.id, node.node.id);
                assert!((old.bounds.x - node.world_origin.x).abs() < 0.001);
                assert!((old.bounds.y - node.world_origin.y).abs() < 0.001);
                assert_eq!(
                    node.node.properties["text"],
                    document::Value::Text(old.text.clone())
                );
                count += 1;
            }
        }
        assert_eq!(std::fs::read(&path)?, bytes);
        println!(
            "PASS: {path}: {} pages, {count} controls; order, positions, text, round trip and unchanged source",
            new.pages.len()
        );
    }
    Ok(())
}
