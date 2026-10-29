// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Production-event regressions. AccessKit events here are framework input,
//! never evidence that a native accessibility adapter delivered those actions.
use super::*;
use eframe::App;
use egui::accesskit::{Action, ActionRequest, NodeId, TreeId};

const DRAFT: &str = "Pending canvas text survives Trash";
const FINISH_REQUIRED: &str = "Finish editing text, then try moving to Trash again.";
const REDO_TITLE: &str = "R11 actual Redo payload";

struct Golden {
    baseline: &'static str,
    edited: &'static str,
    retained: &'static str,
    redo: &'static str,
    batch: bool,
}
const SINGLE: Golden = Golden {
    baseline: include_str!("retention_fixtures/single-B.json"),
    edited: include_str!("retention_fixtures/single-D.json"),
    retained: include_str!("retention_fixtures/single-T-D.json"),
    redo: include_str!("retention_fixtures/single-R.json"),
    batch: false,
};
const BATCH: Golden = Golden {
    baseline: include_str!("retention_fixtures/batch-B.json"),
    edited: include_str!("retention_fixtures/batch-D.json"),
    retained: include_str!("retention_fixtures/batch-T-D.json"),
    redo: include_str!("retention_fixtures/batch-R.json"),
    batch: true,
};

fn project(json: &str) -> Project {
    let project: Project = serde_json::from_str(json).expect("independently frozen complete graph");
    project.validate().expect("valid frozen graph");
    project
}

#[derive(Clone)]
struct LiveNode {
    id: NodeId,
    label: String,
    rect: Rect,
    disabled: bool,
}
struct Paint {
    nodes: Vec<LiveNode>,
    focus: NodeId,
}
impl Paint {
    fn enabled(&self, label: &str) -> LiveNode {
        let nodes: Vec<_> = self
            .nodes
            .iter()
            .filter(|n| n.label == label && !n.disabled)
            .collect();
        assert_eq!(nodes.len(), 1, "one live enabled {label} node required");
        (*nodes[0]).clone()
    }
}

struct Harness {
    ctx: egui::Context,
    app: WireframeApp,
}
impl Harness {
    fn new(baseline: &str, redo: &str, batch: bool) -> Self {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let mut app = WireframeApp::workspace(&ctx, None);
        // No file, recovery path, persistence roots, chooser or native menus.
        // Model use here prepares a genuine known history branch only. All
        // text entry, Trash, Restore and subsequent history witnesses use UI.
        app.editor = Editor::new(project(baseline));
        app.show_library = false;
        app.show_inspector = false;
        app.thumbnails = false;
        assert!(app.file.is_none() && app.persistence.is_none());
        assert!(app.recovery.is_none() && app.recovery_offer.is_none());
        app.editor.rename_project(REDO_TITLE).unwrap();
        assert_eq!(app.editor.document, project(redo));
        app.editor.undo();
        let mut h = Self { ctx, app };
        h.graph(baseline);
        h.history(None, Some("Rename project"));
        h.settle();
        h.click("Draft page", Modifiers::NONE);
        if batch {
            h.click("Batch peer", Modifiers::COMMAND);
        }
        assert_eq!(h.app.editor.page, 1);
        let selected = if batch {
            HashSet::from([1, 2])
        } else {
            HashSet::from([1])
        };
        assert_eq!(h.app.navigator.selected, selected);
        h
    }

    fn frame(&mut self, mut events: Vec<egui::Event>) -> Paint {
        let modifiers = events
            .iter()
            .find_map(|event| match event {
                egui::Event::Key { modifiers, .. }
                | egui::Event::PointerButton { modifiers, .. } => Some(*modifiers),
                _ => None,
            })
            .unwrap_or_default();
        events.insert(0, egui::Event::ModifiersChanged(modifiers));
        let mut raw = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1200., 850.))),
            time: Some(self.ctx.cumulative_frame_nr() as f64 * 0.1),
            events,
            ..Default::default()
        };
        self.app.raw_input_hook(&self.ctx, &mut raw);
        let ctx = self.ctx.clone();
        let out = ctx.run_ui(raw, |ui| {
            self.app.ui(ui, &mut eframe::Frame::_new_kittest())
        });
        let update = out
            .platform_output
            .accesskit_update
            .as_ref()
            .expect("live AccessKit output");
        let paint = Paint {
            focus: update.focus,
            nodes: update
                .nodes
                .iter()
                .filter_map(|(id, node)| {
                    let b = node.bounds()?;
                    Some(LiveNode {
                        id: *id,
                        label: node.label().unwrap_or_default().to_owned(),
                        rect: Rect::from_min_max(
                            egui::pos2(b.x0 as f32, b.y0 as f32),
                            egui::pos2(b.x1 as f32, b.y1 as f32),
                        ),
                        disabled: node.is_disabled(),
                    })
                })
                .collect(),
        };
        out.drop_without_applying_deltas();
        paint
    }

    fn settle(&mut self) -> Paint {
        self.frame(vec![]);
        self.frame(vec![]);
        self.frame(vec![])
    }

    fn tap(&mut self, key: Key, modifiers: Modifiers) -> Paint {
        let event = |pressed| egui::Event::Key {
            key,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers,
        };
        self.frame(vec![event(true)]);
        self.frame(vec![event(false)])
    }

    fn pointer(&mut self, pos: Pos2, button: egui::PointerButton, modifiers: Modifiers) {
        for pressed in [true, false] {
            self.frame(vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button,
                    pressed,
                    modifiers,
                },
            ]);
        }
    }

    fn click(&mut self, label: &str, modifiers: Modifiers) {
        let node = self.settle().enabled(label);
        self.pointer(node.rect.center(), egui::PointerButton::Primary, modifiers);
        self.settle();
    }

    fn focus(&mut self, id: NodeId) {
        let live = self.settle();
        assert!(
            live.nodes.iter().any(|n| n.id == id && !n.disabled),
            "Focus must target an actual live enabled node"
        );
        let focused = self.frame(vec![egui::Event::AccessKitActionRequest(ActionRequest {
            action: Action::Focus,
            target_tree: TreeId::ROOT,
            target_node: id,
            data: None,
        })]);
        assert_eq!(focused.focus, id, "framework must actually transfer Focus");
        assert_eq!(
            self.settle().focus,
            id,
            "Focus must persist before later Space"
        );
    }

    fn popup(&self) -> Rect {
        self.ctx
            .memory(|m| m.area_rect(egui::Id::new("canvas-text-session")))
            .expect("actual canvas popup rectangle")
    }

    fn begin_draft(&mut self, baseline: &str) {
        // Select the real Paragraph on the canvas, then use the ordinary Enter
        // shortcut. No begin_text_edit call or app buffer/view assignment.
        let pos = self.app.screen([250., 110.]);
        self.pointer(pos, egui::PointerButton::Primary, Modifiers::NONE);
        assert_eq!(self.app.editor.selection, HashSet::from([10]));
        self.tap(Key::Enter, Modifiers::NONE);
        self.settle();
        let field = self
            .app
            .text_edit_view
            .as_ref()
            .expect("normal Enter opens canvas editor")
            .field_id
            .unwrap();
        assert_eq!(self.ctx.memory(|m| m.focused()), Some(field));
        self.frame(vec![egui::Event::Text(DRAFT.into())]);
        self.settle();
        self.draft(baseline);
        assert!(self.app.text_edit_view.as_ref().unwrap().preview.is_some());
    }

    fn draft(&self, baseline: &str) {
        self.graph(baseline);
        assert_eq!(
            self.app
                .text_edit
                .as_ref()
                .map(|(id, text)| (*id, text.as_str())),
            Some((10, DRAFT)),
            "real cancellable field buffer must survive"
        );
        assert!(
            self.app
                .text_edit_view
                .as_ref()
                .is_some_and(|view| view.matches(&self.app.editor))
        );
    }

    fn graph(&self, frozen: &str) {
        let expected = project(frozen);
        assert_eq!(
            self.app.editor.document,
            expected,
            "complete project/sentinels/allocator; canvas buffer={:?}; view present={}",
            self.app.text_edit,
            self.app.text_edit_view.is_some()
        );
        assert_eq!(
            self.app.editor.document.encode().unwrap(),
            expected.encode().unwrap(),
            "complete canonical serialized graph"
        );
    }

    fn history(&self, undo: Option<&str>, redo: Option<&str>) {
        assert_eq!(self.app.editor.undo_name(), undo);
        assert_eq!(self.app.editor.redo_name(), redo);
    }

    fn page_menu(&mut self, batch: bool) -> LiveNode {
        let row = self.settle().enabled("Draft page");
        let pos = row.rect.center();
        if self.app.text_edit.is_some() {
            assert!(
                !self.popup().contains(pos),
                "secondary row click must be outside text popup"
            );
        }
        self.pointer(pos, egui::PointerButton::Secondary, Modifiers::NONE);
        assert!(
            egui::Popup::is_any_open(&self.ctx),
            "real row context menu must open"
        );
        self.settle().enabled(if batch {
            "Move 2 items to Trash"
        } else {
            "Move to Trash"
        })
    }

    fn focus_space_move(&mut self, batch: bool, baseline: &str, has_draft: bool) {
        let node = self.page_menu(batch);
        self.focus(node.id);
        if has_draft {
            self.draft(baseline);
            self.history(None, Some("Rename project"));
        }
        // Separate frame(s) after the actual live Focus request. This is Space,
        // not AccessKit Click and not direct page/model command invocation.
        self.tap(Key::Space, Modifiers::NONE);
        self.settle(); // exercise the next-frame stale-context guard as well
        assert!(
            !egui::Popup::is_any_open(&self.ctx),
            "Space must activate/close the real context menu"
        );
    }

    fn cancel_and_prove_redo(&mut self, baseline: &str, redo: &str) {
        self.tap(Key::Escape, Modifiers::NONE);
        self.settle();
        assert!(self.app.text_edit.is_none());
        assert!(self.app.text_edit_view.is_none());
        if self.app.error.is_some() {
            self.tap(Key::Escape, Modifiers::NONE);
            self.settle();
        }
        self.graph(baseline);
        self.history(None, Some("Rename project"));
        self.tap(Key::Z, Modifiers::COMMAND | Modifiers::SHIFT);
        self.graph(redo); // real prior Redo payload, not just its enabled label
        self.tap(Key::Z, Modifiers::COMMAND);
        self.graph(baseline);
        self.history(None, Some("Rename project"));
    }

    fn finish(&mut self, edited: &str) {
        let field = self
            .app
            .text_edit_view
            .as_ref()
            .unwrap()
            .field_id
            .unwrap()
            .accesskit_id();
        self.focus(field);
        self.tap(Key::Enter, Modifiers::COMMAND);
        self.settle();
        assert!(self.app.text_edit.is_none());
        self.graph(edited);
        self.history(Some("Edit text"), None);
    }

    fn history_and_restore(&mut self, golden: &Golden) {
        self.graph(golden.retained);
        self.history(Some("Move to Trash"), None);
        self.tap(Key::Z, Modifiers::COMMAND);
        self.graph(golden.edited);
        self.history(Some("Edit text"), Some("Move to Trash"));
        self.tap(Key::Z, Modifiers::COMMAND);
        self.graph(golden.baseline);
        self.history(None, Some("Edit text"));
        self.tap(Key::Z, Modifiers::COMMAND | Modifiers::SHIFT);
        self.graph(golden.edited);
        self.tap(Key::Z, Modifiers::COMMAND | Modifiers::SHIFT);
        self.graph(golden.retained);
        self.click("Trash", Modifiers::NONE);
        assert!(self.app.trash.is_some());
        self.tap(Key::A, Modifiers::COMMAND); // real modal Select All, also for batch
        self.click("Restore", Modifiers::NONE);
        self.graph(golden.edited);
        self.history(Some("Restore from Trash"), None);
        self.tap(Key::Z, Modifiers::COMMAND);
        self.graph(golden.retained);
        self.tap(Key::Z, Modifiers::COMMAND | Modifiers::SHIFT);
        self.graph(golden.edited);
    }
}

fn guarded_move(golden: &Golden) {
    let mut h = Harness::new(golden.baseline, golden.redo, golden.batch);
    h.begin_draft(golden.baseline);
    h.focus_space_move(golden.batch, golden.baseline, true);
    // Expected fail-before on source799, provided all event witness assertions
    // succeeded. T(B) or a vanished editor is never a preserving pass.
    h.draft(golden.baseline);
    h.history(None, Some("Rename project"));
    assert!(h.app.error.is_none(), "finish guidance should be nonmodal");
    assert_eq!(
        h.app.status, FINISH_REQUIRED,
        "dispatch must clearly explain how to finish and retry"
    );
    h.cancel_and_prove_redo(golden.baseline, golden.redo);
    h.begin_draft(golden.baseline);
    h.finish(golden.edited);
    h.focus_space_move(golden.batch, golden.edited, false);
    h.history_and_restore(golden);
}

#[test]
fn page_context_focus_space_keeps_single_canvas_draft_and_redo_until_finished() {
    guarded_move(&SINGLE);
}

#[test]
fn page_context_focus_space_keeps_batch_canvas_draft_and_redo_until_finished() {
    guarded_move(&BATCH);
}

#[test]
fn page_context_outside_primary_accepts_canvas_text_before_single_and_batch_trash() {
    for golden in [&SINGLE, &BATCH] {
        let mut h = Harness::new(golden.baseline, golden.redo, golden.batch);
        h.begin_draft(golden.baseline);
        let node = h.page_menu(golden.batch);
        let pos = node.rect.center();
        assert!(
            !h.popup().contains(pos),
            "normal pointer control must actually be non-overlapping"
        );
        h.pointer(pos, egui::PointerButton::Primary, Modifiers::NONE);
        h.settle();
        assert!(h.app.text_edit.is_none());
        assert!(
            !egui::Popup::is_any_open(&h.ctx),
            "one press/release must retain normal activation"
        );
        h.history_and_restore(golden);
    }
}

#[test]
fn last_page_focus_space_requires_finish_and_preserves_real_allocator_rejection_history() {
    let baseline = include_str!("retention_fixtures/exhausted-B.json");
    let redo = include_str!("retention_fixtures/exhausted-R.json");
    let mut h = Harness::new(baseline, redo, false);
    h.begin_draft(baseline);
    h.focus_space_move(false, baseline, true);
    h.draft(baseline);
    h.history(None, Some("Rename project"));
    assert!(h.app.error.is_none());
    assert_eq!(h.app.status, FINISH_REQUIRED);
    h.cancel_and_prove_redo(baseline, redo);
    // No draft now: let the real context dispatch reach allocator rejection.
    // The preceding guard is not relabelled an active-draft model rejection.
    h.focus_space_move(false, baseline, false);
    assert!(
        h.app
            .error
            .as_deref()
            .is_some_and(|error| error.contains("Project ID space exhausted"))
    );
    h.graph(baseline);
    h.history(None, Some("Rename project"));
    h.tap(Key::Escape, Modifiers::NONE);
    h.tap(Key::Z, Modifiers::COMMAND | Modifiers::SHIFT);
    h.graph(redo);
    h.tap(Key::Z, Modifiers::COMMAND);
    h.graph(baseline);
}
