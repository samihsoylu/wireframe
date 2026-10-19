// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Presentation navigation shares editor visuals without editing its selection state.
use super::*;

impl WireframeApp {
    pub(super) fn presentation_navigator(&mut self, ui: &mut egui::Ui) {
        ui.spacing_mut().item_spacing.y = 0.;
        let width = ui.available_width();
        egui::Frame::NONE
            .fill(Color32::WHITE)
            .inner_margin(egui::Margin {
                left: 16,
                right: 12,
                top: 6,
                bottom: 6,
            })
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.x = 0.;
                ui.spacing_mut().button_padding = Vec2::new(4., 3.);
                ui.horizontal(|ui| {
                    let button_width = ((width - 72.) / 2.).clamp(28., 64.);
                    // Text metrics snap to physical pixels. Leave a complete pixel
                    // allocation so rounding cannot push the row beyond its pane.
                    let density = ui.ctx().pixels_per_point();
                    let button_width = (button_width * density).floor() / density;
                    for (forward, label) in [(false, "Prev ↑"), (true, "Next ↓")] {
                        let response = ui
                            .add_enabled_ui(
                                self.adjacent_destination(forward, false).is_some(),
                                |ui| {
                                    ui.add_sized(
                                        [button_width, 24.],
                                        egui::Button::new(label).truncate().corner_radius(5),
                                    )
                                },
                            )
                            .inner;
                        if response.clicked() {
                            self.navigate_adjacent(forward, false);
                        }
                        ui.add_space(if forward { 8. } else { 12. });
                    }
                    if chrome::close_button(ui, "Hide Navigator", "Hide Navigator (J)").clicked() {
                        self.presentation_view.session.toggle(Aid::Navigator);
                    }
                });
            });
        crate::app::navigator::view_header(
            ui,
            &mut self.presentation_view.thumbnails,
            chrome::PRESENTATION_HEADING,
        );
        let pages: Vec<_> = self
            .editor
            .document
            .live_pages()
            .map(|p| (p.id, p.name.clone()))
            .collect();
        egui::ScrollArea::vertical()
            .id_salt("presentation-pages")
            .show(ui, |ui| {
                for (id, name) in pages {
                    ui.push_id(id, |ui| {
                        let selected = self.editor.page == id;
                        let thumbnails = self.presentation_view.thumbnails;
                        let (row, _) = ui.allocate_exact_size(
                            Vec2::new(ui.available_width(), if thumbnails { 156. } else { 30. }),
                            Sense::hover(),
                        );
                        let response =
                            ui.interact(row, ui.id().with("presentation-page"), Sense::click());
                        let selected_color = Color32::from_rgb(65, 155, 221);
                        ui.painter().rect_filled(
                            row,
                            0,
                            if selected {
                                selected_color
                            } else {
                                chrome::PANEL
                            },
                        );
                        let caption = if thumbnails {
                            self.paint_page_card(ui, row, self.editor.active_surface(id), &name, 4.)
                                .caption
                        } else {
                            let caption = Rect::from_min_max(
                                row.min + Vec2::new(30., 0.),
                                row.max - Vec2::new(if selected { 32. } else { 12. }, 0.),
                            );
                            ui_text::paint_styled(
                                ui,
                                caption,
                                &name,
                                if selected {
                                    ui_text::CaptionStyle::selected_page()
                                } else {
                                    ui_text::CaptionStyle::regular(12.)
                                },
                                if selected {
                                    Color32::WHITE
                                } else {
                                    Color32::from_gray(40)
                                },
                                false,
                            );
                            caption
                        };
                        response.widget_info(|| {
                            egui::WidgetInfo::selected(
                                egui::WidgetType::SelectableLabel,
                                ui.is_enabled(),
                                selected,
                                &name,
                            )
                        });
                        if response.has_focus() {
                            ui.painter().rect_stroke(
                                row.shrink(1.),
                                0,
                                Stroke::new(1., Color32::from_rgb(35, 100, 155)),
                                StrokeKind::Inside,
                            );
                        }
                        let mut menu_clicked = false;
                        if selected {
                            let menu = Rect::from_center_size(
                                egui::pos2(
                                    if thumbnails {
                                        row.right() - 26.
                                    } else {
                                        row.right() - 20.
                                    },
                                    caption.center().y,
                                ),
                                Vec2::new(24., 24.),
                            );
                            ui.scope_builder(
                                egui::UiBuilder::new().max_rect(menu).layout(
                                    egui::Layout::centered_and_justified(
                                        egui::Direction::LeftToRight,
                                    ),
                                ),
                                |ui| {
                                    let response = egui::containers::menu::MenuButton::from_button(
                                        egui::Button::new("")
                                            .frame(false)
                                            .min_size(Vec2::splat(24.)),
                                    )
                                    .ui(ui, |ui| self.presentation_navigation_menu(ui))
                                    .0;
                                    let color = if thumbnails {
                                        Color32::from_gray(70)
                                    } else {
                                        Color32::WHITE
                                    };
                                    let center = response.rect.center();
                                    ui.painter().add(egui::Shape::line(
                                        vec![
                                            center + Vec2::new(-3., -1.5),
                                            center + Vec2::new(0., 1.5),
                                            center + Vec2::new(3., -1.5),
                                        ],
                                        Stroke::new(0.8, color),
                                    ));
                                    menu_clicked = response.clicked();
                                    response.widget_info(|| {
                                        egui::WidgetInfo::labeled(
                                            egui::WidgetType::Button,
                                            ui.is_enabled(),
                                            "Versions and navigation",
                                        )
                                    });
                                    response.on_hover_text("Versions and navigation");
                                },
                            );
                        }
                        response.context_menu(|ui| self.presentation_navigation_menu(ui));
                        if response.clicked() && !menu_clicked {
                            response.request_focus();
                            self.navigate_page(id);
                        }
                    });
                }
            });
    }

    fn presentation_navigation_menu(&mut self, ui: &mut egui::Ui) {
        self.presentation_versions(ui);
        ui.separator();
        for (forward, label) in [(false, "Back"), (true, "Forward")] {
            if ui
                .add_enabled(
                    self.history_destination(forward).is_some(),
                    egui::Button::new(label),
                )
                .clicked()
            {
                self.navigate_history(forward);
                ui.close();
            }
        }
        if ui.button("Presentation settings…").clicked() {
            self.presentation_view.settings_open = true;
            ui.close();
        }
    }
}
