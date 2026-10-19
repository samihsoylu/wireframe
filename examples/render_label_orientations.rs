// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Editable orientation gallery plus shared SVG, PNG and searchable PDF exports.
use std::{fs, path::PathBuf};
use wireframe::{
    document::{Dimension, NodeContent, Value},
    model::Kind,
    pdf,
    workspace::Editor,
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = PathBuf::from(std::env::args().nth(1).ok_or("Output directory required")?);
    fs::create_dir_all(&out)?;
    let mut e = Editor::default();
    e.document.title = "Label orientations".into();
    e.document.pages[0].name = "Orientation gallery".into();
    for (column, orientation) in ["upright", "left", "right", "diagonal"].iter().enumerate() {
        for row in 0..4 {
            e.add(
                Kind::Label,
                50. + column as f32 * 240.,
                50. + row as f32 * 230.,
            );
            let id = *e.selection.iter().next().unwrap();
            let text = if row == 3 {
                "*Bold* _Italic_ [Visit](https://example.com)"
            } else {
                "Some text"
            };
            e.set_control_property(id, "text", Value::Text(text.into()))?;
            e.set_control_property(id, "text_orientation", Value::Choice((*orientation).into()))?;
            if row == 1 {
                e.set_control_property(id, "font_size", Value::Number(26.))?;
            }
            if row == 2 {
                e.set_control_property(id, "state", Value::Choice("disabled".into()))?;
                e.set_control_property(id, "align", Value::Choice("right".into()))?;
                if let NodeContent::Control { size, .. } = &mut e.document.pages[0]
                    .content
                    .nodes
                    .last_mut()
                    .unwrap()
                    .content
                {
                    size.width = Dimension::Fixed(200.);
                }
            }
        }
    }
    let scene = e.rendered_scene(e.page)?;
    fs::write(out.join("gallery.wireframe"), e.document.encode()?)?;
    fs::write(out.join("gallery.svg"), scene.svg())?;
    fs::write(out.join("gallery.png"), scene.png()?)?;
    let pages = pdf::plan(&e.document, &pdf::Options::default())?;
    fs::write(
        out.join("gallery.pdf"),
        pdf::encode("Label orientations", &pages)?,
    )?;
    // A compact fixture for native editing, separate from the export gallery.
    let mut e = Editor::default();
    e.document.title = "Label orientation study".into();
    e.add(Kind::Label, 50., 50.);
    let id = *e.selection.iter().next().unwrap();
    e.set_control_property(id, "text", Value::Text("Some text".into()))?;
    fs::write(
        out.join("Label orientation study.wireframe"),
        e.document.encode()?,
    )?;
    Ok(())
}
