// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Local annotation gallery and numeric placement diagnostics.
use std::{fs, path::PathBuf};
use wireframe::{
    annotations::{self, Direction, Family},
    document::{Project, Value},
    layout::TextMetrics,
    model::Kind,
    workspace::Editor,
};
#[derive(serde::Deserialize)]
struct Case {
    family: Family,
    width: f32,
    height: f32,
    sketch: bool,
    metric: [f32; 2],
    left: bool,
    direction: Direction,
    fill: [u8; 3],
    opacity: f32,
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.get(1).is_some_and(|s| s == "--geometry") {
        let cases: Vec<Case> = serde_json::from_slice(&fs::read(&args[2])?)?;
        let out:Vec<_>=cases.into_iter().map(|c|{let m=TextMetrics{width:c.metric[0],height:c.metric[1],rows:1};let natural=c.family.intrinsic(m,c.sketch);let(w,h)=if c.width<0.||c.height<0.{natural}else{(c.width,c.height)};serde_json::json!({"natural":natural,"layout":annotations::layout(c.family,w,h,c.sketch,m,c.left,c.direction),"light_text":annotations::light_text(c.family,c.fill,c.opacity,c.sketch)})}).collect();
        println!("{}", serde_json::to_string_pretty(&out)?);
        return Ok(());
    }
    let out = PathBuf::from(args.get(1).cloned().unwrap_or("annotation-gallery".into()));
    fs::create_dir_all(&out)?;
    let mut e = Editor::new(Project::default());
    e.document.title = "Annotations".into();
    e.document.pages[0].name = "Review notes".into();
    for i in 0..9 {
        e.add(Kind::Callout, 20. + i as f32 * 90., 20.);
    }
    for (i, text) in [
        "*Check* spacing",
        "First line\nSecond line",
        "{mockup-name}",
    ]
    .iter()
    .enumerate()
    {
        e.add(Kind::Callout, 20. + i as f32 * 300., 100.);
        let id = *e.selection.iter().next().unwrap();
        e.set_control_property(id, "text", Value::Text((*text).into()))?;
    }
    for (i, align) in ["left", "center", "right"].into_iter().enumerate() {
        e.add(Kind::StickyNote, 20. + i as f32 * 300., 210.);
        let id = *e.selection.iter().next().unwrap();
        e.selected_mut(|c| {
            c.bounds.w = 240.;
            c.bounds.h = 150.;
        });
        e.set_control_property(id, "text", Value::Text("Short note".into()))?;
        e.set_control_property(id, "align", Value::Choice(align.into()))?;
    }
    for (i, (kind, text)) in [
        (Kind::Callout, "Opaque"),
        (Kind::Callout, "Transparent"),
        (
            Kind::StickyNote,
            "*Review*\n[Open](https://example.org)\n- Discuss",
        ),
    ]
    .into_iter()
    .enumerate()
    {
        e.add(kind, 20. + i as f32 * 300., 410.);
        let id = *e.selection.iter().next().unwrap();
        e.selected_mut(|c| {
            c.bounds.w = 240.;
            c.bounds.h = 120.;
        });
        e.set_control_property(id, "text", Value::Text(text.into()))?;
        e.set_control_property(id, "fill", Value::Color([20, 35, 55, 255]))?;
        if i == 1 {
            e.set_control_property(id, "background_opacity", Value::Number(0.))?;
        }
    }
    for (i, d) in ["top_left", "top_right", "bottom_left", "bottom_right"]
        .iter()
        .enumerate()
    {
        e.add(Kind::Tooltip, 20. + i as f32 * 220., 600.);
        let id = *e.selection.iter().next().unwrap();
        e.selected_mut(|c| {
            c.bounds.w = 190.;
            c.bounds.h = 80.;
        });
        e.set_control_property(id, "direction", Value::Choice((*d).into()))?;
        e.set_control_property(
            id,
            "text",
            Value::Text("*Hint* with a\n[link](https://example.org)".into()),
        )?;
    }
    for (i, (w, h)) in [(31., 28.), (32., 27.), (32., 28.), (1., 1.)]
        .into_iter()
        .enumerate()
    {
        e.add(Kind::Tooltip, 20. + i as f32 * 100., 750.);
        e.selected_mut(|c| {
            c.bounds.w = w;
            c.bounds.h = h;
        });
    }
    e.add(Kind::Tooltip, 500., 750.);
    let id = *e.selection.iter().next().unwrap();
    e.set_control_property(id, "text", Value::Text("{mockup-name}".into()))?;
    e.selection.clear();
    e.document.settings.sketch = false;
    fs::write(out.join("Annotations.wireframe"), e.document.encode()?)?;
    for (name, sketch) in [("clean", false), ("sketch", true)] {
        e.document.settings.sketch = sketch;
        let s = e.rendered_scene(e.page)?;
        fs::write(out.join(format!("{name}.svg")), s.svg())?;
        fs::write(out.join(format!("{name}.png")), s.png()?)?;
    }
    Ok(())
}
