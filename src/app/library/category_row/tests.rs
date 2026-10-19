// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use std::collections::BTreeMap;
type Nodes = BTreeMap<String, (egui::accesskit::NodeId, Rect, bool)>;

fn frame(
    ctx: &egui::Context,
    width: f32,
    selected: &[CatalogCategory],
    events: Vec<egui::Event>,
    enabled: bool,
) -> (Nodes, Option<(CatalogCategory, bool)>, f32) {
    let mut chosen = None;
    let mut height = 0.;
    let output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(width, 400.))),
            events,
            ..Default::default()
        },
        |ui| {
            if !enabled {
                ui.disable();
            }
            let start = ui.cursor().top();
            chosen = show(ui, selected);
            height = ui.min_rect().bottom() - start;
        },
    );
    let nodes = output
        .platform_output
        .accesskit_update
        .as_ref()
        .unwrap()
        .nodes
        .iter()
        .filter_map(|(id, n)| {
            let name = n.label()?;
            let b = n.bounds()?;
            Some((
                name.to_owned(),
                (
                    *id,
                    Rect::from_min_max(
                        egui::pos2(b.x0 as f32, b.y0 as f32),
                        egui::pos2(b.x1 as f32, b.y1 as f32),
                    ),
                    n.is_disabled(),
                ),
            ))
        })
        .collect();
    output.drop_without_applying_deltas();
    (nodes, chosen, height)
}

fn setup() -> egui::Context {
    let ctx = egui::Context::default();
    wireframe::fonts::install(&ctx);
    ctx.enable_accesskit();
    ctx.global_style_mut(|s| {
        s.animation_time = 0.;
        s.scroll_animation = egui::style::ScrollAnimation::none();
    });
    ctx
}
fn action(id: egui::accesskit::NodeId, action: egui::accesskit::Action) -> egui::Event {
    egui::Event::AccessKitActionRequest(egui::accesskit::ActionRequest {
        action,
        target_tree: egui::accesskit::TreeId::ROOT,
        target_node: id,
        data: None,
    })
}

#[test]
fn centered_category_row_retains_metrics_and_spacing_at_wide_and_narrow_widths() {
    let ctx = setup();
    let mut original = None;
    for width in [1556., 920.] {
        frame(&ctx, width, &[CatalogCategory::All], vec![], true);
        let (nodes, _, height) = frame(&ctx, width, &[CatalogCategory::All], vec![], true);
        assert!((height - 30.).abs() < 0.1, "row height {height}");
        let rects: Vec<_> = CatalogCategory::ALL
            .iter()
            .map(|c| nodes[c.name()].1)
            .collect();
        for r in &rects {
            assert_eq!(r.height(), 24.);
        }
        for pair in rects.windows(2) {
            assert!((pair[1].left() - pair[0].right() - 4.).abs() < 0.1);
        }
        let center = (rects[0].left() + rects.last().unwrap().right()) / 2.;
        assert!(
            (center - width / 2.).abs() <= 0.6,
            "center {center}, width {width}"
        );
        let widths: Vec<_> = rects.iter().map(Rect::width).collect();
        if let Some(original) = &original {
            assert_eq!(&widths, original);
        } else {
            original = Some(widths);
        }
    }
}

#[test]
fn accessible_activation_keeps_shift_and_disabled_guard() {
    let ctx = setup();
    let selected = [CatalogCategory::All];
    let (nodes, _, _) = frame(&ctx, 920., &selected, vec![], true);
    let id = nodes["Buttons"].0;
    let (_, chosen, _) = frame(
        &ctx,
        920.,
        &selected,
        vec![action(id, egui::accesskit::Action::Click)],
        true,
    );
    assert_eq!(chosen, Some((CatalogCategory::Buttons, false)));
    let (_, chosen, _) = frame(
        &ctx,
        920.,
        &selected,
        vec![
            egui::Event::ModifiersChanged(Modifiers::SHIFT),
            action(id, egui::accesskit::Action::Click),
        ],
        true,
    );
    assert_eq!(chosen, Some((CatalogCategory::Buttons, true)));
    let (nodes, chosen, _) = frame(
        &ctx,
        920.,
        &selected,
        vec![action(id, egui::accesskit::Action::Click)],
        false,
    );
    assert_eq!(chosen, None);
    assert!(nodes.values().all(|(_, _, disabled)| *disabled));
}

#[test]
fn pointer_click_uses_its_shift_state_even_if_released_before_the_frame() {
    let ctx = setup();
    let selected = [CatalogCategory::Buttons];
    frame(&ctx, 920., &selected, vec![], true);
    let (nodes, _, _) = frame(&ctx, 920., &selected, vec![], true);
    let pos = nodes["Components"].1.center();
    let event = |pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Modifiers::SHIFT,
    };
    frame(
        &ctx,
        920.,
        &selected,
        vec![egui::Event::PointerMoved(pos), event(true)],
        true,
    );
    let (_, chosen, _) = frame(
        &ctx,
        920.,
        &selected,
        vec![event(false), egui::Event::ModifiersChanged(Modifiers::NONE)],
        true,
    );
    assert_eq!(chosen, Some((CatalogCategory::Components, true)));
}

#[test]
fn focusing_overflow_category_reveals_it_and_keyboard_activates_it() {
    let ctx = setup();
    let selected = [CatalogCategory::Navigation, CatalogCategory::Charts];
    let (nodes, _, _) = frame(&ctx, 500., &selected, vec![], true);
    let id = nodes["Charts"].0;
    assert!(nodes["Charts"].1.right() > 500.);
    frame(
        &ctx,
        500.,
        &selected,
        vec![action(id, egui::accesskit::Action::Focus)],
        true,
    );
    assert_eq!(
        ctx.memory(|m| m.focused()).map(|id| id.accesskit_id()),
        Some(id)
    );
    // The scroll target settles at the end of a pass; inspect the following layout.
    frame(&ctx, 500., &selected, vec![], true);
    let (nodes, _, _) = frame(&ctx, 500., &selected, vec![], true);
    assert!(nodes["Charts"].1.right() <= 500., "{:?}", nodes["Charts"].1);
    let (_, chosen, _) = frame(
        &ctx,
        500.,
        &selected,
        vec![egui::Event::Key {
            key: Key::Enter,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
        true,
    );
    assert_eq!(chosen, Some((CatalogCategory::Charts, false)));
}
