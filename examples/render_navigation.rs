// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use std::{fs, path::PathBuf};
use wireframe::{
    document::{Link, Value},
    model::Kind,
    workspace::Editor,
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| "evidence/navigation".into());
    fs::create_dir_all(&out)?;
    if out.join("Navigation.wireframe").exists() {
        return Err("Choose a fresh directory to preserve existing fixtures".into());
    }
    let mut e = Editor::default();
    e.document.title = "Navigation controls".into();
    e.document.settings.sketch = false;
    let first = e.page;
    e.document.pages[0].name = "Navigation".into();
    e.add_page();
    let destination = e.page;
    e.document.pages[1].name = "Destination".into();
    e.add(Kind::LinkBar, 50., 50.);
    let back = *e.selection.iter().next().unwrap();
    e.set_control_property(back, "text", Value::Text("Back, Done".into()))?;
    e.set_control_property(
        back,
        "row_links",
        Value::Links(vec![Some(Link::Back), None]),
    )?;
    e.switch_page(first);
    let texts = [
        "Home, Products, Xyz, Features",
        "Home",
        "A,, C",
        " A,  B,C",
        "A\\,B, C",
        "First\nline, Second",
        "*Home*, _Products_, Last",
        "One, Two, Three",
        "Home, Products, Company, Blog",
        "One, Two, Three",
        "One",
        "A,, C",
        " A\\,B,  C",
        "First\nline, Second",
        "*One*, Two, Three",
        "One, Two, Three",
    ];
    let mut geometry = vec![];
    for (i, text) in texts.iter().enumerate() {
        e.add(
            if i < 8 {
                Kind::Breadcrumbs
            } else {
                Kind::LinkBar
            },
            50. + (i % 4) as f32 * 250.,
            50. + (i / 4) as f32 * 120.,
        );
        let id = *e.selection.iter().next().unwrap();
        e.begin("Configure navigation");
        e.set_control_property(id, "text", Value::Text((*text).into()))?;
        if i == 6 {
            e.set_control_property(id, "font_size", Value::Number(24.))?;
        }
        if [9, 14].contains(&i) {
            e.set_control_property(id, "selected_index", Value::Number(1.))?;
        }
        if i == 13 {
            e.set_control_property(id, "selected_index", Value::Number(0.))?;
        }
        if i == 14 {
            e.set_control_property(id, "text_color", Value::Color([255, 0, 0, 255]))?;
            e.set_control_property(id, "border_color", Value::Color([0, 128, 0, 255]))?;
        }
        if i == 0 {
            e.set_control_property(
                id,
                "row_links",
                Value::Links(vec![None, None, None, Some(Link::Page(destination))]),
            )?;
        }
        if i == 9 {
            e.set_control_property(
                id,
                "row_links",
                Value::Links(vec![None, Some(Link::Page(destination)), None]),
            )?;
        }
        if [4, 12].contains(&i) {
            e.set_control_property(
                id,
                "row_links",
                Value::Links(vec![
                    Some(Link::Url("https://example.org/navigation".into())),
                    None,
                ]),
            )?;
        }
        if [7, 15].contains(&i) {
            e.selected_mut(|c| {
                c.bounds.w = 20.;
                c.bounds.h = 50.;
            });
        }
        e.commit();
        geometry.push((i + 1, id, e.selected_views()[0].bounds));
    }
    fs::write(out.join("Navigation.wireframe"), e.document.encode()?)?;
    fs::write(
        out.join("geometry.json"),
        serde_json::to_vec_pretty(&geometry)?,
    )?;
    let scene = e.rendered_scene(e.page)?;
    fs::write(out.join("clean.png"), scene.png()?)?;
    fs::write(out.join("clean.svg"), scene.svg())?;
    e.document.settings.sketch = true;
    let scene = e.rendered_scene(e.page)?;
    fs::write(out.join("sketch.png"), scene.png()?)?;
    fs::write(out.join("sketch.svg"), scene.svg())?;
    println!("{}", out.display());
    Ok(())
}
