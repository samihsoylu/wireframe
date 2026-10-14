// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! A grouped form row for exercising component workflows in the desktop app.
use wireframe::{catalog, document::*, storage::ProjectStore, workspace::Editor};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = std::path::PathBuf::from(std::env::args().nth(1).ok_or("Supply output directory")?);
    std::fs::create_dir_all(&out)?;
    let mut p = Project {
        title: "Component workspace".into(),
        ..Project::default()
    };
    let group = p.allocate_id()?;
    let a = p.allocate_id()?;
    let b = p.allocate_id()?;
    let mut label = catalog::get("button")
        .unwrap()
        .create(a, Point::new(0., 0.));
    label
        .properties
        .insert("text".into(), Value::Text("Save".into()));
    let mut input = catalog::get("text_input")
        .unwrap()
        .create(b, Point::new(85., 0.));
    input
        .properties
        .insert("text".into(), Value::Text("Name".into()));
    input.content = NodeContent::Control {
        kind: "text_input".into(),
        size: Size {
            width: Dimension::Fixed(180.),
            height: Dimension::Auto,
        },
    };
    p.pages[0]
        .content
        .nodes
        .push(Node::group(group, Point::new(50., 50.), vec![label, input]));
    ProjectStore.save(&out.join("Component-workspace.wireframe"), &p)?;
    let mut e = Editor::new(p);
    e.select(group, false);
    let definition = e.create_component("Form row".into())?;
    e.insert_component(definition, 50., 140.)?;
    ProjectStore.save(&out.join("Component-pair.wireframe"), &e.document)?;
    let s = e.rendered_scene(1)?;
    std::fs::write(out.join("Component-pair.png"), s.png()?)?;
    std::fs::write(out.join("Component-pair.svg"), s.svg())?;
    Ok(())
}
