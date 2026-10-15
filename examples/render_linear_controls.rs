// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Scroll indicators and dividers in both styles, including short saved geometry.
use std::{fs, path::PathBuf};
use wireframe::{
    document::{Project, Value},
    model::Kind,
    workspace::Editor,
};
fn add(
    e: &mut Editor,
    kind: Kind,
    x: f32,
    y: f32,
    length: f32,
    value: Option<f64>,
) -> Result<(), String> {
    e.add(kind, x, y);
    let id = *e.selection.iter().next().unwrap();
    if let Some(value) = value {
        e.set_control_property(id, "value", Value::Number(value))?;
    }
    let axis = wireframe::linear_controls::Axis::for_kind(kind).unwrap();
    e.selected_mut(|c| {
        if axis.vertical {
            c.bounds.h = length
        } else {
            c.bounds.w = length
        }
    });
    Ok(())
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = PathBuf::from(
        std::env::args()
            .nth(1)
            .unwrap_or("scroll-indicators".into()),
    );
    fs::create_dir_all(&out)?;
    let mut e = Editor::new(Project::default());
    e.document.title = "Scroll indicators and dividers".into();
    e.document.pages[0].name = "Indicators".into();
    e.document.settings.sketch = false;
    for vertical in [false, true] {
        let kind = if vertical {
            Kind::VerticalScrollbar
        } else {
            Kind::HorizontalScrollbar
        };
        for (col, value) in [0., 0.05, 0.5, 0.95, 1.].into_iter().enumerate() {
            add(
                &mut e,
                kind,
                20. + 140. * col as f32,
                if vertical { 200. } else { 20. },
                100.,
                Some(value),
            )?;
        }
        for (col, length) in [1., 4., 24., 44., 45., 46., 48.].into_iter().enumerate() {
            add(
                &mut e,
                kind,
                20. + 100. * col as f32,
                if vertical { 350. } else { 100. },
                length,
                Some(0.5),
            )?;
        }
    }
    for vertical in [false, true] {
        let kind = if vertical {
            Kind::VerticalSplitter
        } else {
            Kind::HorizontalSplitter
        };
        for (col, length) in [1., 8., 16., 17., 100.].into_iter().enumerate() {
            add(
                &mut e,
                kind,
                20. + 140. * col as f32,
                if vertical { 520. } else { 450. },
                length,
                None,
            )?;
        }
    }
    e.selection.clear();
    fs::write(out.join("Indicators.wireframe"), e.document.encode()?)?;
    for (name, sketch) in [("clean", false), ("sketch", true)] {
        e.document.settings.sketch = sketch;
        let scene = e.rendered_scene(e.page)?;
        fs::write(out.join(format!("{name}.png")), scene.png()?)?;
        fs::write(out.join(format!("{name}.svg")), scene.svg())?;
    }
    Ok(())
}
