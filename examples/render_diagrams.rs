// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Diagram gallery plus deterministic geometry diagnostics with synthetic metrics.
use std::{fs, path::PathBuf};
use wireframe::{
    document::{Dimension, NodeContent, Value},
    layout::{TextMeasurer, TextMetrics, TextStyle},
    model::Kind,
    popover::{self, Side},
    rich_text::RichText,
    site_map,
    workspace::Editor,
};
#[derive(serde::Deserialize)]
struct Case {
    family: String,
    #[serde(default)]
    width: f32,
    #[serde(default)]
    height: f32,
    #[serde(default)]
    sketch: bool,
    #[serde(default)]
    text: String,
    #[serde(default)]
    side: String,
    #[serde(default)]
    position: u8,
    #[serde(default)]
    horizontal: bool,
    #[serde(default = "default_font")]
    font_size: f32,
}
fn default_font() -> f32 {
    13.
}
struct Metrics;
impl TextMeasurer for Metrics {
    fn measure(&self, text: &str, style: TextStyle, _: Option<f32>) -> TextMetrics {
        let value = text.replace("\\r", "\n");
        let lines: Vec<_> = value.split('\n').collect();
        TextMetrics {
            width: lines.iter().map(|s| s.chars().count()).max().unwrap_or(0) as f32
                * style.size
                * 0.5,
            height: lines.len() as f32 * style.size * 1.2,
            rows: lines.len(),
        }
    }
    fn measure_rich(
        &self,
        text: &RichText,
        style: TextStyle,
        width: Option<f32>,
        _: bool,
    ) -> TextMetrics {
        self.measure(&text.source, style, width)
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.get(1).is_some_and(|s| s == "--geometry") {
        let cases: Vec<Case> = serde_json::from_slice(&fs::read(&args[2])?)?;
        let results:Vec<_>=cases.into_iter().map(|c|if c.family=="popover" {let w=if c.width<0.{222.}else{c.width};let h=if c.height<0.{187.}else{c.height};serde_json::to_value(popover::layout(w,h,Side::parse(&c.side),c.position,c.sketch)).unwrap()}else{let tree=site_map::Tree::parse(&c.text);serde_json::json!({"lines":tree.nodes.iter().map(|n|&n.line).collect::<Vec<_>>(),"layout":tree.layout(&Metrics,TextStyle{size:c.font_size,bold:false,italic:false},c.horizontal)})}).collect();
        println!("{}", serde_json::to_string(&results)?);
        return Ok(());
    }
    let dir = PathBuf::from(args.get(1).cloned().unwrap_or("diagram-gallery".into()));
    fs::create_dir_all(&dir)?;
    let mut e = Editor::default();
    e.document.title = "Panels and page maps".into();
    e.document.pages[0].name = "Diagrams".into();
    for (col, side) in ["top", "right", "bottom", "left"].into_iter().enumerate() {
        for (row, position) in [2., 0., 1., 3., 4.].into_iter().enumerate() {
            e.add(
                Kind::Popover,
                20. + 280. * col as f32,
                20. + 240. * row as f32,
            );
            let id = *e.selection.iter().next().unwrap();
            e.set_control_property(id, "direction", Value::Choice(side.into()))?;
            e.set_control_property(id, "arrow_position", Value::Number(position))?;
        }
    }
    for (i, (w, h, text)) in [
        (1., 1., ""),
        (40., 100., ""),
        (41., 100., ""),
        (70., 100., ""),
        (71., 100., ""),
        (72., 100., ""),
        (73., 100., ""),
        (222., 187., "Panel title\nDetails"),
    ]
    .into_iter()
    .enumerate()
    {
        e.add(
            Kind::Popover,
            20. + 280. * (i % 4) as f32,
            1220. + 240. * (i / 4) as f32,
        );
        let id = *e.selection.iter().next().unwrap();
        e.set_control_property(id, "text", Value::Text(text.into()))?;
        if let NodeContent::Control { size, .. } = &mut e.document.pages[0]
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
    for (col, direction) in ["vertical", "horizontal"].into_iter().enumerate() {
        for (row, text) in [
            site_map::DEFAULT,
            "Root\n-- One\n-- Two",
            "Start\n\tSection\n\t\tPage",
            "Root\nOther",
            "Root\n- *Bold*\n- _Italic_\n- [Link](https://example.com)",
        ]
        .into_iter()
        .enumerate()
        {
            e.add(
                Kind::SiteMap,
                20. + 800. * col as f32,
                1740. + 400. * row as f32,
            );
            let id = *e.selection.iter().next().unwrap();
            e.set_control_property(id, "text", Value::Text(text.into()))?;
            e.set_control_property(id, "direction", Value::Choice(direction.into()))?;
        }
    }
    e.selection.clear();
    e.document.settings.sketch = false;
    fs::write(dir.join("Diagrams.wireframe"), e.document.encode()?)?;
    for (style, sketch) in [("clean", false), ("sketch", true)] {
        e.document.settings.sketch = sketch;
        let scene = e.rendered_scene(e.page)?;
        fs::write(dir.join(format!("{style}.png")), scene.png()?)?;
        fs::write(dir.join(format!("{style}.svg")), scene.svg())?;
    }
    Ok(())
}
