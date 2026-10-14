// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Mobile keyboard variants and proportional fitting examples.
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
            .unwrap_or_else(|| "mobile-keyboards".into()),
    );
    fs::create_dir_all(&out)?;
    let mut e = Editor::new(Project::default());
    e.document.title = "Mobile keyboards".into();
    e.document.pages[0].name = "Keyboard layouts".into();
    e.document.settings.sketch = false;
    for (x, y, w, h, tablet, landscape) in [
        (20., 20., 226., 160., false, false),
        (300., 20., 403., 114., false, true),
        (20., 220., 546., 188., true, false),
        (20., 440., 728., 251., true, true),
        (20., 730., 220., 220., false, false),
        (270., 730., 220., 220., false, true),
        (520., 730., 220., 220., true, false),
        (770., 730., 220., 220., true, true),
    ] {
        e.add(Kind::MobileKeyboard, x, y);
        let id = *e.selection.iter().next().unwrap();
        e.set_control_property(
            id,
            "keyboard_device",
            Value::Choice(if tablet { "tablet" } else { "phone" }.into()),
        )?;
        e.set_control_property(
            id,
            "orientation",
            Value::Choice(if landscape { "landscape" } else { "portrait" }.into()),
        )?;
        e.selected_mut(|c| {
            c.bounds.w = w;
            c.bounds.h = h;
        });
    }
    fs::write(out.join("Mobile-keyboards.wireframe"), e.document.encode()?)?;
    for (name, sketch) in [("clean", false), ("sketch", true)] {
        e.document.settings.sketch = sketch;
        let scene = e.rendered_scene(e.page)?;
        fs::write(out.join(format!("{name}.svg")), scene.svg())?;
        fs::write(out.join(format!("{name}.png")), scene.png()?)?;
    }
    Ok(())
}
