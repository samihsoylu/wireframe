// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Generate an editable dialog gallery with two distinct destinations.
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
            .unwrap_or_else(|| "dialog-gallery".into()),
    );
    fs::create_dir_all(&out)?;
    let mut e = Editor::default();
    e.document.title = "Project decisions".into();
    e.document.pages[0].name = "Dialogs".into();
    e.document.settings.sketch = false;
    let page = e.page;
    let mut destinations = Vec::new();
    for name in ["Draft retained", "Changes applied"] {
        e.add_page();
        destinations.push(e.page);
        e.document.pages.last_mut().unwrap().name = name.into();
        e.add(Kind::MenuBar, 50., 50.);
        let id = *e.selection.iter().next().unwrap();
        e.set_control_property(id, "text", Value::Text("Return, Help".into()))?;
        e.set_control_property(
            id,
            "row_links",
            Value::Links(vec![Some(Link::Page(page)), None]),
        )?;
    }
    e.switch_page(page);
    for (index, source) in [
        "Confirm changes\nApply these changes to the project?\nKeep draft, Apply",
        "Save project\nSave your latest changes?\nKeep draft, Apply",
        "Project ready\nYour work is ready for review.\nContinue",
        "*Review*\\rchanges\nCheck the details\\rbefore continuing.\nKeep\\rdraft,Apply",
        "Notice\nMessage\nA very long left button,Right",
        "Compact preview\nMessage\nKeep draft, Apply",
        "Quiet confirmation\nContinue with the update?\nKeep draft, Apply",
        "",
    ]
    .iter()
    .enumerate()
    {
        e.add(
            Kind::Alert,
            50. + (index % 3) as f32 * 330.,
            50. + (index / 3) as f32 * 280.,
        );
        let id = *e.selection.iter().next().unwrap();
        e.set_control_property(id, "text", Value::Text((*source).into()))?;
        e.set_control_property(
            id,
            "row_links",
            Value::Links(vec![
                Some(Link::Page(destinations[0])),
                Some(Link::Page(destinations[1])),
            ]),
        )?;
        if index == 1 || index == 4 {
            e.set_control_property(id, "button_style", Value::Choice("inline".into()))?;
        }
        if index == 3 {
            e.set_control_property(id, "font_size", Value::Number(20.))?;
        }
        if index == 6 {
            e.set_control_property(id, "border", Value::Choice("none".into()))?;
            e.set_control_property(id, "text_color", Value::Color([153, 46, 20, 255]))?;
        }
        e.auto_size_control(id)?;
        if index == 4 || index == 5 {
            e.selected_mut(|c| {
                c.bounds.w = if index == 4 { 70. } else { 200. };
                c.bounds.h = if index == 4 { 190. } else { 40. };
            });
        }
    }
    fs::write(
        out.join("Project-decisions.wireframe"),
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
