// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Mobile menus and fixed picker previews with editable content examples.
use std::{fs, path::PathBuf};
use wireframe::{
    document::{Project, Value},
    model::Kind,
    workspace::Editor,
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .unwrap_or_else(|| "mobile-rows".into()),
    );
    fs::create_dir_all(&out)?;
    let mut e = Editor::new(Project::default());
    e.document.title = "Mobile menus and pickers".into();
    e.document.pages[0].name = "Mobile rows".into();
    e.document.settings.sketch = false;
    for (kind, x, y, width, text) in [
        (
            Kind::MobileMenu,
            20.,
            20.,
            215.,
            wireframe::mobile_menu::DEFAULT,
        ),
        (
            Kind::MobileMenu,
            270.,
            20.,
            100.,
            "+ Add, >\nv Checked, (>)\n* Notice, =\nReady, ON\nPaused, OFF",
        ),
        (
            Kind::MobileMenu,
            410.,
            20.,
            215.,
            "Profile, Work\n__ Picture area\n\nPreferences, >\nPrivacy, >",
        ),
        (
            Kind::MobileMenu,
            670.,
            20.,
            215.,
            "Heading, Details\nSoft\\rbreak, two\\rlines\nOne\\, two, Three\\, four\n*Bold* _italic_, *caption*",
        ),
        (
            Kind::MobileMenu,
            410.,
            250.,
            215.,
            "Borderless section\nOne row\nAnother row",
        ),
        (
            Kind::MobilePicker,
            20.,
            460.,
            214.,
            wireframe::mobile_picker::DEFAULT,
        ),
        (
            Kind::MobilePicker,
            300.,
            460.,
            214.,
            "Mon, 1\nTue, 2\nWed, 3\nThu, 4\nFri, 5\nSat, 6\nSun, 7\nStored extra row",
        ),
        (
            Kind::MobilePicker,
            580.,
            460.,
            214.,
            "\n\nMorning\nAfternoon\nEvening\n\n",
        ),
    ] {
        e.add(kind, x, y);
        let id = *e.selection.iter().next().unwrap();
        e.set_control_property(id, "text", Value::Text(text.into()))?;
        e.selected_mut(|c| c.bounds.w = width);
        if kind == Kind::MobileMenu && y == 250. {
            e.set_control_property(id, "border", Value::Choice("none".into()))?;
        }
    }
    fs::write(out.join("Mobile-rows.wireframe"), e.document.encode()?)?;
    for (name, sketch) in [("clean", false), ("sketch", true)] {
        e.document.settings.sketch = sketch;
        let scene = e.rendered_scene(e.page)?;
        fs::write(out.join(format!("{name}.png")), scene.png()?)?;
        fs::write(out.join(format!("{name}.svg")), scene.svg())?;
    }
    Ok(())
}
