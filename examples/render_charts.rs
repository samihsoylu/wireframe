// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Chart gallery and numeric diagnostics for comparing layout implementations.
use std::{fs, path::PathBuf};
use wireframe::{
    charts::{self, Family},
    model::Kind,
    workspace::Editor,
};
#[derive(serde::Deserialize)]
struct Case {
    family: Family,
    width: f32,
    height: f32,
    sketch: bool,
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.get(1).is_some_and(|s| s == "--geometry") {
        let cases: Vec<Case> = serde_json::from_slice(&fs::read(&args[2])?)?;
        let out: Vec<_> = cases
            .into_iter()
            .map(|c| charts::layout(c.family, c.width, c.height, c.sketch))
            .collect();
        println!("{}", serde_json::to_string(&out)?);
        return Ok(());
    }
    let dir = PathBuf::from(args.get(1).cloned().unwrap_or("chart-gallery".into()));
    fs::create_dir_all(&dir)?;
    let mut e = Editor::default();
    e.document.title = "Charts".into();
    e.document.pages[0].name = "Chart layouts".into();
    for (col, kind) in [
        Kind::ColumnChart,
        Kind::BarChart,
        Kind::LineChart,
        Kind::PieChart,
    ]
    .into_iter()
    .enumerate()
    {
        for (row, size) in [
            None,
            Some((300., 80.)),
            Some((80., 260.)),
            Some((10., 10.)),
            Some((1., 1.)),
            Some((320., 320.)),
        ]
        .into_iter()
        .enumerate()
        {
            e.add(kind, 20. + 360. * col as f32, 20. + 330. * row as f32);
            if let Some((w, h)) = size {
                e.selected_mut(|c| {
                    c.bounds.w = w;
                    c.bounds.h = h;
                });
            }
        }
    }
    e.selection.clear();
    e.document.settings.sketch = false;
    fs::write(dir.join("Charts.wireframe"), e.document.encode()?)?;
    for (name, sketch) in [("clean", false), ("sketch", true)] {
        e.document.settings.sketch = sketch;
        let scene = e.rendered_scene(e.page)?;
        fs::write(dir.join(format!("{name}.png")), scene.png()?)?;
        fs::write(dir.join(format!("{name}.svg")), scene.svg())?;
    }
    Ok(())
}
