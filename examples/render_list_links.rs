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
        .unwrap_or_else(|| PathBuf::from("evidence/list-links"));
    fs::create_dir_all(&out)?;
    let mut e = Editor::default();
    e.document.title = "List navigation".into();
    e.document.settings.sketch = false;
    let first = e.page;
    e.document.pages[0].name = "Start".into();
    e.add_page();
    let second = e.page;
    e.document.pages[1].name = "Destination".into();
    for (page, text, links) in [
        (
            first,
            "Header\nContinue\nWebsite",
            vec![
                Some(Link::Page(second)),
                Some(Link::Page(second)),
                Some(Link::Url("https://example.test".into())),
            ],
        ),
        (second, "Destination\nReturn", vec![None, Some(Link::Back)]),
    ] {
        e.switch_page(page);
        e.add(Kind::List, 50., 50.);
        let id = *e.selection.iter().next().unwrap();
        e.begin("Navigation");
        e.set_control_property(id, "text", Value::Text(text.into()))?;
        e.set_control_property(id, "header", Value::Bool(true))?;
        e.set_control_property(id, "row_links", Value::Links(links))?;
        e.commit();
        let scene = e.rendered_scene(page)?;
        fs::write(out.join(format!("page-{page}.svg")), scene.svg())?;
        fs::write(out.join(format!("page-{page}.png")), scene.png()?)?;
    }
    e.switch_page(first);
    fs::write(out.join("List-links.wireframe"), e.document.encode()?)?;
    Ok(())
}
