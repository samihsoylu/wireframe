// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use std::{fs, path::PathBuf};
use wireframe::{
    document::{IconSource, Value},
    model::Kind,
    workspace::Editor,
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("evidence/list-icons"));
    fs::create_dir_all(&out)?;
    let mut e = Editor::default();
    e.document.title = "List icons".into();
    e.document.settings.sketch = false;
    e.add(Kind::List, 50., 50.);
    let id = *e.selection.iter().next().unwrap();
    e.begin("Example");
    for (key, value) in [
        (
            "text",
            Value::Text("Title\nAlpha\n-Disabled-\nDelta\n*Bravo*\nEcho\nFoxtrot\nGolf".into()),
        ),
        ("header", Value::Bool(true)),
        ("font_size", Value::Number(24.)),
        ("row_height", Value::Number(30.)),
        ("border", Value::Choice("none".into())),
        ("vertical_scrollbar", Value::Bool(true)),
        ("selected_index", Value::Number(3.)),
    ] {
        e.set_control_property(id, key, value)?;
    }
    e.set_control_property(
        id,
        "row_icons",
        Value::Icons(vec![
            None,
            Some(IconSource::Catalog("cloud-solid".into())),
            Some(IconSource::Catalog("asterisk-solid".into())),
            None,
            Some(IconSource::Catalog("asterisk-solid".into())),
        ]),
    )?;
    e.commit();
    fs::write(out.join("List-icons.wireframe"), e.document.encode()?)?;
    let scene = e.rendered_scene(e.page)?;
    fs::write(out.join("list-icons.svg"), scene.svg())?;
    fs::write(out.join("list-icons.png"), scene.png()?)?;
    Ok(())
}
