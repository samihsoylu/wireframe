// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Reproducible local browser-frame gallery, including overlapping cutouts.
use std::{fs, path::PathBuf};
use wireframe::{document::Value, model::Kind, workspace::Editor};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .unwrap_or_else(|| "browser-gallery".into()),
    );
    fs::create_dir_all(&out)?;
    let mut e = Editor::default();
    e.document.title = "Website concepts".into();
    e.document.pages[0].name = "Browser frames".into();
    e.document.settings.sketch = false;
    let sources = [
        "Project overview\nhttps://example.org/projects",
        "*Workspace*\nhttps://example.org/local",
        "{mockup-name}\nhttps://example.org",
        "A longer page title with additional information\nhttps://example.org/a/longer/address/path",
        "Notes\\rcontinued\nhttps://example.org\\rcontinued",
        "Small frame\nAddress",
    ];
    for (i, source) in sources.into_iter().enumerate() {
        e.add(
            Kind::Browser,
            50. + (i % 3) as f32 * 520.,
            50. + (i / 3) as f32 * 320.,
        );
        let id = *e.selection.iter().next().unwrap();
        e.selected_mut(|c| {
            c.bounds.h = 240.;
            if i == 5 {
                c.bounds.w = 173.;
            }
        });
        e.set_control_property(id, "text", Value::Text(source.into()))?;
        if i != 0 {
            e.set_control_property(id, "vertical_scrollbar", Value::Bool(true))?;
            e.set_control_property(id, "value", Value::Number((i % 3) as f64 / 2.))?;
        }
        if matches!(i, 1 | 2 | 5) {
            e.set_control_property(id, "border", Value::Choice("torn_bottom".into()))?;
        }
        if i == 1 {
            e.set_control_property(id, "fill", Value::Color([170, 216, 245, 255]))?;
        }
        if i == 2 {
            e.set_control_property(id, "fill", Value::Color([40, 50, 65, 255]))?;
        }
    }
    let gallery = e.page;
    e.add_page();
    e.document.pages[1].name = "Cutouts".into();
    for (height, fill, torn) in [
        (250., [30, 170, 80, 255], false),
        (180., [150, 200, 250, 255], true),
    ] {
        e.add(Kind::Browser, 50., 50.);
        let id = *e.selection.iter().next().unwrap();
        e.selected_mut(|c| c.bounds.h = height);
        e.set_control_property(id, "fill", Value::Color(fill))?;
        if torn {
            e.set_control_property(id, "border", Value::Choice("torn_bottom".into()))?;
        }
    }
    e.switch_page(gallery);
    fs::write(out.join("Website-concepts.wireframe"), e.document.encode()?)?;
    for (name, sketch) in [("clean", false), ("sketch", true)] {
        e.document.settings.sketch = sketch;
        let scene = e.rendered_scene(e.page)?;
        fs::write(out.join(format!("{name}.svg")), scene.svg())?;
        fs::write(out.join(format!("{name}.png")), scene.png()?)?;
    }
    Ok(())
}
