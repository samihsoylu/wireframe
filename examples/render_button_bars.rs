// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use std::{fs, path::PathBuf};
use wireframe::{
    document::{Link, Value},
    model::Kind,
    workspace::Editor,
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| "evidence/button-bars".into());
    fs::create_dir_all(&out)?;
    if out.join("Button-bars.wireframe").exists() {
        return Err("Choose a fresh directory to preserve existing fixtures".into());
    }
    let mut e = Editor::default();
    e.document.title = "Button bars".into();
    e.document.settings.sketch = false;
    let first = e.page;
    e.document.pages[0].name = "Segments".into();
    e.add_page();
    let destination = e.page;
    e.document.pages[1].name = "Destination".into();
    e.add(Kind::ButtonBar, 50., 50.);
    let back = *e.selection.iter().next().unwrap();
    e.set_control_property(back, "text", Value::Text("Back, Done".into()))?;
    e.set_control_property(
        back,
        "row_links",
        Value::Links(vec![Some(Link::Back), None]),
    )?;
    e.switch_page(first);
    for i in 0..16 {
        e.add(
            Kind::ButtonBar,
            50. + (i % 4) as f32 * 230.,
            50. + (i / 4) as f32 * 120.,
        );
        let id = *e.selection.iter().next().unwrap();
        e.begin("Configure segments");
        let text = match i {
            3 => "One",
            4 => "A,,C",
            5 => "A,  B,C",
            6 => "First\nline, Second",
            7 => "",
            10 => "*Bold*, _Italic_, &Underlined&",
            11 => "[Website](https://example.org), Continue, Last",
            14 => "Same, Same, Other",
            15 => "Start, End",
            _ => "One, Two, Three",
        };
        e.set_control_property(id, "text", Value::Text(text.into()))?;
        if matches!(i, 1 | 2 | 8 | 9 | 11 | 12 | 13) {
            e.set_control_property(id, "selected_index", Value::Number(1.))?;
        }
        if i == 15 {
            e.set_control_property(id, "selected_index", Value::Number(-1.))?;
        }
        if matches!(i, 1 | 2 | 9 | 11 | 12 | 13 | 14) {
            e.set_control_property(
                id,
                "row_links",
                Value::Links(vec![
                    None,
                    Some(Link::Page(destination)),
                    Some(Link::Url("https://example.org/third".into())),
                ]),
            )?;
        }
        e.commit();
        if matches!(i, 1 | 2 | 8 | 9 | 12 | 13 | 15) {
            let (w, h) = match i {
                1 | 13 => (80., 21.),
                2 => (100., 21.),
                8 => (20., 50.),
                9 => (210., 50.),
                12 => (210., 21.),
                _ => (210., 60.),
            };
            e.begin("Resize bar");
            e.selected_mut(|c| {
                c.bounds.w = w;
                c.bounds.h = h;
            });
            e.commit();
        }
    }
    fs::write(out.join("Button-bars.wireframe"), e.document.encode()?)?;
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
