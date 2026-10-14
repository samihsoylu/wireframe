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
        .unwrap_or_else(|| "evidence/tabs-gallery".into());
    fs::create_dir_all(&out)?;
    if out.join("Tabs.wireframe").exists() {
        return Err("Choose a fresh directory to preserve existing fixtures".into());
    }
    let mut e = Editor::default();
    e.document.title = "Tab controls".into();
    e.document.settings.sketch = false;
    let first = e.page;
    e.document.pages[0].name = "Tabs".into();
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
    let mut ids = vec![];
    for i in 0..16 {
        e.add(
            if i < 8 {
                Kind::Tabs
            } else {
                Kind::VerticalTabs
            },
            50. + (i % 4) as f32 * 250.,
            50. + (i / 4) as f32 * 170.,
        );
        let id = *e.selection.iter().next().unwrap();
        ids.push(id);
        e.begin("Configure tabs");
        let mut props = vec![];
        if [1, 7, 9, 11, 15].contains(&i) {
            props.push(("selected_index", Value::Number(1.)));
        }
        if [2, 3].contains(&i) {
            props.push(("selected_index", Value::Number(2.)));
        }
        if i == 10 {
            props.push(("selected_index", Value::Number(3.)));
        }
        if i == 1 {
            props.extend([
                ("tab_position", Value::Choice("bottom".into())),
                ("align", Value::Choice("center".into())),
            ]);
        }
        if i == 3 {
            props.push(("align", Value::Choice("right".into())));
        }
        if i == 9 {
            props.push(("tab_position", Value::Choice("right".into())));
        }
        if i == 4 {
            props.push(("text", Value::Text(" A\\,B,  C,,D".into())));
        }
        if [5, 12].contains(&i) {
            props.push(("text", Value::Text("One".into())));
        }
        if i == 13 {
            props.push(("text", Value::Text("A\n\nC".into())));
        }
        if [6, 14].contains(&i) {
            props.extend([
                (
                    "text",
                    Value::Text(
                        if i == 6 {
                            "*First*\nline, Second"
                        } else {
                            "*First tab*\n_Second tab_"
                        }
                        .into(),
                    ),
                ),
                ("font_size", Value::Number(24.)),
                ("selected_index", Value::Number(0.)),
            ]);
        }
        if [7, 15].contains(&i) {
            props.extend([
                ("fill", Value::Color([0, 0, 0, 255])),
                (
                    "background_opacity",
                    Value::Number(if i == 7 { 0.5 } else { 1. }),
                ),
                ("border", Value::Choice("none".into())),
                ("vertical_scrollbar", Value::Bool(true)),
                ("value", Value::Number(0.75)),
            ]);
        }
        if i == 2 {
            props.push((
                "row_links",
                Value::Links(vec![None, None, Some(Link::Page(destination)), None]),
            ));
        }
        if i == 10 {
            props.push((
                "row_links",
                Value::Links(vec![None, None, None, Some(Link::Page(destination))]),
            ));
        }
        if [4, 13].contains(&i) {
            props.push((
                "row_links",
                Value::Links(vec![Some(Link::Url("https://example.org/tab".into()))]),
            ));
        }
        for (key, value) in props {
            e.set_control_property(id, key, value)?;
        }
        let fixed = match i {
            1 => Some((230., 130.)),
            2 | 3 => Some((90., 100.)),
            7 => Some((220., 110.)),
            10 => Some((200., 60.)),
            11 => Some((50., 150.)),
            15 => Some((220., 150.)),
            _ => None,
        };
        if let Some((w, h)) = fixed {
            e.selected_mut(|c| {
                c.bounds.w = w;
                c.bounds.h = h;
            });
        }
        e.commit();
    }
    fs::write(out.join("Tabs.wireframe"), e.document.encode()?)?;
    let mut geometry = vec![];
    for (sketch, name) in [(false, "clean"), (true, "sketch")] {
        e.document.settings.sketch = sketch;
        let mut cases = vec![];
        for (i, &id) in ids.iter().enumerate() {
            e.select(id, false);
            cases.push((i + 1, id, e.selected_views()[0].bounds));
        }
        geometry.push((name, cases));
        let s = e.rendered_scene(first)?;
        fs::write(out.join(format!("{name}.png")), s.png()?)?;
        fs::write(out.join(format!("{name}.svg")), s.svg())?;
    }
    fs::write(
        out.join("geometry.json"),
        serde_json::to_vec_pretty(&geometry)?,
    )?;
    println!("{}", out.display());
    Ok(())
}
