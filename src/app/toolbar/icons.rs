// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Original, resolution-independent strokes for editor commands.
use eframe::egui::{self, Color32, Pos2, Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2};

#[derive(Clone, Copy)]
pub(super) enum Icon {
    Pages,
    Add,
    NewLibrary,
    Undo,
    Redo,
    ZoomOut,
    ZoomIn,
    ActualSize,
    Fit,
    Library,
    Inspector,
    Comments,
    Present,
    Trash,
}

pub(super) fn button(
    ui: &mut Ui,
    icon: Icon,
    label: &str,
    help: &str,
    enabled: bool,
    selected: Option<bool>,
) -> Response {
    let response = ui
        .add_enabled_ui(enabled, |ui| {
            let size = if matches!(icon, Icon::Trash) {
                Vec2::new(24., 25.)
            } else {
                Vec2::splat(20.)
            };
            let (rect, response) = ui.allocate_exact_size(size, Sense::click());
            if response.has_focus() {
                ui.painter().rect_stroke(
                    rect,
                    3.,
                    ui.visuals().selection.stroke,
                    StrokeKind::Inside,
                );
            }
            let color = if response.is_pointer_button_down_on() && response.contains_pointer() {
                ui.visuals().widgets.active.fg_stroke.color
            } else if selected == Some(true) {
                ui.visuals().selection.stroke.color
            } else {
                Color32::WHITE
            };
            draw(ui, rect.center(), icon, color, selected == Some(true));
            response.widget_info(|| match selected {
                Some(value) => egui::WidgetInfo::selected(
                    egui::WidgetType::Button,
                    ui.is_enabled(),
                    value,
                    label,
                ),
                None => egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label),
            });
            response
        })
        .inner;
    response.on_hover_text(help)
}

fn draw(ui: &Ui, center: Pos2, icon: Icon, color: Color32, selected: bool) {
    let p = ui.painter();
    let stroke = Stroke::new(1.2, color);
    let point = |x, y| center + Vec2::new(x, y);
    let line = |a: (f32, f32), b: (f32, f32)| {
        p.line_segment([point(a.0, a.1), point(b.0, b.1)], stroke);
    };
    let rectangle = |a: (f32, f32), b: (f32, f32), radius| {
        p.rect_stroke(
            Rect::from_min_max(point(a.0, a.1), point(b.0, b.1)),
            radius,
            stroke,
            StrokeKind::Inside,
        );
    };
    match icon {
        Icon::Trash => {
            rectangle((-6., -5.), (6., 9.), 1.);
            line((-8., -7.), (8., -7.));
            line((-3., -10.), (3., -10.));
            for x in [-3., 0., 3.] {
                line((x, -2.), (x, 6.));
            }
        }
        Icon::ZoomOut | Icon::ZoomIn | Icon::ActualSize | Icon::Fit => {
            // Circle strokes grow outward; keep the outer lens radius at eight
            // points and give the handle its own, heavier rounded silhouette.
            let stroke = Stroke::new(1., color);
            let line = |a: (f32, f32), b: (f32, f32)| {
                p.line_segment([point(a.0, a.1), point(b.0, b.1)], stroke);
            };
            p.circle_stroke(point(1.5, -2.5), 7., stroke);
            capsule(p, point(-4., 3.), point(-8.5, 7.5), 1., color);
            match icon {
                Icon::ZoomOut => line((-3., -2.5), (6., -2.5)),
                Icon::ZoomIn => {
                    line((-3., -2.5), (6., -2.5));
                    line((1.5, -7.), (1.5, 2.));
                }
                Icon::ActualSize => {
                    // Hand-drawn digits keep this command independent of UI fonts.
                    for x in [-1., 4.5] {
                        line((x - 2., -3.5), (x, -5.5));
                        line((x, -5.5), (x, 1.));
                    }
                    p.circle_filled(point(1.5, -3.5), 0.65, color);
                    p.circle_filled(point(1.5, 0.), 0.65, color);
                }
                Icon::Fit => {
                    // Four compact square corners leave a clear central opening.
                    for (x, y, sx, sy) in [
                        (-2., -6., 1., 1.),
                        (5., -6., -1., 1.),
                        (-2., 1., 1., -1.),
                        (5., 1., -1., -1.),
                    ] {
                        for (dx, dy) in [(3. * sx, sy), (sx, 3. * sy)] {
                            p.rect_filled(
                                Rect::from_two_pos(point(x, y), point(x + dx, y + dy)),
                                0.,
                                color,
                            );
                        }
                    }
                }
                _ => unreachable!(),
            }
        }
        Icon::Undo | Icon::Redo => {
            let stroke = Stroke::new(1., color);
            let direction = if matches!(icon, Icon::Undo) { 1. } else { -1. };
            let arc: Vec<_> = (0..=24)
                .map(|i| {
                    let angle = (-90. + i as f32 * 11.25).to_radians();
                    point(direction * 7.5 * angle.cos(), 1. + 7.5 * angle.sin())
                })
                .collect();
            p.add(egui::Shape::line(arc, stroke));
            p.add(egui::Shape::convex_polygon(
                vec![point(-8. * direction, -6.), point(0., -11.), point(0., -1.)],
                color,
                Stroke::NONE,
            ));
        }
        Icon::Pages => {
            if selected {
                for (a, b) in [
                    ((-9., -9.), (9., -5.)),
                    ((-9., -3.), (-4., 9.)),
                    ((-2., -3.), (9., 9.)),
                ] {
                    p.rect_filled(
                        Rect::from_min_max(point(a.0, a.1), point(b.0, b.1)),
                        0.,
                        color,
                    );
                }
                return;
            }
            rectangle((-9., -9.), (9., 9.), 0.);
            line((-9., -4.), (9., -4.));
            line((-3., -4.), (-3., 9.));
        }
        Icon::NewLibrary => {
            let rear: Vec<_> = (0..=24)
                .map(|i| {
                    let angle = (90. + i as f32 * 11.25).to_radians();
                    point(-3. + 6. * angle.cos(), -3. + 6. * angle.sin())
                })
                .collect();
            p.add(egui::Shape::line(rear, stroke));
            p.circle_stroke(point(3., 3.), 6., stroke);
            line((0., 3.), (6., 3.));
            line((3., 0.), (3., 6.));
        }
        Icon::Add => {
            line((-9., 0.), (9., 0.));
            line((0., -9.), (0., 9.));
        }
        Icon::Library => {
            rectangle((-9., -9.), (9., 9.), 1.);
            rectangle((-5.5, -5.5), (-0.5, -0.5), 0.);
            p.circle_stroke(point(4., -3.), 2.2, stroke);
            line((-5., 5.), (4., 5.));
            line((4., 5.), (4., 1.5));
            line((4., 5.), (0.5, 1.5));
        }
        Icon::Comments => {
            p.add(egui::Shape::closed_line(
                vec![
                    point(-9., -8.),
                    point(9., -8.),
                    point(9., 5.),
                    point(-1., 5.),
                    point(-6., 10.),
                    point(-6., 5.),
                    point(-9., 5.),
                ],
                stroke,
            ));
        }
        Icon::Inspector => {
            if selected {
                p.circle_filled(center, 9., color);
                let ink = super::chrome::TOOLBAR;
                p.circle_filled(point(0., -4.5), 1., ink);
                p.line_segment([point(0., -1.), point(0., 5.)], Stroke::new(1.5, ink));
                return;
            }
            p.circle_stroke(center, 9., stroke);
            p.circle_filled(point(0., -4.5), 1., color);
            line((0., -1.), (0., 5.));
        }
        Icon::Present => {
            rectangle((-9., -9.), (9., 9.), 2.);
            p.add(egui::Shape::closed_line(
                vec![point(-3., -5.), point(5., 0.), point(-3., 5.)],
                stroke,
            ));
        }
    }
}

fn capsule(p: &egui::Painter, start: Pos2, end: Pos2, radius: f32, color: Color32) {
    let angle = (end - start).angle();
    let half_turn = std::f32::consts::PI;
    let points = [
        (start, angle + half_turn * 0.5),
        (end, angle - half_turn * 0.5),
    ]
    .into_iter()
    .flat_map(|(center, from)| {
        (0..=8).map(move |step| center + Vec2::angled(from + half_turn * step as f32 / 8.) * radius)
    })
    .collect();
    p.add(egui::Shape::convex_polygon(points, color, Stroke::NONE));
}

pub(super) fn back_button(ui: &mut Ui) -> Response {
    let label = "Back to Wireframes";
    let label_width = ui
        .painter()
        .layout_no_wrap(
            label.into(),
            egui::FontId::proportional(12.),
            Color32::from_gray(235),
        )
        .size()
        .x;
    let (rect, response) = ui.allocate_exact_size(
        Vec2::new((label_width + 24. + 8. + 5.).ceil(), 20.),
        Sense::click(),
    );
    let fill = if response.hovered() || response.has_focus() {
        Color32::from_gray(125)
    } else {
        Color32::from_gray(105)
    };
    ui.painter().rect_filled(rect, 3., fill);
    let center = rect.left_center() + Vec2::new(16., 0.);
    ui.painter().add(egui::Shape::line(
        vec![
            center + Vec2::new(2., -4.),
            center - Vec2::new(2., 0.),
            center + Vec2::new(2., 4.),
        ],
        Stroke::new(1.4, Color32::from_gray(235)),
    ));
    wireframe::ui_text::paint(
        ui,
        Rect::from_min_max(rect.min + Vec2::new(25., 0.), rect.max - Vec2::new(12., 0.)),
        label,
        12.,
        Color32::from_gray(235),
        false,
    );
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label)
    });
    response
}

pub(super) fn search(ui: &Ui, rect: Rect) {
    let center = rect.center() - Vec2::new(1., 1.);
    let stroke = Stroke::new(1.2, Color32::from_gray(235));
    ui.painter().circle_stroke(center, 4., stroke);
    ui.painter()
        .line_segment([center + Vec2::splat(3.), center + Vec2::splat(6.)], stroke);
}
