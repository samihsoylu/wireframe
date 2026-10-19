// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Paired base/preset controls and progress states for visual review.
use std::{fs, path::PathBuf};
use wireframe::{
    document::{NodeContent, Project, Size, Value},
    model::Kind,
    workspace::{CatalogPreset, Editor},
};
fn label(e: &mut Editor, text: &str, x: f32, y: f32) -> Result<(), String> {
    e.add(Kind::Label, x, y);
    let id = *e.selection.iter().next().unwrap();
    e.set_control_property(id, "text", Value::Text(text.into()))?;
    if let NodeContent::Control { size, .. } = &mut e.document.pages[0]
        .content
        .nodes
        .last_mut()
        .unwrap()
        .content
    {
        *size = Size::fixed(310., 28.);
    }
    Ok(())
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = PathBuf::from(std::env::args().nth(1).unwrap_or("catalog-presets".into()));
    fs::create_dir_all(&out)?;
    if out.join("Catalog presets.wireframe").exists() {
        return Err("Choose a fresh output directory".into());
    }
    let mut e = Editor::new(Project::default());
    e.document.title = "Catalog presets".into();
    e.document.pages[0].name = "Base and preset".into();
    for (i, preset) in CatalogPreset::ALL.into_iter().enumerate() {
        let y = 20. + i as f32 * 155.;
        label(&mut e, preset.kind().name(), 20., y)?;
        label(&mut e, preset.name(), 350., y)?;
        e.add(preset.kind(), 20., y + 35.);
        e.add_preset(preset, 350., y + 35.);
    }
    let first = e.page;
    e.add_page();
    e.document.pages.last_mut().unwrap().name = "Progress states".into();
    for (thin, value, indeterminate, x, y) in [
        (true, 0.5, false, 50., 50.),
        (false, 0.5, false, 220., 50.),
        (true, 0.8, false, 50., 100.),
        (true, 0.5, true, 50., 150.),
        (false, 0.5, true, 220., 150.),
    ] {
        if thin {
            e.add_preset(CatalogPreset::AndroidProgress, x, y);
        } else {
            e.add(Kind::Progress, x, y);
        }
        let id = *e.selection.iter().next().unwrap();
        e.set_control_property(id, "value", Value::Number(value))?;
        e.set_control_property(id, "indeterminate", Value::Bool(indeterminate))?;
    }
    e.selection.clear();
    fs::write(out.join("Catalog presets.wireframe"), e.document.encode()?)?;
    for (style, sketch) in [("sketch", true), ("clean", false)] {
        e.document.settings.sketch = sketch;
        for (name, page) in [("presets", first), ("progress", e.page)] {
            let scene = e.rendered_scene(page)?;
            fs::write(out.join(format!("{name}-{style}.png")), scene.png()?)?;
            fs::write(out.join(format!("{name}-{style}.svg")), scene.svg())?;
        }
    }
    Ok(())
}
