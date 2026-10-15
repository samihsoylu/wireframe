// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Static media placeholders and narrow geometry, using the shared renderer.
use std::{fs, path::PathBuf};
use wireframe::{document::Project, model::Kind, workspace::Editor};
fn add(e: &mut Editor, kind: Kind, x: f32, y: f32, w: f32, h: f32) {
    e.add(kind, x, y);
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
            kind: Kind,
            width: f32,
            height: f32,
            sketch: bool,
        }
        let input: Vec<Input> = serde_json::from_slice(&fs::read(&args[2])?)?;
        let layouts: Vec<_> = input
            .into_iter()
            .map(|i| wireframe::media_controls::layout(i.kind, i.width, i.height, i.sketch))
            .collect();
        println!("{}", serde_json::to_string_pretty(&layouts)?);
        return Ok(());
    }
    let out = PathBuf::from(args.get(1).cloned().unwrap_or("media-controls".into()));
    fs::create_dir_all(&out)?;
    let mut e = Editor::new(Project::default());
    e.document.title = "Media placeholders".into();
    e.document.pages[0].name = "Media".into();
    e.document.settings.sketch = false;
    for (i, w) in [1., 10., 16., 32., 52., 53., 54., 55., 72., 100., 200., 300.]
        .into_iter()
        .enumerate()
    {
        add(
            &mut e,
            Kind::VolumeSlider,
            20. + (i % 3) as f32 * 320.,
            20. + (i / 3) as f32 * 40.,
            w,
            16.,
        );
    }
    for (i, (w, h)) in [(110., 36.), (220., 72.)].into_iter().enumerate() {
        add(&mut e, Kind::Playback, 20. + 160. * i as f32, 200., w, h);
    }
    for (x, w, h) in [
        (20., 177., 146.),
        (220., 300., 73.),
        (550., 88., 292.),
        (680., 10., 10.),
        (740., 1., 1.),
        (780., 100., 100.),
    ] {
        add(&mut e, Kind::Webcam, x, 270., w, h);
    }
    e.selection.clear();
    fs::write(out.join("Media.wireframe"), e.document.encode()?)?;
    for (name, sketch) in [("clean", false), ("sketch", true)] {
        e.document.settings.sketch = sketch;
        let scene = e.rendered_scene(e.page)?;
        fs::write(out.join(format!("{name}.png")), scene.png()?)?;
        fs::write(out.join(format!("{name}.svg")), scene.svg())?;
    }
    Ok(())
}
