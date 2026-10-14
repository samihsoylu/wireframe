// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Calendar sizes and editable date fields in both styles.
use std::{fs, path::PathBuf};
use wireframe::{
    document::{Project, Value},
    model::Kind,
    workspace::Editor,
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .unwrap_or_else(|| "date-controls".into()),
    );
    fs::create_dir_all(&out)?;
    let mut e = Editor::new(Project::default());
    e.document.title = "Calendars and dates".into();
    e.document.pages[0].name = "Dates".into();
    e.document.settings.sketch = false;
    for (x, y, w, h) in [
        (20., 20., 96., 96.),
        (160., 20., 200., 200.),
        (410., 20., 300., 80.),
        (20., 270., 300., 300.),
    ] {
        e.add(Kind::Calendar, x, y);
        e.selected_mut(|c| {
            c.bounds.w = w;
            c.bounds.h = h;
        });
    }
    for (n, text) in [
        "  /  /    ",
        "21/03/2028",
        "21/03/2028",
        "Choose a date",
        "Morning\\rAfternoon",
        "",
    ]
    .iter()
    .enumerate()
    {
        e.add(Kind::DateChooser, 380., 170. + n as f32 * 70.);
        let id = *e.selection.iter().next().unwrap();
        e.set_control_property(id, "text", Value::Text((*text).into()))?;
        match n {
            1 => e.set_control_property(id, "state", Value::Choice("focused".into()))?,
            2 => e.set_control_property(id, "state", Value::Choice("disabled".into()))?,
            3 => {
                e.set_control_property(id, "font_size", Value::Number(24.))?;
                e.set_control_property(id, "border_color", Value::Color([120, 70, 160, 255]))?;
            }
            5 => e.selected_mut(|c| c.bounds.w = 1.),
            _ => {}
        }
    }
    fs::write(out.join("Dates.wireframe"), e.document.encode()?)?;
    for (name, sketch) in [("clean", false), ("sketch", true)] {
        e.document.settings.sketch = sketch;
        let scene = e.rendered_scene(e.page)?;
        fs::write(out.join(format!("{name}.png")), scene.png()?)?;
        fs::write(out.join(format!("{name}.svg")), scene.svg())?;
    }
    Ok(())
}
