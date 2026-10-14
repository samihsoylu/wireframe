// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use std::{fs, io::Cursor, path::PathBuf};
use wireframe::{
    document::{IconSource, Value},
    images::Prepared,
    model::Kind,
    workspace::{Editor, ImageTarget, ImportChoice},
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| "evidence/optional-icons".into());
    fs::create_dir_all(&out)?;
    let colors = [
        [220, 40, 40, 255],
        [30, 180, 70, 255],
        [40, 80, 220, 255],
        [0; 4],
    ];
    let image = image::RgbaImage::from_fn(120, 80, |x, y| {
        image::Rgba(colors[(x / 60 + y / 40 * 2) as usize])
    });
    let mut bytes = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(image).write_to(&mut bytes, image::ImageFormat::Png)?;
    let bytes = bytes.into_inner();
    fs::write(out.join("Quadrants.png"), &bytes)?;
    let mut e = Editor::default();
    e.document.title = "Optional icons".into();
    e.document.settings.sketch = false;
    for i in 0..16 {
        let dropdown = (8..14).contains(&i);
        e.add(
            if dropdown {
                Kind::Dropdown
            } else {
                Kind::Button
            },
            50. + (i % 4) as f32 * 220.,
            50. + (i / 4) as f32 * 120.,
        );
        let id = *e.selection.iter().next().unwrap();
        e.begin("Configure example");
        let text = if i == 4 {
            ""
        } else if dropdown {
            if i < 12 {
                "Choose\nAlpha\nBravo"
            } else {
                "Choose"
            }
        } else {
            "Go"
        };
        e.set_control_property(id, "text", Value::Text(text.into()))?;
        if i < 14 {
            e.set_control_property(
                id,
                "icon",
                Value::Icon(IconSource::Catalog("arrow-right-solid".into())),
            )?;
            let size = match i {
                0 => "16",
                2 | 13 => "64",
                7 => "48",
                _ => "32",
            };
            e.set_control_property(id, "icon_size", Value::Choice(size.into()))?;
        }
        if [1, 4, 5, 14, 15].contains(&i) {
            e.set_control_property(id, "menu_icon", Value::Bool(true))?;
        }
        let state = match i {
            2 => "disabled",
            3 => "selected",
            8 | 12 | 13 => "closed",
            9 => "focused_closed",
            10 => "disabled_closed",
            _ => "normal",
        };
        e.set_control_property(id, "state", Value::Choice(state.into()))?;
        if [3, 7, 13].contains(&i) {
            e.set_control_property(id, "icon_rotation", Value::Choice("90".into()))?;
        }
        if i == 6 {
            e.set_control_property(id, "align", Value::Choice("right".into()))?;
        }
        if i >= 14 {
            e.set_control_property(
                id,
                "font_size",
                Value::Number(if i == 14 { 28. } else { 60. }),
            )?;
        }
        e.commit();
        if [7, 13].contains(&i) {
            e.import_image_at(
                ImageTarget::Icon(id),
                Prepared::from_bytes("Quadrants.png", &bytes)?,
                ImportChoice::UseExisting,
            )?;
        }
        if [5, 6, 12].contains(&i) {
            e.begin("Fixed example");
            e.selected_mut(|c| {
                c.bounds.w = match i {
                    5 => 50.,
                    6 => 160.,
                    _ => 35.,
                };
                if i == 5 {
                    c.bounds.h = 32.;
                }
                if i == 6 {
                    c.bounds.h = 50.;
                }
            });
            e.commit();
        }
    }
    fs::write(out.join("Optional-icons.wireframe"), e.document.encode()?)?;
    fs::write(out.join("clean.png"), e.rendered_scene(e.page)?.png()?)?;
    fs::write(out.join("clean.svg"), e.rendered_scene(e.page)?.svg())?;
    e.document.settings.sketch = true;
    fs::write(out.join("sketch.png"), e.rendered_scene(e.page)?.png()?)?;
    Ok(())
}
