// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Save a reproducible styled-text and grid specimen into a supplied directory.
use wireframe::{catalog, document::*, storage::ProjectStore, workspace::Editor};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = std::path::PathBuf::from(
        std::env::args()
            .nth(1)
            .ok_or("Supply an output directory")?,
    );
    std::fs::create_dir_all(&out)?;
    let mut p = Project {
        title: "Text and tables".into(),
        next_id: 5,
        ..Project::default()
    };
    p.pages[0].name = "Formatting".into();
    let mut paragraph = catalog::get("paragraph")
        .unwrap()
        .create(2, Point::new(40., 40.));
    paragraph.content = NodeContent::Control {
        kind: "paragraph".into(),
        size: Size::fixed(450., 180.),
    };
    paragraph.properties.insert("text".into(),Value::Text("*Planning* with _clarity_\n&Underlined& and ~removed~; -disabled-\n{color:#cc2200}A red note{color} and {size:24}larger type{size}\n[Continue](page:4) or [read more](https://example.org)\n* First idea\n* Second idea".into()));
    let mut table = catalog::get("table")
        .unwrap()
        .create(3, Point::new(40., 260.));
    table.properties.insert(
        "text".into(),
        Value::Text(
            "Name ^, Score ^v, Ready\nAlice\\, Jr, *42*, [x]\nBob, , [ ]\n{60L,20R,20C}".into(),
        ),
    );
    p.pages[0].content.nodes = vec![paragraph, table];
    p.pages.push(Page {
        id: 4,
        name: "Linked page".into(),
        notes: String::new(),
        parent: None,
        content: Content::default(),
        alternates: vec![],
    });
    p.validate()?;
    ProjectStore.save(&out.join("Text-and-tables.wireframe"), &p)?;
    let e = Editor::new(p);
    let scene = e.rendered_scene(1)?;
    std::fs::write(out.join("Text-and-tables.svg"), scene.svg())?;
    std::fs::write(out.join("Text-and-tables.png"), scene.png()?)?;
    for node in e.layout_page(1)?.nodes {
        println!(
            "{:?}: {} x {}",
            node.kind, node.bounds.width, node.bounds.height
        );
    }
    println!(
        "{} draw items; {} link regions",
        scene.items.len(),
        scene.links.len()
    );
    Ok(())
}
