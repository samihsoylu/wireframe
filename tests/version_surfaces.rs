// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{document::*, export_plan, workspace::Editor};

fn existing_fixture() -> Project {
    let mut p = Project::default();
    p.pages[0].notes = "Official notes".into();
    // Deserialize to exercise the existing serialized alternate boundary.
    p.pages[0].alternates.push(
        serde_json::from_value(serde_json::json!({
            "id": 8, "name": "Compact", "notes": "Compact notes",
            "content": {"nodes": []}
        }))
        .unwrap(),
    );
    p.next_id = 9;
    p
}

#[test]
fn alternate_is_a_renderable_editor_surface() {
    let e = Editor::new(existing_fixture());
    assert!(
        e.layout_page(8).is_ok(),
        "an existing alternate must resolve as a surface"
    );
}

#[test]
fn alternate_export_uses_its_own_notes() {
    let p = existing_fixture();
    let entries = export_plan::entries(&p);
    assert_eq!(entries[1].page(&p).unwrap().notes, "Compact notes");
}
