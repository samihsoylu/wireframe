// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{
    document::{NodeContent, Size},
    workspace::Editor,
};
/// Seed stored geometry independently of the interactive resize command.
/// Rendering and compatibility checks must retain subminimum fixture coverage.
pub fn stored_size(editor: &mut Editor, width: f32, height: f32) {
    let id = *editor
        .selection
        .iter()
        .next()
        .expect("Selected fixture control");
    let node = editor
        .document
        .pages
        .iter_mut()
        .find(|page| page.id == editor.page)
        .expect("Current fixture page")
        .content
        .nodes
        .iter_mut()
        .find(|n| n.id == id)
        .expect("Root fixture control");
    let NodeContent::Control { size, .. } = &mut node.content else {
        panic!("Control fixture required")
    };
    *size = Size::fixed(width, height);
}
