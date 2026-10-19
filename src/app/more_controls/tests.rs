// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
fn frame(
    ctx: &egui::Context,
    app: &mut WireframeApp,
    events: Vec<egui::Event>,
) -> std::collections::HashMap<String, Rect> {
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1100., 800.))),
            events,
            ..Default::default()
        },
        |ui| {
            app.shortcuts(ui.ctx());
            app.more_controls_dialog(ui.ctx());
        },
    );
    let nodes = output
        .platform_output
        .accesskit_update
        .as_ref()
        .unwrap()
        .nodes
        .iter()
        .filter_map(|(_, n)| {
            let b = n.bounds()?;
            Some((
                n.label()?.to_owned(),
                Rect::from_min_max(
                    egui::pos2(b.x0 as f32, b.y0 as f32),
                    egui::pos2(b.x1 as f32, b.y1 as f32),
                ),
            ))
        })
        .collect();
    output.textures_delta.clear();
    nodes
}
fn click(ctx: &egui::Context, app: &mut WireframeApp, label: &str) {
    frame(ctx, app, vec![]);
    let nodes = frame(ctx, app, vec![]);
    let pos = nodes[label].center();
    frame(
        ctx,
        app,
        vec![
            egui::Event::PointerMoved(pos),
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::NONE,
            },
        ],
    );
    frame(
        ctx,
        app,
        vec![egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        }],
    );
}
fn setup(index: usize) -> (egui::Context, WireframeApp) {
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let mut app = WireframeApp::workspace(&ctx, None);
    app.open_more_controls();
    let browser = app.more_controls.as_mut().unwrap();
    browser.preview =
        Some(Preview::new(index, browser.catalog.entries[index].load().unwrap()).unwrap());
    (ctx, app)
}
#[test]
fn browser_import_buttons_apply_once_and_undo_redo_exactly() {
    let (ctx, mut app) = setup(0);
    let before = app.editor.document.clone();
    click(&ctx, &mut app, "Import into Project");
    assert!(
        app.more_controls
            .as_ref()
            .unwrap()
            .review
            .as_ref()
            .unwrap()
            .prepared
            .is_some()
    );
    assert_eq!(app.editor.document, before);
    click(&ctx, &mut app, "Import project");
    assert!(app.more_controls.is_none());
    let imported = app.editor.document.clone();
    assert_eq!(imported.live_pages().count(), 4);
    assert_eq!(app.editor.undo_name(), Some("Import project"));
    app.editor.undo();
    assert_eq!(app.editor.document, before);
    app.editor.redo();
    assert_eq!(app.editor.document, imported);
    assert_eq!(
        wireframe::document::decode(&imported.encode().unwrap()).unwrap(),
        imported
    );
}
#[test]
fn cancel_escape_and_canvas_shortcuts_do_not_mutate_the_destination() {
    let (ctx, mut app) = setup(0);
    let before = app.editor.document.clone();
    click(&ctx, &mut app, "Import into Project");
    click(&ctx, &mut app, "Cancel import");
    assert!(app.more_controls.as_ref().unwrap().review.is_none());
    let escape = || egui::Event::Key {
        key: Key::Escape,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    };
    frame(&ctx, &mut app, vec![escape()]);
    assert!(app.more_controls.as_ref().unwrap().preview.is_none());
    frame(
        &ctx,
        &mut app,
        vec![egui::Event::Key {
            key: Key::Delete,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
    );
    assert_eq!(app.editor.document, before);
    frame(&ctx, &mut app, vec![escape()]);
    assert!(app.more_controls.is_none());
    assert_eq!(app.editor.document, before);
}
#[test]
fn review_stages_reuse_pages_and_libraries_without_empty_history() {
    let mut e = Editor::default();
    let snapshot = Catalog::default().entries[3].load().unwrap();
    let mut review = Review::new(&e, &snapshot).unwrap();
    e.apply_project_copy(review.take_prepared(&e, &snapshot).unwrap())
        .unwrap();
    let before = e.document.clone();
    let history = e.undo_name().map(str::to_owned);
    let mut repeated = Review::new(&e, &snapshot).unwrap();
    assert!(repeated.conflicts().is_empty());
    e.apply_project_copy(repeated.take_prepared(&e, &snapshot).unwrap())
        .unwrap();
    assert_eq!(e.document, before);
    assert_eq!(e.undo_name(), history.as_deref());
    let landing = Catalog::default().entries[0].load().unwrap();
    let mut add = Review::new(&e, &landing).unwrap();
    e.apply_project_copy(add.take_prepared(&e, &landing).unwrap())
        .unwrap();
    let before = e.document.clone();
    let mut repeat = Review::new(&e, &landing).unwrap();
    assert!(repeat.conflicts().is_empty());
    e.apply_project_copy(repeat.take_prepared(&e, &landing).unwrap())
        .unwrap();
    assert_eq!(e.document, before);
}
#[test]
fn keep_both_reopens_review_for_a_previously_equivalent_linking_page() {
    let mut snapshot = Catalog::default().entries[0].load().unwrap();
    let mut destination = Editor::default();
    let mut first = Review::new(&destination, &snapshot).unwrap();
    destination
        .apply_project_copy(first.take_prepared(&destination, &snapshot).unwrap())
        .unwrap();
    let before = destination.document.clone();
    let contact = snapshot.project.pages[1].id;
    let landing = snapshot.project.pages[0].id;
    snapshot.project.pages[1].notes = "Changed contact".into();
    let mut review = Review::new(&destination, &snapshot).unwrap();
    assert_eq!(
        review
            .conflicts()
            .iter()
            .map(|c| c.incoming)
            .collect::<Vec<_>>(),
        vec![contact]
    );
    review.choose(Choice::Both);
    assert_eq!(
        review
            .conflicts()
            .iter()
            .map(|c| c.incoming)
            .collect::<Vec<_>>(),
        vec![landing]
    );
    review.choose(Choice::New);
    assert!(review.conflicts().is_empty());
    let prepared = review.take_prepared(&destination, &snapshot).unwrap();
    assert_eq!(prepared.counts().pages, 1);
    assert_eq!(prepared.counts().pages_replaced, 1);
    destination.apply_project_copy(prepared).unwrap();
    let copied_contact = destination
        .document
        .pages
        .iter()
        .find(|p| p.notes == "Changed contact")
        .unwrap()
        .id;
    let landing = destination
        .document
        .pages
        .iter()
        .find(|p| p.name == "Landing")
        .unwrap();
    assert!(
        landing
            .content
            .nodes
            .iter()
            .any(|n| n.properties.get("link")
                == Some(&wireframe::document::Value::Link(
                    wireframe::document::Link::Page(copied_contact)
                )))
    );
    let after = destination.document.clone();
    destination.undo();
    assert_eq!(destination.document, before);
    destination.redo();
    assert_eq!(destination.document, after);
}

#[test]
fn library_keep_both_rechecks_automatic_pages_without_losing_the_choice() {
    let mut snapshot = Catalog::default().entries[3].load().unwrap();
    let definition = snapshot.project.definitions[0].id;
    snapshot.project.pages[0].name = "Component preview".into();
    let mut source = Editor::new(snapshot.project.clone());
    source.insert_component(definition, 50., 50.).unwrap();
    snapshot.project = source.document;
    let mut destination = Editor::default();
    let mut first = Review::new(&destination, &snapshot).unwrap();
    destination
        .apply_project_copy(first.take_prepared(&destination, &snapshot).unwrap())
        .unwrap();
    let before = destination.document.clone();
    snapshot.project.definitions[0].content.nodes[0]
        .properties
        .insert(
            "text".into(),
            wireframe::document::Value::Text("Changed source".into()),
        );
    let mut review = Review::new(&destination, &snapshot).unwrap();
    assert_eq!(review.conflicts().len(), 1);
    assert!(matches!(
        review.conflicts()[0].kind,
        review::Resource::Library
    ));
    review.choose(Choice::Both);
    assert_eq!(review.conflicts().len(), 1);
    assert!(matches!(review.conflicts()[0].kind, review::Resource::Page));
    review.choose(Choice::New);
    assert!(review.conflicts().is_empty());
    let prepared = review.take_prepared(&destination, &snapshot).unwrap();
    assert_eq!(prepared.counts().libraries, 1);
    assert_eq!(prepared.counts().pages_replaced, 1);
    destination.apply_project_copy(prepared).unwrap();
    let page = destination
        .document
        .live_pages()
        .find(|p| !p.content.nodes.is_empty())
        .unwrap();
    let wireframe::document::NodeContent::Instance {
        definition: new_definition,
        ..
    } = page.content.nodes[0].content
    else {
        panic!("expected instance");
    };
    assert!(!before.definitions.iter().any(|d| d.id == new_definition));
    let after = destination.document.clone();
    destination.undo();
    assert_eq!(destination.document, before);
    destination.redo();
    assert_eq!(destination.document, after);
}

#[test]
fn stale_destination_or_source_cannot_apply_reviewed_import() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("source.wireframe");
    let source = Catalog::default().entries[0].load().unwrap().project;
    std::fs::write(&path, source.encode().unwrap()).unwrap();
    let snapshot = Snapshot::local(&path).unwrap();
    let mut e = Editor::default();
    let mut review = Review::new(&e, &snapshot).unwrap();
    e.document.title = "Changed destination".into();
    let before = e.document.clone();
    assert!(review.take_prepared(&e, &snapshot).is_err());
    assert_eq!(e.document, before);
    let mut review = Review::new(&e, &snapshot).unwrap();
    std::fs::write(&path, b"changed source").unwrap();
    assert!(review.take_prepared(&e, &snapshot).is_err());
    assert_eq!(e.document, before);
}
#[test]
fn back_preserves_search_and_category_and_preview_is_read_only() {
    let (ctx, mut app) = setup(1);
    let browser = app.more_controls.as_mut().unwrap();
    browser.catalog.query = "contact".into();
    browser.catalog.category = "Forms".into();
    let source = browser.preview.as_ref().unwrap().snapshot.project.clone();
    let before = app.editor.document.clone();
    click(&ctx, &mut app, "← Back to results");
    let browser = app.more_controls.as_ref().unwrap();
    assert_eq!(browser.catalog.query, "contact");
    assert_eq!(browser.catalog.category, "Forms");
    assert!(browser.preview.is_none());
    assert_eq!(app.editor.document, before);
    assert_eq!(
        source,
        Catalog::default().entries[1].load().unwrap().project
    );
}

#[test]
fn combined_review_steps_images_pages_and_libraries_before_one_apply() {
    use std::{collections::BTreeMap, io::Cursor};
    use wireframe::document::{Asset, Value};
    let mut snapshot = Catalog::default().entries[0].load().unwrap();
    let components = Catalog::default().entries[3].load().unwrap();
    let mut source = Editor::new(snapshot.project.clone());
    let copy = source
        .prepare_project_copy(&components.project, &components.origin, &[])
        .unwrap();
    source.apply_project_copy(copy).unwrap();
    let asset = source.document.allocate_id().unwrap();
    let png = |color: u8| {
        let mut out = Cursor::new(vec![]);
        image::RgbaImage::from_pixel(1, 1, image::Rgba([color, 0, 0, 255]))
            .write_to(&mut out, image::ImageFormat::Png)
            .unwrap();
        out.into_inner()
    };
    source.document.assets.push(Asset {
        trashed: false,
        id: asset,
        name: "Logo".into(),
        media_type: "image/png".into(),
        bytes: png(0),
    });
    snapshot.project = source.document;
    let mut destination = Editor::default();
    let mut first = Review::new(&destination, &snapshot).unwrap();
    assert!(first.error.is_none(), "{:?}", first.error);
    destination
        .apply_project_copy(first.take_prepared(&destination, &snapshot).unwrap())
        .unwrap();
    let before = destination.document.clone();
    snapshot.project.assets[0].bytes = png(255);
    snapshot.project.pages[0].notes = "Changed page".into();
    snapshot.project.definitions[0].content.nodes[0].properties =
        BTreeMap::from([("text".into(), Value::Text("New action".into()))]);
    let mut review = Review::new(&destination, &snapshot).unwrap();
    assert!(matches!(
        review.conflicts()[0].kind,
        review::Resource::Asset
    ));
    review.choose(Choice::Existing);
    assert!(matches!(review.conflicts()[0].kind, review::Resource::Page));
    review.all_remaining = true;
    review.choose(Choice::Both);
    assert!(matches!(
        review.conflicts()[0].kind,
        review::Resource::Library
    ));
    review.choose(Choice::New);
    assert_eq!(destination.document, before);
    let prepared = review.take_prepared(&destination, &snapshot).unwrap();
    assert_eq!(prepared.counts().pages, 1);
    assert_eq!(prepared.counts().pages_reused, 2);
    assert_eq!(prepared.counts().assets_reused, 1);
    assert_eq!(prepared.counts().libraries_replaced, 1);
    destination.apply_project_copy(prepared).unwrap();
    let after = destination.document.clone();
    destination.undo();
    assert_eq!(destination.document, before);
    destination.redo();
    assert_eq!(destination.document, after);
}

#[test]
fn catalog_results_are_vertical_and_inside_the_window() {
    let (ctx, mut app) = setup(0);
    app.more_controls.as_mut().unwrap().preview = None;
    frame(&ctx, &mut app, vec![]);
    let nodes = frame(&ctx, &mut app, vec![]);
    let labels = [
        "Preview Contact forms",
        "Preview Form components",
        "Preview Landing page",
        "Preview Tablet order",
    ];
    let mut bottom = 0.;
    for label in labels {
        let rect = nodes[label];
        assert!(
            rect.left() >= 0. && rect.right() <= 1100.,
            "{label}: {rect:?}"
        );
        assert!(
            rect.top() >= bottom,
            "overlapping results: {label}: {rect:?}"
        );
        bottom = rect.bottom();
    }
    let close = nodes["Close"];
    assert!(
        close.left() >= 0. && close.right() <= 1100. && close.top() >= 0. && close.bottom() <= 800.
    );
}

#[test]
fn conflict_preview_uses_destination_font_without_changing_the_source() {
    let mut snapshot = Catalog::default().entries[0].load().unwrap();
    let source_font = snapshot.project.font.clone();
    let mut destination = Editor::new(Project {
        font: None,
        ..Project::default()
    });
    let mut first = Review::new(&destination, &snapshot).unwrap();
    destination
        .apply_project_copy(first.take_prepared(&destination, &snapshot).unwrap())
        .unwrap();
    // A real metadata change requires review; identical pages now reuse directly.
    snapshot.project.pages[0].notes = "Updated notes".into();
    let incoming = Editor::new(snapshot.project.clone());
    let mut review = Review::new(&destination, &snapshot).unwrap();
    review.preview_pair(&destination, &incoming);
    let pair = &review.pair.as_ref().unwrap().scenes;
    assert!(pair[0].as_ref().unwrap().font.is_none());
    assert!(pair[1].as_ref().unwrap().font.is_none());
    assert_eq!(
        pair[0].as_ref().unwrap().bounds,
        pair[1].as_ref().unwrap().bounds
    );
    assert_eq!(incoming.document.font, source_font);
    assert_eq!(snapshot.project.font, source_font);
}

#[test]
fn asset_review_exposes_availability_and_previews_retained_artwork_read_only() {
    use wireframe::{document::Asset, render::Drawing};
    let mut snapshot = Catalog::default().entries[0].load().unwrap();
    let id = snapshot.project.allocate_id().unwrap();
    let mut bytes = std::io::Cursor::new(vec![]);
    image::RgbaImage::from_pixel(2, 2, image::Rgba([20, 40, 60, 255]))
        .write_to(&mut bytes, image::ImageFormat::Png)
        .unwrap();
    snapshot.project.assets.push(Asset {
        id,
        trashed: false,
        name: "Photo".into(),
        media_type: "image/png".into(),
        bytes: bytes.into_inner(),
    });
    let mut destination = Editor::default();
    let mut first = Review::new(&destination, &snapshot).unwrap();
    destination
        .apply_project_copy(first.take_prepared(&destination, &snapshot).unwrap())
        .unwrap();
    snapshot.project.assets[0].trashed = true;
    let before = snapshot.project.clone();
    let mut review = Review::new(&destination, &snapshot).unwrap();
    assert_eq!(
        review.conflicts()[0].image_availability,
        Some((false, true))
    );
    review.preview_pair(&destination, &Editor::new(snapshot.project.clone()));
    for scene in &review.pair.as_ref().unwrap().scenes {
        assert!(
            scene
                .as_ref()
                .unwrap()
                .items
                .iter()
                .any(|item| matches!(item.drawing, Drawing::Image { .. }))
        );
    }
    assert_eq!(snapshot.project, before);
    assert!(snapshot.project.assets[0].trashed);
    assert!(!destination.document.assets[0].trashed);
}

#[test]
fn preview_resource_names_stay_bounded_and_keep_full_accessible_values() {
    let (ctx, mut app) = setup(0);
    let name = "واجهة שלום 👩‍💻 long-page-name ".repeat(20);
    let preview = app
        .more_controls
        .as_mut()
        .unwrap()
        .preview
        .as_mut()
        .unwrap();
    preview.snapshot.project.pages[0].name = name.clone();
    preview.resource = preview.snapshot.project.pages[0].id;
    let before = app.editor.document.clone();
    for _ in 0..3 {
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1100., 800.))),
                ..Default::default()
            },
            |ui| {
                app.more_controls_dialog(ui.ctx());
            },
        );
        output.textures_delta.clear();
        let nodes = &output
            .platform_output
            .accesskit_update
            .as_ref()
            .unwrap()
            .nodes;
        let combo = nodes
            .iter()
            .find(|(_, n)| n.role() == egui::accesskit::Role::ComboBox)
            .unwrap();
        assert_eq!(combo.1.value(), Some(name.as_str()));
        let bounds = combo.1.bounds().unwrap();
        assert!(
            bounds.width() <= 1036. && bounds.x1 <= 1100.,
            "preview picker={bounds:?}"
        );
    }
    assert_eq!(app.editor.document, before);
}
