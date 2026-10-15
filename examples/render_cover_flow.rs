// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Image carousel geometry diagnostics and a local gallery.
use std::{fs, io::Cursor, path::PathBuf};
use wireframe::{
    document::{Project, Value},
    images::Prepared,
    model::Kind,
    workspace::{Editor, ImportChoice},
};
#[derive(Clone, serde::Deserialize)]
struct Case {
    width: f32,
    height: f32,
    scroll: bool,
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.get(1).is_some_and(|s| s == "--geometry") {
        let cases: Vec<Case> = serde_json::from_slice(&fs::read(&args[2])?)?;
        println!(
            "{}",
            serde_json::to_string_pretty(
                &cases
                    .into_iter()
                    .map(|c| wireframe::cover_flow::layout(c.width, c.height, c.scroll))
                    .collect::<Vec<_>>()
            )?
        );
        return Ok(());
    }
    let out = PathBuf::from(args.get(1).cloned().unwrap_or("carousel-gallery".into()));
    fs::create_dir_all(&out)?;
    let prepared = if let Some(path) = args.get(2) {
        Prepared::read(std::path::Path::new(path))?
    } else {
        let im = image::RgbaImage::from_fn(160, 100, |x, y| {
            image::Rgba(match (x < 80, y < 50) {
                (true, true) => [240, 40, 30, 255],
                (false, true) => [30, 210, 70, 255],
                (true, false) => [30, 70, 240, 255],
                _ => [250, 190, 20, 255],
            })
        });
        let mut bytes = Cursor::new(Vec::new());
        im.write_to(&mut bytes, image::ImageFormat::Png)?;
        Prepared::from_bytes("Quadrants.png", &bytes.into_inner())?
    };
    let mut e = Editor::new(Project::default());
    e.document.title = "Image carousels".into();
    e.document.pages[0].name = "Carousel".into();
    e.document.settings.sketch = false;
    let mut cases = vec![];
    for filter in [false, true] {
        for (col, rotation) in [0., 90., 180., 270.].into_iter().enumerate() {
            cases.push((
                col,
                usize::from(filter),
                493.,
                198.,
                true,
                rotation,
                filter,
                false,
                0.,
            ));
        }
    }
    for (col, value) in [0., 0.5, 1.].into_iter().enumerate() {
        cases.push((col, 2, 493., 198., true, 0., false, true, value));
    }
    cases.push((3, 2, 493., 198., false, 0., false, true, 0.5));
    for (col, w) in [60., 61.].into_iter().enumerate() {
        cases.push((col, 3, w, 50., true, 0., false, true, 0.5));
    }
    for (col, h) in [16., 17.].into_iter().enumerate() {
        cases.push((col + 2, 3, 493., h, true, 0., false, true, 0.5));
    }
    for (col, w, h, source) in [
        (0, 493., 198., false),
        (1, 493., 198., false),
        (2, 493., 100., true),
        (3, 246., 396., true),
    ] {
        cases.push((col, 4, w, h, source, 0., false, false, 0.));
    }
    for (col, w, h) in [(0, 1., 198.), (1, 493., 1.), (2, 800., 100.)] {
        cases.push((col, 6, w, h, true, 0., false, false, 0.));
    }
    for (col, row, w, h, source, rotation, filter, scroll, value) in cases {
        e.add(
            Kind::CoverFlow,
            20. + col as f32 * 520.,
            20. + row as f32 * 260.,
        );
        let id = *e.selection.iter().next().unwrap();
        e.selected_mut(|c| {
            c.bounds.w = w;
            c.bounds.h = h;
        });
        if source {
            e.import_image(id, prepared.clone(), ImportChoice::UseExisting)?;
        }
        for (key, value) in [
            ("rotation", Value::Number(rotation)),
            ("sketch_filter", Value::Bool(filter)),
            ("vertical_scrollbar", Value::Bool(scroll)),
            ("value", Value::Number(value)),
        ] {
            e.set_control_property(id, key, value)?;
        }
    }
    e.selection.clear();
    fs::write(out.join("Carousels.wireframe"), e.document.encode()?)?;
    for (name, sketch) in [("clean", false), ("sketch", true)] {
        e.document.settings.sketch = sketch;
        let scene = e.rendered_scene(e.page)?;
        fs::write(out.join(format!("{name}.png")), scene.png()?)?;
        fs::write(out.join(format!("{name}.svg")), scene.svg())?;
    }
    Ok(())
}
