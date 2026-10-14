// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use std::{fs, path::PathBuf};
use wireframe::{document::Value, model::Kind, workspace::Editor};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("evidence/list-control"));
    fs::create_dir_all(&out)?;
    for sketch in [true, false] {
        let mut e = Editor::default();
        e.document.settings.sketch = sketch;
        for column in 0..3 {
            e.add(Kind::List, 50. + column as f32 * 180., 50.);
            let id = *e.selection.iter().next().unwrap();
            e.begin("List specimen");
            for (key, value) in [
                (
                    "text",
                    Value::Text(
                        "Title\nAlpha\n*Bravo*\n-Disabled-\nDelta\nEcho\nFoxtrot\nGolf".into(),
                    ),
                ),
                ("header", Value::Bool(column > 0)),
                ("selected_index", Value::Number(1.)),
                ("vertical_scrollbar", Value::Bool(column == 2)),
                ("value", Value::Number(1.)),
            ] {
                e.set_control_property(id, key, value)?;
            }
            if column == 2 {
                e.set_control_property(id, "alternate_fill", Value::Color([32, 48, 64, 255]))?;
                e.set_control_property(id, "border", Value::Choice("none".into()))?;
                e.set_control_property(id, "row_height", Value::Number(30.))?;
            }
            e.commit();
        }
        let name = if sketch {
            "Lists-sketch"
        } else {
            "Lists-clean"
        };
        let scene = e.rendered_scene(e.page)?;
        fs::write(out.join(format!("{name}.png")), scene.png()?)?;
        fs::write(out.join(format!("{name}.svg")), scene.svg())?;
        fs::write(out.join(format!("{name}.wireframe")), e.document.encode()?)?;
    }
    Ok(())
}
