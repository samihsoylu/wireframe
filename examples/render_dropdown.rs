// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Editable dropdown menu and state gallery.
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
            "Sketch-dropdown"
        } else {
            "Clean-dropdown"
        };
        let mut p = Project {
            title: name.into(),
            ..Project::default()
        };
        p.settings.sketch = sketch;
        p.pages[0].name = "Dropdown states".into();
        for (row, state) in ["normal", "focused", "disabled"].iter().enumerate() {
            let id = p.allocate_id()?;
            let mut label = catalog::get("label")
                .unwrap()
                .create(id, Point::new(20., 30. + row as f32 * 145.));
            label
                .properties
                .insert("text".into(), Value::Text((*state).into()));
            p.pages[0].content.nodes.push(label);
            for (col, closed) in [false, true].iter().enumerate() {
                let id = p.allocate_id()?;
                let mut n = catalog::get("dropdown").unwrap().create(
                    id,
                    Point::new(110. + col as f32 * 175., 25. + row as f32 * 145.),
                );
                n.content = NodeContent::Control {
                    kind: "dropdown".into(),
                    size: Size {
                        width: Dimension::Fixed(150.),
                        height: Dimension::Auto,
                    },
                };
                let state = if *closed {
                    if *state == "normal" {
                        "closed".into()
                    } else {
                        format!("{state}_closed")
                    }
                } else {
                    (*state).into()
                };
                n.properties.insert("state".into(), Value::Choice(state));
                n.properties.insert(
                    "text".into(),
                    Value::Text("Choose\nAlpha\n\nx Beta\no Gamma >\nDelta, literal".into()),
                );
                n.properties
                    .insert("selected_index".into(), Value::Number(3.));
                p.pages[0].content.nodes.push(n);
            }
        }
        ProjectStore.save(&out.join(format!("{name}.wireframe")), &p)?;
        let s = Editor::new(p).rendered_scene(1)?;
        std::fs::write(out.join(format!("{name}.svg")), s.svg())?;
        std::fs::write(out.join(format!("{name}.png")), s.png()?)?;
    }
    Ok(())
}
