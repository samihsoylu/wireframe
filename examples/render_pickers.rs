// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! A date/time dialog gallery with optional HH:MM clocks for repeatable exports.
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU16, Ordering},
};
use wireframe::{
    document::{Link, Project, Value},
    model::Kind,
    tablet::Clock,
    workspace::Editor,
};
static TIME: AtomicU16 = AtomicU16::new(u16::MAX);
fn clock() -> Clock {
    let n = TIME.load(Ordering::SeqCst);
    if n == u16::MAX {
        Clock::now()
    } else {
        Clock::new((n / 60) as u8, (n % 60) as u8).unwrap()
    }
}
fn set_clock(s: Option<&String>) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(s) = s {
        let (h, m) = s.split_once(':').ok_or("Clock must be HH:MM")?;
        let h: u8 = h.parse()?;
        let m: u8 = m.parse()?;
        Clock::new(h, m).ok_or("Invalid clock")?;
        TIME.store(u16::from(h) * 60 + u16::from(m), Ordering::SeqCst);
    }
    Ok(())
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    let out = PathBuf::from(args.get(1).map(String::as_str).unwrap_or("picker-dialogs"));
    fs::create_dir_all(&out)?;
    let mut e = Editor::with_clock(Project::default(), clock);
    e.document.title = "Date and time dialogs".into();
    e.document.pages[0].name = "Pickers".into();
    e.document.settings.sketch = false;
    let home = e.page;
    e.add_page();
    e.document.pages.last_mut().unwrap().name = "Cancel destination".into();
    let cancel = e.page;
    e.add(Kind::Heading, 40., 40.);
    e.selected_mut(|c| c.text = "Cancelled".into());
    e.add_page();
    e.document.pages.last_mut().unwrap().name = "OK destination".into();
    let ok = e.page;
    e.add(Kind::Heading, 40., 40.);
    e.selected_mut(|c| c.text = "Confirmed".into());
    e.switch_page(home);
    for (row, (w, h, y)) in [(320., 473., 20.), (570., 303., 520.), (60., 200., 860.)]
        .into_iter()
        .enumerate()
    {
        for (col, (kind, year)) in [
            (Kind::DatePicker, false),
            (Kind::DatePicker, true),
            (Kind::TimePicker, false),
        ]
        .into_iter()
        .enumerate()
        {
            e.add(kind, 20. + 620. * col as f32, y);
            let id = *e.selection.iter().next().unwrap();
            if year {
                e.set_control_property(id, "date_mode", Value::Choice("year".into()))?;
            }
            e.selected_mut(|c| {
                c.bounds.w = w;
                c.bounds.h = h;
            });
            e.set_control_property(
                id,
                "row_links",
                Value::Links(vec![Some(Link::Page(cancel)), Some(Link::Page(ok))]),
            )?;
            if row == 2 && kind == Kind::TimePicker {
                e.set_control_property(
                    id,
                    "row_links",
                    Value::Links(vec![Some(Link::Page(cancel)), Some(Link::Back)]),
                )?;
            }
        }
    }
    fs::write(out.join("Pickers.wireframe"), e.document.encode()?)?;
    for (name, sketch, clock_index) in [("clean", false, 2), ("sketch", true, 3)] {
        set_clock(args.get(clock_index))?;
        let mut render = Editor::with_clock(e.document.clone(), clock);
        render.document.settings.sketch = sketch;
        let scene = render.rendered_scene(home)?;
        fs::write(out.join(format!("{name}.png")), scene.png()?)?;
        fs::write(out.join(format!("{name}.svg")), scene.svg())?;
    }
    Ok(())
}
