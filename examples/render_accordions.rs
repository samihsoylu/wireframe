// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Generate an editable accordion gallery with both drawing styles.
use std::{fs, path::PathBuf};
use wireframe::{
    document::{Link, Value},
    model::Kind,
    workspace::Editor,
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .unwrap_or_else(|| "accordion-gallery".into()),
    );
    fs::create_dir_all(&out)?;
    let mut e = Editor::default();
    e.document.title = "Workspace panels".into();
    e.document.pages[0].name = "Panels".into();
    e.document.settings.sketch = false;
    let page = e.page;
    e.add_page();
    let destination = e.page;
    e.document.pages[1].name = "Overview".into();
    e.add(Kind::MenuBar, 50., 50.);
    let back = *e.selection.iter().next().unwrap();
    e.set_control_property(back, "text", Value::Text("Return, Help".into()))?;
    e.set_control_property(
        back,
        "row_links",
        Value::Links(vec![Some(Link::Page(page)), None]),
    )?;
    e.switch_page(page);
    let cases = [
        ("Overview\nPages\n- Landing\n- Settings\nAssets\nNotes", 0),
        ("Overview\nPages\n- Landing\n- Settings\nAssets\nNotes", 1),
        ("Overview\nPages\n- Landing\n- Settings\nAssets\nNotes", 2),
        (
            "*Project*\nPages\n- _Landing_\n- Settings\nAssets\nNotes",
            2,
        ),
        (
            "Overview\nPages\n- A longer page title\n- Settings\nAssets\nNotes",
            1,
        ),
        ("Overview\nPages\n- Landing\n- Settings\nAssets\nNotes", -1),
    ];
    for (index, (source, selected)) in cases.iter().enumerate() {
        e.add(
            Kind::Accordion,
            50. + (index % 3) as f32 * 300.,
            50. + (index / 3) as f32 * 340.,
        );
        let id = *e.selection.iter().next().unwrap();
        e.set_control_property(id, "text", Value::Text((*source).into()))?;
        e.set_control_property(id, "selected_index", Value::Number(*selected as f64))?;
        e.set_control_property(
            id,
            "row_links",
            Value::Links(vec![Some(Link::Page(destination)); 6]),
        )?;
        e.auto_size_control(id)?;
        if index == 3 || index == 4 {
            e.set_control_property(id, "vertical_scrollbar", Value::Bool(true))?;
            e.set_control_property(id, "value", Value::Number(0.75))?;
            e.selected_mut(|c| {
                c.bounds.w = 200.;
                c.bounds.h = if index == 4 { 60. } else { 280. };
            });
        }
    }
    fs::write(out.join("Workspace-panels.wireframe"), e.document.encode()?)?;
    for (name, sketch) in [("clean", false), ("sketch", true)] {
        e.document.settings.sketch = sketch;
        let scene = e.rendered_scene(page)?;
        fs::write(out.join(format!("{name}.png")), scene.png()?)?;
        fs::write(out.join(format!("{name}.svg")), scene.svg())?;
    }
    Ok(())
}
