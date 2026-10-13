// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Editable gallery of embedded scrollbar positions and states.
use wireframe::{catalog, document::*, storage::ProjectStore, workspace::Editor};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = std::path::PathBuf::from(std::env::args().nth(1).ok_or("Supply output directory")?);
    std::fs::create_dir_all(&out)?;
    for sketch in [true, false] {
        let name = if sketch {
            "Sketch-scrollbars"
        } else {
            "Clean-scrollbars"
        };
        let mut p = Project {
            title: name.into(),
            ..Project::default()
        };
        p.settings.sketch = sketch;
        for (row, state) in ["normal", "focused", "disabled"].iter().enumerate() {
            for (col, value) in [0., 0.5, 1.].iter().enumerate() {
                for (offset, kind) in [(0., "dropdown"), (155., "text_area")] {
                    let id = p.allocate_id()?;
                    let mut n = catalog::get(kind).unwrap().create(
                        id,
                        Point::new(20. + col as f32 * 335. + offset, 20. + row as f32 * 165.),
                    );
                    n.content = NodeContent::Control {
                        kind: kind.into(),
                        size: Size {
                            width: Dimension::Fixed(140.),
                            height: if kind == "dropdown" {
                                Dimension::Auto
                            } else {
                                Dimension::Fixed(140.)
                            },
                        },
                    };
                    n.properties.insert("text".into(),Value::Text(if kind=="dropdown" {"Choose\nAlpha\nBeta\nGamma\nDelta"} else {"A scrollable text area with several words that wrap onto multiple lines."}.into()));
                    n.properties
                        .insert("state".into(), Value::Choice((*state).into()));
                    n.properties
                        .insert("vertical_scrollbar".into(), Value::Bool(true));
                    n.properties.insert("value".into(), Value::Number(*value));
                    p.pages[0].content.nodes.push(n);
                }
            }
        }
        ProjectStore.save(&out.join(format!("{name}.wireframe")), &p)?;
        let s = Editor::new(p).rendered_scene(1)?;
        std::fs::write(out.join(format!("{name}.svg")), s.svg())?;
        std::fs::write(out.join(format!("{name}.png")), s.png()?)?;
    }
    Ok(())
}
