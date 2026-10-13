// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Editable basic form states in each drawing style.
use wireframe::{catalog, document::*, storage::ProjectStore, workspace::Editor};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = std::path::PathBuf::from(
        std::env::args()
            .nth(1)
            .ok_or("Supply an output directory")?,
    );
    std::fs::create_dir_all(&out)?;
    for sketch in [true, false] {
        let name = if sketch {
            "Sketch-form-states"
        } else {
            "Clean-form-states"
        };
        let mut p = Project {
            title: name.into(),
            ..Project::default()
        };
        p.settings.sketch = sketch;
        p.pages[0].name = "Form states".into();
        for (row, state) in ["normal", "selected", "focused", "disabled"]
            .iter()
            .enumerate()
        {
            let id = p.allocate_id()?;
            let mut label = catalog::get("label")
                .unwrap()
                .create(id, Point::new(30., 40. + row as f32 * 90.));
            label
                .properties
                .insert("text".into(), Value::Text((*state).into()));
            p.pages[0].content.nodes.push(label);
            for (column, kind) in ["button", "text_input", "text_area"].iter().enumerate() {
                if *state == "selected" && *kind != "button" {
                    continue;
                }
                let id = p.allocate_id()?;
                let mut node = catalog::get(kind).unwrap().create(
                    id,
                    Point::new(130. + column as f32 * 165., 30. + row as f32 * 90.),
                );
                node.properties
                    .insert("state".into(), Value::Choice((*state).into()));
                node.properties.insert(
                    "text".into(),
                    Value::Text(
                        if *kind == "button" {
                            "Button"
                        } else if *kind == "text_input" {
                            "Account"
                        } else {
                            "Write a note\nSecond line"
                        }
                        .into(),
                    ),
                );
                node.content = NodeContent::Control {
                    kind: (*kind).into(),
                    size: Size::fixed(140., if *kind == "text_area" { 65. } else { 32. }),
                };
                p.pages[0].content.nodes.push(node);
            }
        }
        ProjectStore.save(&out.join(format!("{name}.wireframe")), &p)?;
        let scene = Editor::new(p).rendered_scene(1)?;
        std::fs::write(out.join(format!("{name}.svg")), scene.svg())?;
        std::fs::write(out.join(format!("{name}.png")), scene.png()?)?;
    }
    let mut e = Editor::default();
    for kind in [
        wireframe::model::Kind::Button,
        wireframe::model::Kind::TextInput,
        wireframe::model::Kind::TextArea,
    ] {
        e.add(kind, 0., 0.);
    }
    for c in e.current().controls {
        println!("{}: {} x {}", c.kind.name(), c.bounds.w, c.bounds.h);
    }
    Ok(())
}
