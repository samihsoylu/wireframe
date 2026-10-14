// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Generate an editable offline Tree gallery with both drawing styles.
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
            .unwrap_or_else(|| "tree-gallery".into()),
    );
    fs::create_dir_all(&out)?;
    let mut e = Editor::default();
    e.document.title = "Project navigator".into();
    e.document.pages[0].name = "Tree gallery".into();
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
        "F Workspace\n f Pages\n..- Landing\n..- Settings\n f Media\n..- Logo",
        "[+] Backlog\n[-] Current sprint\n [x] Navigation\n [ ] Search\n[=] Archive",
        "F Recent\n - Planning\n - Notes\n - Sketches",
        "F Project\n - *Overview*\n - _Research_\n - -Archived-",
        "F A folder with a long title\n - The first document\n - Another document\n - More content",
        "F Transparent pane\n > Section\n..- A document\n_ Notes without a symbol",
    ];
    for (index, text) in cases.iter().enumerate() {
        e.add(
            Kind::Tree,
            50. + (index % 3) as f32 * 340.,
            50. + (index / 3) as f32 * 240.,
        );
        let id = *e.selection.iter().next().unwrap();
        e.set_control_property(id, "text", Value::Text((*text).into()))?;
        e.set_control_property(id, "selected_index", Value::Number(1.))?;
        e.set_control_property(
            id,
            "row_links",
            Value::Links(vec![None, Some(Link::Page(destination))]),
        )?;
        e.auto_size_control(id)?;
        if index == 2 {
            e.set_control_property(id, "state", Value::Choice("disabled".into()))?;
        }
        if index == 3 {
            e.set_control_property(id, "fill", Value::Color([25, 35, 45, 255]))?;
        }
        if index == 4 {
            e.set_control_property(id, "vertical_scrollbar", Value::Bool(true))?;
            e.set_control_property(id, "value", Value::Number(0.6))?;
            e.selected_mut(|c| {
                c.bounds.w = 160.;
                c.bounds.h = 65.;
            });
        }
        if index == 5 {
            e.set_control_property(id, "border", Value::Choice("none".into()))?;
            e.set_control_property(id, "background_opacity", Value::Number(0.))?;
        }
    }
    fs::write(
        out.join("Project-navigator.wireframe"),
        e.document.encode()?,
    )?;
    for (name, sketch) in [("clean", false), ("sketch", true)] {
        e.document.settings.sketch = sketch;
        let scene = e.rendered_scene(page)?;
        fs::write(out.join(format!("{name}.png")), scene.png()?)?;
        fs::write(out.join(format!("{name}.svg")), scene.svg())?;
    }
    Ok(())
}
