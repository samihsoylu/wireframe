// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Reproducible checkbox/radio state gallery with editable source.
use wireframe::{catalog, document::*, storage::ProjectStore, workspace::Editor};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = std::path::PathBuf::from(
        std::env::args()
            .nth(1)
            .ok_or("Supply an output directory")?,
    );
    std::fs::create_dir_all(&out)?;
    let mut p = Project {
        title: "Choice states".into(),
        next_id: 14,
        ..Project::default()
    };
    p.pages[0].name = "Choice states".into();
    let states = [
        ("normal", "Normal"),
        ("selected", "Selected"),
        ("disabled", "Disabled"),
        ("disabled_selected", "Selected and disabled"),
        ("indeterminate", "Indeterminate"),
        ("disabled_indeterminate", "Indeterminate and disabled"),
    ];
    for (column, kind) in ["checkbox", "radio"].iter().enumerate() {
        for (row, (state, label)) in states.iter().enumerate() {
            let id = 2 + (column * 6 + row) as u64;
            let mut n = catalog::get(kind).unwrap().create(
                id,
                Point::new(40. + column as f32 * 240., 40. + row as f32 * 45.),
            );
            n.properties
                .insert("state".into(), Value::Choice((*state).into()));
            n.properties
                .insert("text".into(), Value::Text((*label).into()));
            p.pages[0].content.nodes.push(n);
        }
    }
    p.validate()?;
    ProjectStore.save(&out.join("Choice-states.wireframe"), &p)?;
    let scene = Editor::new(p).rendered_scene(1)?;
    std::fs::write(out.join("Choice-states.svg"), scene.svg())?;
    std::fs::write(out.join("Choice-states.png"), scene.png()?)?;
    let mut defaults = Editor::default();
    for kind in [
        wireframe::model::Kind::Checkbox,
        wireframe::model::Kind::Radio,
    ] {
        defaults.add(kind, 0., 0.);
    }
    for control in defaults.current().controls {
        println!(
            "{}: {} x {}",
            control.kind.name(),
            control.bounds.w,
            control.bounds.h
        );
    }
    Ok(())
}
