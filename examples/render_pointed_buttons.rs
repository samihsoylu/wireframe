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
        .unwrap_or_else(|| "evidence/pointed-buttons".into());
    fs::create_dir_all(&out)?;
    if out.join("Pointed-buttons.wireframe").exists() {
        return Err("Choose a fresh directory to preserve existing fixtures".into());
    }
    let colors = [
        [220, 40, 40, 255],
        [30, 180, 70, 255],
        [40, 80, 220, 255],
        [0; 4],
    ];
    let im = image::RgbaImage::from_fn(120, 80, |x, y| {
        image::Rgba(colors[(x / 60 + y / 40 * 2) as usize])
    });
    let mut data = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(im).write_to(&mut data, image::ImageFormat::Png)?;
    let bytes = data.into_inner();
    fs::write(out.join("Quadrants.png"), &bytes)?;
    let mut e = Editor::default();
    e.document.title = "Pointed buttons".into();
    e.document.settings.sketch = false;
    for i in 0..16 {
        e.add(
            Kind::PointedButton,
            50. + (i % 4) as f32 * 200.,
            50. + (i / 4) as f32 * 110.,
        );
        let id = *e.selection.iter().next().unwrap();
        e.begin("Configure pointed button");
        if [1, 4, 10].contains(&i) {
            e.set_control_property(id, "direction", Value::Choice("right".into()))?;
        }
        if [2, 11, 14].contains(&i) {
            e.set_control_property(id, "direction", Value::Choice("center".into()))?;
        }
        if [3, 4, 5, 9, 10, 11, 14].contains(&i) {
            e.set_control_property(id, "border", Value::Choice("solid".into()))?;
        }
        if i >= 5 {
            e.set_control_property(
                id,
                "text",
                Value::Text(if i == 15 { "" } else { "Go" }.into()),
            )?;
        }
        if [5, 6, 7, 8, 12, 13, 15].contains(&i) {
            e.set_control_property(
                id,
                "icon",
                Value::Icon(IconSource::Catalog("arrow-right-solid".into())),
            )?;
            e.set_control_property(
                id,
                "icon_size",
                Value::Choice(
                    if [7, 8].contains(&i) {
                        "48"
                    } else if [6, 12, 13].contains(&i) {
                        "32"
                    } else {
                        "24"
                    }
                    .into(),
                ),
            )?;
        }
        if [5, 9, 10, 11].contains(&i) {
            e.set_control_property(id, "state", Value::Choice("disabled".into()))?;
        }
        if [6, 8, 13].contains(&i) {
            e.set_control_property(id, "menu_icon", Value::Bool(true))?;
        }
        if [12, 13].contains(&i) {
            e.set_control_property(id, "icon_rotation", Value::Choice("90".into()))?;
            e.set_control_property(id, "fill", Value::Color([0, 0, 0, 255]))?;
        }
        if i == 14 {
            e.set_control_property(id, "font_size", Value::Number(28.))?;
        }
        e.commit();
        if [7, 8].contains(&i) {
            e.begin("Narrow pointed button");
            e.selected_mut(|c| c.bounds.w = 35.);
            e.commit();
        }
        if i == 12 {
            e.import_image_at(
                ImageTarget::Icon(id),
                Prepared::from_bytes("Quadrants.png", &bytes)?,
                ImportChoice::KeepBoth,
            )?;
        }
    }
    fs::write(out.join("Pointed-buttons.wireframe"), e.document.encode()?)?;
    let s = e.rendered_scene(e.page)?;
    fs::write(out.join("clean.png"), s.png()?)?;
    fs::write(out.join("clean.svg"), s.svg())?;
    e.document.settings.sketch = true;
    let s = e.rendered_scene(e.page)?;
    fs::write(out.join("sketch.png"), s.png()?)?;
    fs::write(out.join("sketch.svg"), s.svg())?;
    println!("{}", out.display());
    Ok(())
}
