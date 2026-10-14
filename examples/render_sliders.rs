// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Original slider gallery spanning shapes, levels, disabled and narrow controls.
use std::{fs, path::PathBuf};
use wireframe::{
    document::{Project, Value},
    model::Kind,
    workspace::Editor,
};
fn add(
    e: &mut Editor,
    kind: Kind,
    position: (f32, f32),
    length: f32,
    thumb: &str,
    value: f64,
    disabled: bool,
) -> Result<(), String> {
    e.add(kind, position.0, position.1);
    let id = *e.selection.iter().next().unwrap();
    e.set_control_property(id, "slider_thumb", Value::Choice(thumb.into()))?;
    e.set_control_property(id, "value", Value::Number(value))?;
    e.set_control_property(id, "fill", Value::Color([51, 102, 153, 255]))?;
    if disabled {
        e.set_control_property(id, "state", Value::Choice("disabled".into()))?;
    }
    e.selected_mut(|c| {
        if kind == Kind::HorizontalSlider {
            c.bounds.w = length
        } else {
            c.bounds.h = length
        }
    });
    Ok(())
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = PathBuf::from(std::env::args().nth(1).unwrap_or("sliders".into()));
    fs::create_dir_all(&out)?;
    let mut e = Editor::new(Project::default());
    e.document.title = "Sliders and levels".into();
    e.document.pages[0].name = "Sliders".into();
    e.document.settings.sketch = false;
    for (row, thumb) in ["pointy", "rectangle", "circle"].into_iter().enumerate() {
        for (col, value) in [0., 0.05, 0.5, 1.].into_iter().enumerate() {
            add(
                &mut e,
                Kind::HorizontalSlider,
                (20. + 180. * col as f32, 20. + 55. * row as f32),
                120.,
                thumb,
                value,
                false,
            )?;
        }
        for (col, value) in [0., 0.5, 1.].into_iter().enumerate() {
            add(
                &mut e,
                Kind::VerticalSlider,
                (30. + 45. * col as f32 + 180. * row as f32, 250.),
                120.,
                thumb,
                value,
                false,
            )?;
        }
        add(
            &mut e,
            Kind::HorizontalSlider,
            (20. + 180. * row as f32, 205.),
            120.,
            thumb,
            0.5,
            true,
        )?;
    }
    for (col, length) in [1., 6., 12., 14., 33.].into_iter().enumerate() {
        add(
            &mut e,
            Kind::HorizontalSlider,
            (20. + 120. * col as f32, 420.),
            length,
            "circle",
            1.,
            false,
        )?;
    }
    e.selection.clear();
    fs::write(out.join("Sliders.wireframe"), e.document.encode()?)?;
    for (name, sketch) in [("clean", false), ("sketch", true)] {
        e.document.settings.sketch = sketch;
        let scene = e.rendered_scene(e.page)?;
        fs::write(out.join(format!("{name}.png")), scene.png()?)?;
        fs::write(out.join(format!("{name}.svg")), scene.svg())?;
    }
    Ok(())
}
