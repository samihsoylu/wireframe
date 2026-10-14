// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Reproducible device gallery with transparent and stretched screens.
use std::{fs, path::PathBuf};
use wireframe::{document::Value, model::Kind, workspace::Editor};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .unwrap_or_else(|| "phones".into()),
    );
    fs::create_dir_all(&out)?;
    let mut e = Editor::default();
    e.document.title = "Device frames".into();
    e.document.pages[0].name = "Phone examples".into();
    e.document.settings.sketch = false;
    e.add(Kind::FieldSet, 660., 490.);
    let back = *e.selection.iter().next().unwrap();
    e.set_control_property(back, "text", Value::Text(String::new()))?;
    e.set_control_property(back, "fill", Value::Color([30, 170, 80, 255]))?;
    e.selected_mut(|c| {
        c.bounds.w = 340.;
        c.bounds.h = 540.;
    });
    for (i, model) in [
        "classic", "tall", "wide", "notched", "notched", "notched", "notched", "classic",
    ]
    .into_iter()
    .enumerate()
    {
        e.add(
            Kind::Phone,
            20. + (i % 4) as f32 * 330.,
            20. + (i / 4) as f32 * 490.,
        );
        let id = *e.selection.iter().next().unwrap();
        for (key, value) in [
            ("device_model", model),
            ("device_resizing", if i < 4 { "fit" } else { "stretch" }),
            ("orientation", if i == 7 { "landscape" } else { "portrait" }),
            (
                "background_pattern",
                match i {
                    5 => "dark",
                    6 => "header_dark_footer",
                    7 => "header_footer",
                    _ => "white",
                },
            ),
        ] {
            e.set_control_property(id, key, Value::Choice(value.into()))?;
        }
        e.set_control_property(id, "transparent_background", Value::Bool(i == 6))?;
        e.selected_mut(|c| {
            c.bounds.w = if i < 4 { 270. } else { 300. };
            c.bounds.h = if i < 4 {
                430.
            } else if i == 7 {
                200.
            } else {
                500.
            };
        });
    }
    fs::write(out.join("Device-frames.wireframe"), e.document.encode()?)?;
    for (name, sketch) in [("clean", false), ("sketch", true)] {
        e.document.settings.sketch = sketch;
        let scene = e.rendered_scene(e.page)?;
        fs::write(out.join(format!("{name}.svg")), scene.svg())?;
        fs::write(out.join(format!("{name}.png")), scene.png()?)?;
    }
    Ok(())
}
