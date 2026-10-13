// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Editable gallery of input borders, fill opacity and text contrast.
use wireframe::{catalog, document::*, storage::ProjectStore, workspace::Editor};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = std::path::PathBuf::from(std::env::args().nth(1).ok_or("Supply output directory")?);
    std::fs::create_dir_all(&out)?;
    for sketch in [true, false] {
        let name = if sketch {
            "Sketch-input-appearance"
        } else {
            "Clean-input-appearance"
        };
        let mut p = Project {
            title: name.into(),
            ..Project::default()
        };
        p.settings.sketch = sketch;
        for (row, state) in ["normal", "focused", "disabled"].iter().enumerate() {
            for (col, (kind, alpha, border)) in [
                ("text_input", 1., "rectangle"),
                ("text_input", 0.5, "line"),
                ("text_area", 0.75, "rectangle"),
            ]
            .iter()
            .enumerate()
            {
                let id = p.allocate_id()?;
                let mut n = catalog::get(kind).unwrap().create(
                    id,
                    Point::new(20. + col as f32 * 225., 20. + row as f32 * 165.),
                );
                n.content = NodeContent::Control {
                    kind: (*kind).into(),
                    size: Size {
                        width: Dimension::Fixed(200.),
                        height: if *kind == "text_area" {
                            Dimension::Fixed(140.)
                        } else {
                            Dimension::Auto
                        },
                    },
                };
                for (key, value) in [
                    (
                        "text",
                        Value::Text(format!("{state} [link](https://example.com)")),
                    ),
                    ("state", Value::Choice((*state).into())),
                    ("fill", Value::Color([0, 0, 0, 255])),
                    ("border_color", Value::Color([190, 30, 70, 255])),
                    ("background_opacity", Value::Number(*alpha)),
                ] {
                    n.properties.insert(key.into(), value);
                }
                if *kind == "text_input" {
                    n.properties
                        .insert("border".into(), Value::Choice((*border).into()));
                }
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
