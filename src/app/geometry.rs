// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;

fn row(ui: &mut egui::Ui, title: &str, fields: [(&str, &mut f32, bool); 2]) -> (bool, bool) {
    let mut changed = false;
    let mut finish = false;
    let label_width = (ui.available_width() * 0.4).floor();
    let width = ((ui.available_width() - label_width - 16.) / 2.).floor();
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = 8.;
        ui.allocate_ui_with_layout(
            Vec2::new(label_width, 24.),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.set_min_size(Vec2::new(label_width, 24.));
                ui.strong(title);
            },
        );
        for (label, value, enabled) in fields {
            ui.allocate_ui_with_layout(
                Vec2::new(width, 41.),
                egui::Layout::top_down(egui::Align::Center),
                |ui| {
                    ui.set_min_size(Vec2::new(width, 41.));
                    ui.spacing_mut().item_spacing.y = 0.;
                    if enabled {
                        let id = ui.next_auto_id();
                        if ui.memory(|m| m.has_focus(id))
                            && ui.input(|i| i.key_pressed(Key::Escape))
                        {
                            ui.memory_mut(|m| m.surrender_focus(id));
                        }
                        ui.spacing_mut().interact_size = Vec2::new(width, 24.);
                        let response = ui.add(
                            egui::DragValue::new(value)
                                .speed(1.)
                                .update_while_editing(false)
                                .range(0.0..=100_000.0)
                                .clamp_existing_to_range(false)
                                .custom_formatter(|v, _| format!("{v:.0}"))
                                .custom_parser(|s| {
                                    s.chars()
                                        .filter(char::is_ascii_digit)
                                        .collect::<String>()
                                        .parse()
                                        .ok()
                                }),
                        );
                        if response.has_focus() {
                            ui.memory_mut(|m| {
                                m.set_focus_lock_filter(
                                    response.id,
                                    egui::EventFilter {
                                        horizontal_arrows: true,
                                        vertical_arrows: true,
                                        tab: false,
                                        escape: true,
                                    },
                                )
                            });
                        }
                        changed |= response.changed();
                        finish |= response.drag_stopped() || response.lost_focus();
                        let caption = ui.label(label);
                        response.labelled_by(caption.id);
                    } else {
                        ui.add_sized([width, 24.], egui::Label::new(format!("{value:.0}")));
                        ui.label(label);
                    }
                },
            );
        }
    });
    (changed, finish)
}

impl WireframeApp {
    pub(super) fn geometry_properties(&mut self, ui: &mut egui::Ui) {
        let Some(state) = self.editor.selection_geometry() else {
            return;
        };
        let mut bounds = state.bounds;
        ui.add_space(10.);
        let (position, size) = ui
            .scope(|ui| {
                ui.spacing_mut().item_spacing.y = 0.;
                let position = row(
                    ui,
                    "Position",
                    [
                        ("X", &mut bounds.x, !state.locked),
                        ("Y", &mut bounds.y, !state.locked),
                    ],
                );
                let size = row(
                    ui,
                    "Size",
                    [
                        (
                            "Width",
                            &mut bounds.w,
                            !state.locked && state.width_editable,
                        ),
                        (
                            "Height",
                            &mut bounds.h,
                            !state.locked && state.height_editable,
                        ),
                    ],
                );
                (position, size)
            })
            .inner;
        if (position.0 || size.0) && bounds != state.bounds {
            self.editor.begin("Edit geometry");
            if size.0 {
                self.editor.update_selection_geometry(bounds);
            } else {
                self.editor
                    .update_selection_position(wireframe::document::Point::new(bounds.x, bounds.y));
            }
        }
        if position.1
            || size.1
            || ((position.0 || size.0)
                && !ui.ctx().text_edit_focused()
                && !ui.input(|i| i.pointer.any_down()))
        {
            self.editor.commit();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(
        app: &mut WireframeApp,
        ctx: &egui::Context,
        width: f32,
        events: Vec<egui::Event>,
    ) -> egui::FullOutput {
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(width, 500.))),
                events,
                ..Default::default()
            },
            |ui| {
                app.shortcuts(ui.ctx());
                app.geometry_properties(ui);
                assert!(
                    ui.min_rect().right() <= width + 1.,
                    "geometry overflow {:?}",
                    ui.min_rect()
                );
            },
        );
        output.textures_delta.clear();
        output
    }
    fn field(output: &egui::FullOutput, label: &str) -> Pos2 {
        let nodes = &output
            .platform_output
            .accesskit_update
            .as_ref()
            .unwrap()
            .nodes;
        let (_, node) = nodes
            .iter()
            .find(|(_, node)| {
                node.labelled_by().iter().any(|id| {
                    nodes.iter().any(|(other, caption)| {
                        other == id && caption.value().or_else(|| caption.label()) == Some(label)
                    })
                })
            })
            .unwrap_or_else(|| panic!("missing field {label}"));
        let r = node.bounds().unwrap();
        Pos2::new((r.x0 + r.x1) as f32 / 2., (r.y0 + r.y1) as f32 / 2.)
    }
    fn click(app: &mut WireframeApp, ctx: &egui::Context, at: Pos2) {
        for pressed in [true, false] {
            frame(
                app,
                ctx,
                300.,
                vec![
                    egui::Event::PointerMoved(at),
                    egui::Event::PointerButton {
                        pos: at,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: Modifiers::NONE,
                    },
                ],
            );
        }
        frame(app, ctx, 300., vec![]);
    }
    fn key(key: egui::Key) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }
    }
    #[test]
    fn numeric_draft_escape_commit_and_undo_preserve_selection() {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let mut app = WireframeApp::workspace(&ctx, None);
        app.editor.add(Kind::Button, 50., 50.);
        let before = app.editor.document.clone();
        let undo = app.editor.undo_name().map(str::to_owned);
        let out = frame(&mut app, &ctx, 300., vec![]);
        click(&mut app, &ctx, field(&out, "X"));
        frame(&mut app, &ctx, 300., vec![egui::Event::Text("200".into())]);
        assert_eq!(app.editor.document, before, "typing is only a draft");
        frame(&mut app, &ctx, 300., vec![key(egui::Key::Escape)]);
        assert_eq!(app.editor.document, before);
        assert_eq!(app.editor.undo_name(), undo.as_deref());
        let out = frame(&mut app, &ctx, 300., vec![]);
        assert_eq!(
            app.editor.document, before,
            "cancel remains discarded on the following frame"
        );
        click(&mut app, &ctx, field(&out, "X"));
        frame(&mut app, &ctx, 300., vec![egui::Event::Text("200".into())]);
        frame(&mut app, &ctx, 300., vec![key(egui::Key::Enter)]);
        assert_eq!(app.editor.selection_geometry().unwrap().bounds.x, 200.);
        assert_eq!(app.editor.undo_name(), Some("Edit geometry"));
        let after = app.editor.document.clone();
        app.editor.undo();
        assert_eq!(app.editor.document, before);
        app.editor.redo();
        assert_eq!(app.editor.document, after);
    }
    #[test]
    fn hidden_group_position_field_moves_without_using_visible_size_as_resize_target() {
        use wireframe::document::{Node, Point, Size};
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let mut app = WireframeApp::workspace(&ctx, None);
        let mut p = Project {
            next_id: 10,
            ..Project::default()
        };
        let plain = Node::control(3, "rectangle", Point::default(), Size::fixed(80., 40.));
        let mut marked = Node::control(4, "rectangle", Point::new(300., 0.), Size::fixed(80., 40.));
        marked.markup = true;
        p.pages[0].content.nodes =
            vec![Node::group(2, Point::new(100., 100.), vec![plain, marked])];
        app.editor = Editor::new(p);
        app.editor.select(2, false);
        app.editor.set_markup_visible(false);
        let before = app.editor.document.clone();
        let out = frame(&mut app, &ctx, 300., vec![]);
        click(&mut app, &ctx, field(&out, "X"));
        frame(&mut app, &ctx, 300., vec![egui::Event::Text("200".into())]);
        frame(&mut app, &ctx, 300., vec![key(Key::Enter)]);
        assert_eq!(
            app.editor.bounds(),
            Some(Bounds::new(200., 100., 380., 40.))
        );
        assert_eq!(app.editor.selection_geometry().unwrap().bounds.w, 80.);
        let moved = app.editor.document.clone();
        let out = frame(&mut app, &ctx, 300., vec![]);
        click(&mut app, &ctx, field(&out, "Width"));
        frame(&mut app, &ctx, 300., vec![egui::Event::Text("160".into())]);
        frame(&mut app, &ctx, 300., vec![key(Key::Enter)]);
        assert_eq!(
            app.editor.bounds(),
            Some(Bounds::new(200., 100., 160., 40.))
        );
        assert_eq!(app.editor.selection_geometry().unwrap().bounds.w, 34.);
        app.editor.undo();
        assert_eq!(app.editor.document, moved);
        app.editor.undo();
        assert_eq!(app.editor.document, before);
    }
    #[test]
    fn geometry_rows_fit_narrow_widths_and_do_not_commit_canvas_preview() {
        for width in [220., 300., 400.] {
            let ctx = egui::Context::default();
            ctx.enable_accesskit();
            let mut app = WireframeApp::workspace(&ctx, None);
            app.editor.add(Kind::Button, 50., 50.);
            let before = app.editor.document.clone();
            app.editor.begin("Resize");
            let mut bounds = app.editor.selection_geometry().unwrap().bounds;
            bounds.w *= 2.;
            app.editor.update_selection_bounds(bounds);
            let output = frame(&mut app, &ctx, width, vec![]);
            for label in ["X", "Y", "Width", "Height"] {
                field(&output, label);
            }
            assert!((field(&output, "X").x - field(&output, "Width").x).abs() < 0.1);
            assert!((field(&output, "Y").x - field(&output, "Height").x).abs() < 0.1);
            app.editor.cancel();
            assert_eq!(
                app.editor.document, before,
                "inspector must leave canvas transaction open"
            );
        }
    }
}
