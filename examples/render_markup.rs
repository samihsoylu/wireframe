// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Markup gallery and neutral layout diagnostics.
use std::{fs, path::PathBuf};
use wireframe::{
    document::Value,
    markup::{self, Family},
    model::Kind,
    workspace::Editor,
};
#[derive(serde::Deserialize)]
struct Case {
    family: Family,
    width: f32,
    height: f32,
    direction: String,
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.get(1).is_some_and(|s| s == "--geometry") {
        let cases: Vec<Case> = serde_json::from_slice(&fs::read(&args[2])?)?;
        let out: Vec<_> = cases
            .into_iter()
            .map(|c| markup::layout(c.family, c.width, c.height, &c.direction))
            .collect();
        println!("{}", serde_json::to_string(&out)?);
        return Ok(());
    }
    let dir = PathBuf::from(args.get(1).cloned().unwrap_or("markup-gallery".into()));
    fs::create_dir_all(&dir)?;
    let mut e = Editor::default();
    e.document.title = "Markup".into();
    e.document.pages[0].name = "Markup study".into();
    for (col, kind) in [
        Kind::HorizontalBrace,
        Kind::VerticalBrace,
        Kind::RedCross,
        Kind::ScratchOut,
    ]
    .into_iter()
    .enumerate()
    {
        for (row, size) in [
            None,
            Some((300., 60.)),
            Some((60., 160.)),
            Some((10., 10.)),
            Some((1., 1.)),
            Some((20., 20.)),
            Some((21., 21.)),
            Some((200., 120.)),
            Some((200., 120.)),
            Some((200., 120.)),
        ]
        .into_iter()
        .enumerate()
        {
            e.add(kind, 20. + 360. * col as f32, 20. + 200. * row as f32);
            if let Some((w, h)) = size {
                e.selected_mut(|c| {
                    c.bounds.w = w;
                    c.bounds.h = h;
                });
            }
            let id = *e.selection.iter().next().unwrap();
            let brace = col < 2;
            if brace && [1, 2, 5, 7, 9].contains(&row) {
                e.set_control_property(
                    id,
                    "direction",
                    Value::Choice(if col == 0 { "bottom" } else { "right" }.into()),
                )?;
            }
            if brace && row == 7 {
                e.set_control_property(
                    id,
                    "text",
                    Value::Text("*Bold* [link](https://example.com)\n{mockup-name}".into()),
                )?;
            }
            if brace && row == 8 {
                e.set_control_property(id, "text", Value::Text("One\nTwo".into()))?;
                e.set_control_property(id, "text_color", Value::Color([0, 51, 204, 255]))?;
                e.set_control_property(id, "font_size", Value::Number(20.))?;
            }
            if brace && row == 9 {
                e.set_control_property(id, "text", Value::Text(String::new()))?;
            }
            if col == 3 && row >= 7 {
                e.set_control_property(id, "fill", Value::Color([0, 51, 204, 255]))?;
                e.set_control_property(
                    id,
                    "background_opacity",
                    Value::Number((row - 7) as f64 * 0.5),
                )?;
            }
        }
    }
    e.selection.clear();
    e.document.settings.sketch = false;
    fs::write(dir.join("Markup.wireframe"), e.document.encode()?)?;
    for (name, sketch) in [("clean", false), ("sketch", true)] {
        e.document.settings.sketch = sketch;
        let scene = e.rendered_scene(e.page)?;
        fs::write(dir.join(format!("{name}.png")), scene.png()?)?;
        fs::write(dir.join(format!("{name}.svg")), scene.svg())?;
    }
    Ok(())
}
