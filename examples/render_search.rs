// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Reproducible, editable search states and icon overflow examples.
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
            "Sketch-search"
        } else {
            "Clean-search"
        };
        let mut p = Project {
            title: name.into(),
            ..Project::default()
        };
        p.settings.sketch = sketch;
        p.pages[0].name = "Search states".into();
        for (row, state) in ["normal", "focused", "disabled"].iter().enumerate() {
            let id = p.allocate_id()?;
            let mut label = catalog::get("label")
                .unwrap()
                .create(id, Point::new(20., 30. + row as f32 * 65.));
            label
                .properties
                .insert("text".into(), Value::Text((*state).into()));
            p.pages[0].content.nodes.push(label);
            for (column, shape) in ["rounded", "rectangle"].iter().enumerate() {
                let id = p.allocate_id()?;
                let mut node = catalog::get("search").unwrap().create(
                    id,
                    Point::new(110. + column as f32 * 155., 25. + row as f32 * 65.),
                );
                node.properties
                    .insert("state".into(), Value::Choice((*state).into()));
                node.properties
                    .insert("shape".into(), Value::Choice((*shape).into()));
                node.properties
                    .insert("microphone_icon".into(), Value::Bool(true));
                p.pages[0].content.nodes.push(node);
            }
        }
        for (column, text) in ["Long text hides the microphone", "No search icon"]
            .iter()
            .enumerate()
        {
            let id = p.allocate_id()?;
            let mut node = catalog::get("search")
                .unwrap()
                .create(id, Point::new(110. + column as f32 * 155., 220.));
            node.content = NodeContent::Control {
                kind: "search".into(),
                size: Size {
                    width: Dimension::Fixed(140.),
                    height: Dimension::Auto,
                },
            };
            node.properties
                .insert("text".into(), Value::Text((*text).into()));
            node.properties
                .insert("search_icon".into(), Value::Bool(column == 0));
            node.properties
                .insert("microphone_icon".into(), Value::Bool(true));
            p.pages[0].content.nodes.push(node);
        }
        ProjectStore.save(&out.join(format!("{name}.wireframe")), &p)?;
        let scene = Editor::new(p).rendered_scene(1)?;
        std::fs::write(out.join(format!("{name}.svg")), scene.svg())?;
        std::fs::write(out.join(format!("{name}.png")), scene.png()?)?;
    }
    Ok(())
}
