// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Reproducible form-section gallery, including a transparent overlap.
use std::{fs, path::PathBuf};
use wireframe::{document::Value, model::Kind, workspace::Editor};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .unwrap_or_else(|| "form-sections".into()),
    );
    fs::create_dir_all(&out)?;
    let mut e = Editor::default();
    e.document.title = "Form sections".into();
    e.document.pages[0].name = "Section examples".into();
    e.document.settings.sketch = false;
    for (i, text) in [
        "Group Name",
        "",
        "A much longer form section heading",
        "Account *details*\\roptional {mockup-name}",
        "Dark background",
        "Half opacity",
        "Transparent",
        "Large title",
    ]
    .into_iter()
    .enumerate()
    {
        e.add(
            Kind::FieldSet,
            20. + (i % 4) as f32 * 230.,
            20. + (i / 4) as f32 * 220.,
        );
        let id = *e.selection.iter().next().unwrap();
        e.set_control_property(id, "text", Value::Text(text.into()))?;
        if i == 2 {
            e.selected_mut(|c| {
                c.bounds.w = 90.;
                c.bounds.h = 30.;
            });
        }
        if (4..=6).contains(&i) {
            e.set_control_property(id, "fill", Value::Color([0, 0, 0, 255]))?;
            e.set_control_property(
                id,
                "background_opacity",
                Value::Number([1., 0.5, 0.][i - 4]),
            )?;
        }
        if i == 7 {
            e.set_control_property(id, "font_size", Value::Number(32.))?;
            e.selected_mut(|c| {
                c.bounds.w = 200.;
                c.bounds.h = 100.;
            });
        }
    }
    e.add(Kind::FieldSet, 20., 460.);
    let back = *e.selection.iter().next().unwrap();
    e.set_control_property(back, "fill", Value::Color([30, 170, 80, 255]))?;
    e.set_control_property(back, "text", Value::Text(String::new()))?;
    e.add(Kind::FieldSet, 40., 480.);
    let front = *e.selection.iter().next().unwrap();
    e.set_control_property(
        front,
        "text",
        Value::Text("Overlapping transparent section".into()),
    )?;
    e.set_control_property(front, "background_opacity", Value::Number(0.))?;
    fs::write(out.join("Form-sections.wireframe"), e.document.encode()?)?;
    for (name, sketch) in [("clean", false), ("sketch", true)] {
        e.document.settings.sketch = sketch;
        let s = e.rendered_scene(e.page)?;
        fs::write(out.join(format!("{name}.svg")), s.svg())?;
        fs::write(out.join(format!("{name}.png")), s.png()?)?;
    }
    Ok(())
}
