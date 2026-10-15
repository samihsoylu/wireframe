// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Editable text gallery and geometry diagnostics.
use std::{fs, path::PathBuf};
use wireframe::{
    document::{Dimension, NodeContent, Value},
    layout::TextMetrics,
    model::Kind,
    text_controls as text,
    workspace::Editor,
};
#[derive(serde::Deserialize)]
struct Case {
    family: String,
    width: f32,
    height: f32,
    #[serde(default)]
    font: f32,
    #[serde(default)]
    id: u64,
    #[serde(default)]
    text: String,
    #[serde(default)]
    metrics: Vec<[f32; 2]>,
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.get(1).is_some_and(|v| v == "--geometry") {
        let cases: Vec<Case> = serde_json::from_slice(&fs::read(&args[2])?)?;
        let out:Vec<_>=cases.into_iter().map(|c|match c.family.as_str() {
            "block"=>serde_json::json!(text::block_rows(c.width,c.height,c.font)),
            "line"=>serde_json::json!(text::line_width(c.width,c.height)),
            "title"|"subtitle"=>serde_json::json!(text::title_box(c.width,c.height,c.family=="subtitle")),
            "cloud"=>{let mut i=0;serde_json::json!(text::cloud_layout_with(&c.text,c.id,c.width,c.height,|_,_|{let m=c.metrics[i];i+=1;TextMetrics{width:m[0],height:m[1],rows:1}}).iter().map(|l|serde_json::json!({"font":l.font,"bounds":l.bounds,"visible":l.visible})).collect::<Vec<_>>())},
            _=>panic!("Unknown diagnostic family")
        }).collect();
        println!("{}", serde_json::to_string(&out)?);
        return Ok(());
    }
    let dir = PathBuf::from(args.get(1).cloned().unwrap_or("text-gallery".into()));
    fs::create_dir_all(&dir)?;
    let mut e = Editor::default();
    e.document.title = "Text layouts".into();
    e.document.pages[0].name = "Text study".into();
    for (col, kind) in [
        Kind::TextBlock,
        Kind::TextLine,
        Kind::Subtitle,
        Kind::TagCloud,
        Kind::Heading,
    ]
    .into_iter()
    .enumerate()
    {
        for row in 0..8 {
            e.add(kind, 20. + 390. * col as f32, 20. + 190. * row as f32);
            let id = *e.selection.iter().next().unwrap();
            let size = if col < 2 {
                [
                    (None, None),
                    (Some(60.), Some(150.)),
                    (Some(1.), Some(80.)),
                    (Some(100.), Some(16.)),
                    (Some(100.), Some(17.)),
                    (Some(300.), Some(60.)),
                    (Some(10.), Some(10.)),
                    (Some(275.), Some(150.)),
                ][row]
            } else if col == 3 {
                [
                    (None, None),
                    (Some(60.), Some(150.)),
                    (Some(1.), Some(80.)),
                    (Some(100.), Some(25.)),
                    (Some(100.), Some(26.)),
                    (Some(350.), Some(110.)),
                    (Some(350.), Some(110.)),
                    (Some(350.), Some(110.)),
                ][row]
            } else {
                [
                    (None, None),
                    (Some(80.), None),
                    (Some(1.), Some(1.)),
                    (Some(100.), Some(100.)),
                    (Some(200.), None),
                    (Some(300.), None),
                    (Some(200.), None),
                    (Some(200.), None),
                ][row]
            };
            if let NodeContent::Control { size: s, .. } = &mut e.document.pages[0]
                .content
                .nodes
                .last_mut()
                .unwrap()
                .content
            {
                if let Some(w) = size.0 {
                    s.width = Dimension::Fixed(w);
                }
                if let Some(h) = size.1 {
                    s.height = Dimension::Fixed(h);
                }
            }
            if col == 2 || col == 4 {
                let value = match row {
                    1 => Some("A longer project title"),
                    3 => Some("Left aligned"),
                    4 => Some("Centered"),
                    5 => Some("*Bold* _Italic_ [link](https://example.com)"),
                    6 => Some(""),
                    7 => Some("One\nTwo\\rThree {mockup-name}"),
                    _ => None,
                };
                if let Some(v) = value {
                    e.set_control_property(id, "text", Value::Text(v.into()))?;
                }
                if row == 4 || row == 5 {
                    e.set_control_property(
                        id,
                        "align",
                        Value::Choice(if row == 4 { "center" } else { "right" }.into()),
                    )?;
                }
                if row == 5 {
                    e.set_control_property(id, "font_size", Value::Number(20.))?;
                    e.set_control_property(id, "text_color", Value::Color([0, 51, 204, 255]))?;
                }
            } else if col == 3 {
                let value = match row {
                    1..=4 => Some("one two three"),
                    5 => Some("web\\ design web\\ 2.0 {color:#FF0000}red{color} blue"),
                    6 => Some(""),
                    7 => Some("{mockup-name} One\nTwo [link](https://example.com)"),
                    _ => None,
                };
                if let Some(v) = value {
                    e.set_control_property(id, "text", Value::Text(v.into()))?;
                }
            }
        }
    }
    e.selection.clear();
    e.document.settings.sketch = false;
    e.document.settings.font_size = 13.;
    fs::write(dir.join("Text-layouts.wireframe"), e.document.encode()?)?;
    for (style, sketch) in [("clean", false), ("sketch", true)] {
        for font in [13., 24.] {
            e.document.settings.sketch = sketch;
            e.document.settings.font_size = font;
            let scene = e.rendered_scene(e.page)?;
            fs::write(dir.join(format!("{style}-{font}.png")), scene.png()?)?;
            fs::write(dir.join(format!("{style}-{font}.svg")), scene.svg())?;
        }
    }
    Ok(())
}
