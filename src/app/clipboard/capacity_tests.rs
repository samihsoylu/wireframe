// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use wireframe::document::{Asset, Node, Size, Value};

fn image_project() -> Project {
    let mut state = 42_u32;
    let pixels = image::RgbImage::from_fn(1024, 512, |_, _| {
        let mut rgb = [0; 3];
        for byte in &mut rgb {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            *byte = state as u8;
        }
        image::Rgb(rgb)
    });
    let mut encoded = std::io::Cursor::new(Vec::new());
    pixels
        .write_to(&mut encoded, image::ImageFormat::Png)
        .unwrap();
    let mut project = Project {
        font: None,
        next_id: 100,
        ..Project::default()
    };
    project.pages[0].name = "Image page".into();
    let mut node = Node::control(2, "image", Point::new(20., 30.), Size::fixed(320., 160.));
    node.properties.insert("image".into(), Value::Asset(3));
    project.pages[0].content.nodes.push(node);
    project.assets.push(Asset {
        attribution: None,
        id: 3,
        trashed: false,
        name: "Photo".into(),
        media_type: "image/png".into(),
        bytes: encoded.into_inner(),
    });
    project.validate().unwrap();
    project
}

#[test]
fn large_image_clipboards_round_trip_canvas_page_and_library_with_exact_history() {
    let project = image_project();
    for route in 0..3 {
        let ctx = egui::Context::default();
        let mut source = WireframeApp::workspace(&ctx, None);
        source.editor = Editor::new(project.clone());
        source.editor.selection.insert(2);
        let origin = source.clipboard_origin();
        let copy = match route {
            0 => source.editor.copy_with_origin(&origin).unwrap(),
            1 => source.editor.copy_pages(&[1], &origin).unwrap(),
            _ => {
                let id = source
                    .editor
                    .create_component("Photo component".into())
                    .unwrap();
                let library = source.editor.document.component_source(id).unwrap().0.id;
                source.editor.copy_libraries(&[library], &origin).unwrap()
            }
        };
        let text = clipboard_text(&copy).expect("valid image payload must fit transport");
        assert!(text.len() > 4_000_000);
        let source_before = source.editor.document.clone();
        let scope = match route {
            0 => CutScope::Controls,
            1 => CutScope::Pages(vec![1]),
            _ => CutScope::Libraries(copy.resources.as_ref().unwrap().requested.clone()),
        };
        source.request_cut(&ctx, copy.clone(), scope).unwrap();
        assert_eq!(source.editor.document, source_before);
        let request = source.pending_cut_request().unwrap();
        source.finish_cut(&ctx, request, &Err("Clipboard busy".into()));
        assert_eq!(source.editor.document, source_before);
        assert!(source.error.take().unwrap().contains("Nothing was cut"));
        source.publish_clipboard(&ctx, copy).unwrap();
        assert!(ctx.output(|o| {
            o.commands
                .iter()
                .any(|c| matches!(c, egui::OutputCommand::CopyText(t) if t == &text))
        }));
        let mut target = WireframeApp::workspace(&ctx, None);
        let before = target.editor.document.clone();
        if route == 1 {
            target.pasted_page_text(&text);
        } else {
            target.pasted(&text);
        }
        assert!(target.error.is_none(), "route {route}: {:?}", target.error);
        assert!(target.editor.error.is_none(), "{:?}", target.editor.error);
        assert_eq!(
            target.editor.document.assets[0].bytes,
            project.assets[0].bytes
        );
        assert_ne!(target.editor.document, before);
        let after = target.editor.document.clone();
        target.editor.undo();
        assert_eq!(target.editor.document, before);
        target.editor.redo();
        assert_eq!(target.editor.document, after);
    }
}

#[test]
fn oversized_input_is_rejected_before_parsing_without_changing_document_or_redo() {
    let ctx = egui::Context::default();
    let mut app = WireframeApp::workspace(&ctx, None);
    app.editor.add(Kind::Button, 20., 30.);
    app.editor.undo();
    let before = app.editor.document.clone();
    let redo = app.editor.redo_name().map(str::to_owned);
    let mut text = String::from("wireframe-nodes-v2:");
    text.extend(std::iter::repeat_n(' ', MAX_JSON_BYTES + 1));
    for page_target in [false, true] {
        if page_target {
            app.pasted_page_text(&text);
        } else {
            app.pasted(&text);
        }
        assert_eq!(
            app.error.take().as_deref(),
            Some("Clipboard content is too large")
        );
        assert_eq!(app.editor.document, before);
        assert_eq!(app.editor.redo_name(), redo.as_deref());
    }
}
