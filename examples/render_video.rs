// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Video layout diagnostics and a neutral sizing gallery.
use std::{fs, path::PathBuf};
use wireframe::{document::Project, model::Kind, workspace::Editor};
fn add(e: &mut Editor, x: f32, y: f32, w: f32, h: f32) {
    e.add(Kind::Video, x, y);
    e.selected_mut(|c| {
        c.bounds.w = w;
        c.bounds.h = h;
    });
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.get(1).is_some_and(|s| s == "--geometry") {
        #[derive(serde::Deserialize)]
        struct Input {
            width: f32,
            height: f32,
            sketch: bool,
        }
        let input: Vec<Input> = serde_json::from_slice(&fs::read(&args[2])?)?;
        let layouts: Vec<_> = input
            .into_iter()
            .map(|i| wireframe::video::layout(i.width, i.height, i.sketch))
            .collect();
        println!("{}", serde_json::to_string_pretty(&layouts)?);
        return Ok(());
    }
    let out = PathBuf::from(args.get(1).cloned().unwrap_or("video-gallery".into()));
    fs::create_dir_all(&out)?;
    let mut e = Editor::new(Project::default());
    e.document.title = "Video layouts".into();
    e.document.pages[0].name = "Video".into();
    e.document.settings.sketch = false;
    for (x, w, h) in [
        (20., 300., 200.),
        (380., 600., 400.),
        (1020., 150., 400.),
        (1220., 10., 10.),
    ] {
        add(&mut e, x, 20., w, h);
    }
    for (i, w) in [1., 10., 100., 207., 208., 209., 219., 220., 221., 300.]
        .into_iter()
        .enumerate()
    {
        add(
            &mut e,
            20. + (i % 5) as f32 * 330.,
            460. + (i / 5) as f32 * 100.,
            w,
            50.,
        );
    }
    for (i, h) in [1., 10., 39., 40., 41., 49., 50., 51.]
        .into_iter()
        .enumerate()
    {
        add(
            &mut e,
            20. + (i % 4) as f32 * 350.,
            700. + (i / 4) as f32 * 150.,
            300.,
            h,
        );
    }
    e.selection.clear();
    fs::write(out.join("Video.wireframe"), e.document.encode()?)?;
    for (name, sketch) in [("clean", false), ("sketch", true)] {
        e.document.settings.sketch = sketch;
        let scene = e.rendered_scene(e.page)?;
        fs::write(out.join(format!("{name}.png")), scene.png()?)?;
        fs::write(out.join(format!("{name}.svg")), scene.svg())?;
    }
    Ok(())
}
