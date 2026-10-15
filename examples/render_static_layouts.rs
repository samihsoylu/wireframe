// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Static layout gallery and geometry diagnostics.
use std::{fs, path::PathBuf};
use wireframe::{
    document::{Dimension, Link, NodeContent, Value},
    model::Kind,
    static_layouts as layouts,
    workspace::Editor,
};
#[derive(serde::Deserialize)]
struct Case {
    family: String,
    width: f32,
    height: f32,
    #[serde(default)]
    sketch: bool,
    #[serde(default)]
    stroke: String,
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.get(1).is_some_and(|s| s == "--geometry") {
        let cases: Vec<Case> = serde_json::from_slice(&fs::read(&args[2])?)?;
        let out:Vec<_>=cases.into_iter().map(|c|{let dimension=|v|if v<0.{Dimension::Auto}else{Dimension::Fixed(v)};serde_json::json!({"size":layouts::size(&c.family,dimension(c.width),dimension(c.height)),"toolbar":layouts::toolbar(c.width),"rule":layouts::rule(c.height,c.stroke=="dotted",c.sketch)})}).collect();
        println!("{}", serde_json::to_string(&out)?);
        return Ok(());
    }
    let dir = PathBuf::from(
        args.get(1)
            .cloned()
            .unwrap_or("static-layouts-gallery".into()),
    );
    fs::create_dir_all(&dir)?;
    let mut e = Editor::default();
    e.document.title = "Static layouts".into();
    e.document.pages[0].name = "Layout controls".into();
    for (col, kind) in [
        Kind::FormattingToolbar,
        Kind::StreetMap,
        Kind::ModalOverlay,
        Kind::VerticalRule,
    ]
    .into_iter()
    .enumerate()
    {
        for row in 0..7 {
            e.add(kind, 20. + 340. * col as f32, 20. + 270. * row as f32);
            let id = *e.selection.iter().next().unwrap();
            let size = match kind {
                Kind::FormattingToolbar => Some((
                    [261., 23., 24., 131., 132., 249., 250.][row],
                    if row == 6 { 100. } else { 29. },
                )),
                Kind::VerticalRule => Some((
                    if row == 6 { 100. } else { 10. },
                    [100., 100., 100., 100., 10., 1., 240.][row],
                )),
                _ => {
                    if row == 0 {
                        None
                    } else {
                        Some(
                            [
                                (100., 100.),
                                (100., 200.),
                                (300., 80.),
                                (10., 10.),
                                (1., 1.),
                                if kind == Kind::StreetMap {
                                    (252., 222.)
                                } else {
                                    (100., 100.)
                                },
                            ][row - 1],
                        )
                    }
                }
            };
            if kind == Kind::VerticalRule {
                if [1, 2, 3, 6].contains(&row) {
                    e.set_control_property(
                        id,
                        "stroke",
                        Value::Choice(
                            if row == 1 || row == 3 {
                                "dashed"
                            } else {
                                "dotted"
                            }
                            .into(),
                        ),
                    )?;
                }
                if row == 3 || row == 6 {
                    e.set_control_property(
                        id,
                        "border_color",
                        Value::Color(if row == 3 {
                            [255, 0, 0, 255]
                        } else {
                            [0, 0, 255, 255]
                        }),
                    )?;
                }
                if row == 3 {
                    e.set_control_property(id, "background_opacity", Value::Number(0.4))?;
                }
            }
            if kind == Kind::ModalOverlay && row == 6 {
                e.set_control_property(
                    id,
                    "link",
                    Value::Link(Link::Url("https://example.com".into())),
                )?;
            }
            if let Some((w, h)) = size
                && let NodeContent::Control { size, .. } = &mut e.document.pages[0]
                    .content
                    .nodes
                    .last_mut()
                    .unwrap()
                    .content
            {
                size.width = Dimension::Fixed(w);
                size.height = Dimension::Fixed(h);
            }
        }
    }
    e.selection.clear();
    e.document.settings.sketch = false;
    fs::write(dir.join("Static-layouts.wireframe"), e.document.encode()?)?;
    for (style, sketch) in [("clean", false), ("sketch", true)] {
        e.document.settings.sketch = sketch;
        let s = e.rendered_scene(e.page)?;
        fs::write(dir.join(format!("{style}.png")), s.png()?)?;
        fs::write(dir.join(format!("{style}.svg")), s.svg())?;
    }
    Ok(())
}
