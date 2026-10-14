// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Reproducible Tablet gallery with a fixed clock for image comparisons.
use std::{fs, path::PathBuf};
use wireframe::{
    document::{Project, Value},
    model::Kind,
    tablet::Clock,
    workspace::Editor,
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .unwrap_or_else(|| "tablets".into()),
    );
    fs::create_dir_all(&out)?;
    let mut e = Editor::with_clock(Project::default(), || Clock::new(9, 52).unwrap());
    e.document.title = "Tablet frames".into();
    e.document.pages[0].name = "Tablet examples".into();
    e.document.settings.sketch = false;
    e.add(Kind::FieldSet, 670., 10.);
    let back = *e.selection.iter().next().unwrap();
    e.set_control_property(back, "text", Value::Text(String::new()))?;
    e.set_control_property(back, "fill", Value::Color([30, 170, 80, 255]))?;
    e.selected_mut(|c| {
        c.bounds.w = 320.;
        c.bounds.h = 440.;
    });
    let cases = [
        (20., 20., 300., 420., false, false, false, true),
        (350., 20., 300., 420., false, true, false, true),
        (680., 20., 300., 420., false, true, true, true),
        (1010., 20., 300., 420., false, true, false, false),
        (20., 480., 400., 280., true, false, false, true),
        (450., 480., 400., 280., true, true, false, true),
        (880., 480., 71., 280., false, true, false, true),
        (980., 480., 72., 280., false, true, false, true),
        (20., 800., 300., 201., false, true, false, true),
        (350., 800., 300., 202., false, true, false, true),
        (680., 800., 300., 203., false, true, false, true),
    ];
    for (x, y, w, h, landscape, stretch, transparent, status) in cases {
        e.add(Kind::Tablet, x, y);
        let id = *e.selection.iter().next().unwrap();
        e.set_control_property(
            id,
            "orientation",
            Value::Choice(if landscape { "landscape" } else { "portrait" }.into()),
        )?;
        e.set_control_property(
            id,
            "device_resizing",
            Value::Choice(if stretch { "stretch" } else { "fit" }.into()),
        )?;
        e.set_control_property(id, "transparent_background", Value::Bool(transparent))?;
        e.set_control_property(id, "status_bar", Value::Bool(status))?;
        e.selected_mut(|c| {
            c.bounds.w = w;
            c.bounds.h = h;
        });
    }
    fs::write(out.join("Tablet-frames.wireframe"), e.document.encode()?)?;
    for (name, sketch) in [("clean", false), ("sketch", true)] {
        e.document.settings.sketch = sketch;
        let scene = e.rendered_scene(e.page)?;
        fs::write(out.join(format!("{name}.svg")), scene.svg())?;
        fs::write(out.join(format!("{name}.png")), scene.png()?)?;
    }
    Ok(())
}
