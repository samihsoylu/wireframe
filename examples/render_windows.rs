// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Reproducible desktop-window design gallery.
use std::{fs, path::PathBuf};
use wireframe::{document::Value, model::Kind, workspace::Editor};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .unwrap_or_else(|| "window-gallery".into()),
    );
    fs::create_dir_all(&out)?;
    let mut e = Editor::default();
    e.document.title = "Desktop layouts".into();
    e.document.pages[0].name = "Window concepts".into();
    e.document.settings.sketch = false;
    for (i, title) in [
        "Project overview",
        "*Workspace*",
        "Details",
        "Notes\\rcontinued",
        "A long title with more detail than the available header space can hold",
        "Compact",
    ]
    .into_iter()
    .enumerate()
    {
        e.add(
            Kind::Panel,
            50. + (i % 3) as f32 * 520.,
            50. + (i / 3) as f32 * 320.,
        );
        let id = *e.selection.iter().next().unwrap();
        e.selected_mut(|c| c.bounds.h = 240.);
        e.set_control_property(id, "text", Value::Text(title.into()))?;
        if i > 0 {
            for k in ["minimize", "maximize", "close", "resize_handle"] {
                e.set_control_property(id, k, Value::Bool(true))?;
            }
        }
        if i == 2 {
            e.set_control_property(id, "header_height", Value::Number(60.))?;
            e.set_control_property(id, "footer_height", Value::Number(55.))?;
        }
        if i >= 3 {
            e.set_control_property(id, "vertical_scrollbar", Value::Bool(true))?;
            e.set_control_property(id, "value", Value::Number((i - 3) as f64 / 2.))?;
        }
        if i == 5 {
            e.selected_mut(|c| c.bounds.w = 180.);
        }
    }
    fs::write(out.join("Desktop-layouts.wireframe"), e.document.encode()?)?;
    for (name, sketch) in [("clean", false), ("sketch", true)] {
        e.document.settings.sketch = sketch;
        let scene = e.rendered_scene(e.page)?;
        fs::write(out.join(format!("{name}.svg")), scene.svg())?;
        fs::write(out.join(format!("{name}.png")), scene.png()?)?;
    }
    Ok(())
}
