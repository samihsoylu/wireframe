// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Editable choice-row gallery and deterministic exports.
use std::{fs, path::PathBuf};
use wireframe::{
    document::{Link, Value},
    model::Kind,
    workspace::Editor,
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .ok_or("Supply a fresh output directory")?,
    );
    fs::create_dir_all(&out)?;
    if out.join("Choice-groups.wireframe").exists() {
        return Err("Choose a fresh directory to preserve existing fixtures".into());
    }
    let mut e = Editor::default();
    e.document.title = "Choice groups".into();
    e.document.settings.sketch = false;
    e.document.pages[0].name = "Choice groups".into();
    let first = e.page;
    e.add_page();
    let destination = e.page;
    e.document.pages[1].name = "Destination".into();
    e.add(Kind::Button, 50., 50.);
    let back = *e.selection.iter().next().unwrap();
    e.set_control_property(back, "text", Value::Text("Back to choices".into()))?;
    e.set_control_property(back, "link", Value::Link(Link::Back))?;
    e.switch_page(first);
    for i in 0..12 {
        let kind = if matches!(i, 1 | 4 | 7 | 9 | 11) {
            Kind::RadioGroup
        } else {
            Kind::CheckboxGroup
        };
        e.add(
            kind,
            50. + (i % 3) as f32 * 320.,
            50. + (i / 3) as f32 * 270.,
        );
        let id = *e.selection.iter().next().unwrap();
        let text = match i {
            0 => wireframe::choice_group::CHECKBOX,
            1 => wireframe::choice_group::RADIO,
            2 | 3 => "[ ] First\n[x] Second\n[-] Third",
            4 => "( ) First\n(o) Second\n(-) Third",
            5 => "Plain\n\n[x] Last",
            6 => "[x] Bold *word*\n-[ ] Disabled-\n(o) Wrong kind",
            7 => "(x) Choice\n-(-) Mixed-\n[x] Wrong kind",
            8 => "[ ]\n[x]\n[-]",
            9 => "(o) First\\rsecond\n( ) Last",
            10 => "{color:#ff0000}[x] Red{color}\n{color:#0000ff}[ ]{color} Label",
            _ => "( ) First\n-(o) Disabled link-\nPlain",
        };
        e.set_control_property(id, "text", Value::Text(text.into()))?;
        if matches!(i, 8 | 9) {
            e.set_control_property(
                id,
                "font_size",
                Value::Number(if i == 8 { 8. } else { 24. }),
            )?;
        }
        if i == 11 {
            e.set_control_property(
                id,
                "row_links",
                Value::Links(vec![Some(Link::Page(destination)); 3]),
            )?;
        }
        if matches!(i, 3 | 4) {
            e.begin("Resize choices");
            e.selected_mut(|c| {
                c.bounds.w = 65.;
                c.bounds.h = 40.;
            });
            e.commit();
        }
    }
    fs::write(out.join("Choice-groups.wireframe"), e.document.encode()?)?;
    for c in e.current().controls {
        println!(
            "{} {} {} {} {}",
            c.id,
            c.kind.name(),
            c.bounds.x,
            c.bounds.w,
            c.bounds.h
        );
    }
    for (name, sketch) in [("clean", false), ("sketch", true)] {
        e.document.settings.sketch = sketch;
        let scene = e.rendered_scene(first)?;
        fs::write(out.join(format!("{name}.png")), scene.png()?)?;
        fs::write(out.join(format!("{name}.svg")), scene.svg())?;
    }
    Ok(())
}
