// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Page-navigation presentation; document mutations remain in the editor.
use super::*;
mod drag;
mod selection;
#[cfg(test)]
mod tests;
pub(super) use selection::Navigator;

impl WireframeApp {
    fn navigator_header(&mut self, ui: &mut egui::Ui) {
        if view_header(
            ui,
            &mut self.thumbnails,
            chrome::panel_heading(ui, &self.menus),
        ) {
            self.navigator.scroll_to = Some(self.editor.page);
        }
    }
    pub(super) fn sidebar(&mut self, ui: &mut egui::Ui) {
        chrome::prepare_panel_resize(
            ui,
            egui::Id::new("navigator"),
            chrome::PanelEdge::Left,
            160. ..=300.,
            self.image_crop.is_none() && !self.canvas_input_blocked(),
        );
        egui::Panel::left("navigator")
            .resizable(true)
            .frame(egui::Frame::NONE.fill(Color32::from_gray(238)))
            .default_size(192.)
            .size_range(160.0..=300.0)
            .show(ui, |ui| {
                if self.image_crop.is_some() {
                    ui.disable();
                }
                ui.spacing_mut().item_spacing.y = 0.;
                if self.editor.in_source_workspace() {
                    self.navigator.focus_ids.clear();
                    self.navigator_tools(ui);
                    self.source_navigator(ui);
                    return;
                }
                // A retained/removed library can return the editor to pages.
                // Its former resource selection and pending focus are no longer active.
                self.source_navigation = Default::default();
                self.navigator_header(ui);
                self.navigator_tools(ui);
                let owned_focus = self.navigator.focused(ui.ctx());
                self.navigator.focus_ids.clear();
                let tree = wireframe::page_tree::PageTree::new(&self.editor.document);
                let current_changed = self.navigator.sync(
                    &tree,
                    self.editor.page,
                    self.editor.duplicated_page_selection(),
                );
                if owned_focus && current_changed {
                    self.navigator.focus_to = Some(self.editor.page);
                }
                // Navigation needs metadata only; keep page contents in the document.
                let pages: Vec<_> = tree
                    .visible(&self.navigator.collapsed)
                    .into_iter()
                    .map(|row| {
                        let page = self.editor.document.live_page(row.id).unwrap();
                        let surface = self.editor.active_surface(page.id);
                        let caption = page
                            .alternates
                            .iter()
                            .find(|a| a.id == surface)
                            .map_or_else(
                                || page.name.clone(),
                                |a| format!("{} ({})", page.name, a.name),
                            );
                        (page.id, surface, caption, row.depth, row.has_children)
                    })
                    .collect();
                egui::ScrollArea::vertical()
                    .id_salt("page-navigator")
                    .max_height(ui.available_height())
                    .show(ui, |ui| {
                        let mut drag_rows = Vec::new();
                        if self.image_crop.is_some() {
                            ui.disable();
                        }
                        ui.spacing_mut().item_spacing.y = 0.;
                        for (owner, surface, caption, depth, has_children) in pages {
                            let active =
                                self.editor.definition.is_none() && self.editor.page == owner;
                            let selected = self.editor.definition.is_none()
                                && self.navigator.selected.contains(&owner);
                            let indent =
                                (depth as f32 * 16.).min((ui.available_width() - 100.).max(0.));
                            if self.thumbnails {
                                let width = ui.available_width();
                                let (r, _) = ui.allocate_exact_size(
                                    Vec2::new(width, 150. - indent),
                                    Sense::hover(),
                                );
                                let response = ui.interact(
                                    r,
                                    ui.id().with(("page-row", owner)),
                                    Sense::click_and_drag(),
                                );
                                drag_rows.push((owner, r));
                                self.navigator_drag_start(ui, &tree, owner, &caption, &response);
                                self.navigator.focus_ids.insert(response.id);
                                self.navigator_reveal(ui, owner, r, &response);
                                if ui.is_rect_visible(r) {
                                    ui.painter().rect_filled(
                                        r,
                                        0,
                                        if active && selected {
                                            Color32::from_rgb(65, 155, 221)
                                        } else if selected {
                                            Color32::from_rgb(179, 218, 245)
                                        } else {
                                            Color32::from_gray(204)
                                        },
                                    );
                                    let card = self.paint_page_card(
                                        ui,
                                        Rect::from_min_max(r.min + Vec2::new(indent, 0.), r.max),
                                        surface,
                                        &caption,
                                        14.,
                                    );
                                    if response.has_focus() {
                                        ui.painter().rect_stroke(
                                            card.paper.expand(2.),
                                            0,
                                            Stroke::new(1., Color32::from_rgb(35, 100, 155)),
                                            StrokeKind::Inside,
                                        );
                                    }
                                }
                                response.widget_info(|| {
                                    egui::WidgetInfo::selected(
                                        egui::WidgetType::SelectableLabel,
                                        ui.is_enabled(),
                                        selected,
                                        &caption,
                                    )
                                });
                                let toggled = self.page_disclosure(
                                    ui,
                                    &tree,
                                    owner,
                                    &caption,
                                    r,
                                    indent,
                                    has_children,
                                );
                                self.page_context_menu(ui, &response, owner);
                                if response.clicked() && !toggled {
                                    response.request_focus();
                                    self.end_pointer_gestures();
                                    let page = self.navigator.select(
                                        &tree,
                                        owner,
                                        self.editor.page,
                                        ui.input(|i| i.modifiers),
                                    );
                                    self.editor.switch_page(page);
                                }
                            } else {
                                let (r, _) = ui.allocate_exact_size(
                                    Vec2::new(ui.available_width(), 30.),
                                    Sense::hover(),
                                );
                                let response = ui.interact(
                                    r,
                                    ui.id().with(("page-row", owner)),
                                    Sense::click_and_drag(),
                                );
                                drag_rows.push((owner, r));
                                self.navigator_drag_start(ui, &tree, owner, &caption, &response);
                                self.navigator.focus_ids.insert(response.id);
                                self.navigator_reveal(ui, owner, r, &response);
                                let color = if active && selected {
                                    Color32::from_rgb(65, 155, 221)
                                } else if selected {
                                    Color32::from_rgb(179, 218, 245)
                                } else {
                                    Color32::from_gray(238)
                                };
                                ui.painter().rect_filled(r, 0, color);
                                ui_text::paint(
                                    ui,
                                    Rect::from_min_max(
                                        r.min + Vec2::new(30. + indent, 0.),
                                        r.max - Vec2::new(12., 0.),
                                    ),
                                    &caption,
                                    12.,
                                    if active && selected {
                                        Color32::WHITE
                                    } else {
                                        Color32::from_gray(40)
                                    },
                                    false,
                                );
                                response.widget_info(|| {
                                    egui::WidgetInfo::selected(
                                        egui::WidgetType::SelectableLabel,
                                        ui.is_enabled(),
                                        selected,
                                        &caption,
                                    )
                                });
                                let toggled = self.page_disclosure(
                                    ui,
                                    &tree,
                                    owner,
                                    &caption,
                                    r,
                                    indent,
                                    has_children,
                                );
                                self.page_context_menu(ui, &response, owner);
                                if response.clicked() && !toggled {
                                    response.request_focus();
                                    self.end_pointer_gestures();
                                    let page = self.navigator.select(
                                        &tree,
                                        owner,
                                        self.editor.page,
                                        ui.input(|i| i.modifiers),
                                    );
                                    self.editor.switch_page(page);
                                }
                            }
                        }
                        self.navigator_drag_finish(ui, &tree, &drag_rows);
                    });
            });
        ui.memory_mut(|m| {
            if let Some(id) = m.focused()
                && self.navigator.focus_ids.contains(&id)
            {
                m.set_focus_lock_filter(
                    id,
                    egui::EventFilter {
                        horizontal_arrows: true,
                        vertical_arrows: true,
                        escape: true,
                        tab: true,
                    },
                );
            }
        });
    }
    fn navigator_reveal(
        &mut self,
        ui: &egui::Ui,
        owner: Id,
        rect: Rect,
        response: &egui::Response,
    ) {
        if self.navigator.scroll_to == Some(owner) {
            ui.scroll_to_rect(rect, None);
            self.navigator.scroll_to = None;
        }
        if self.navigator.focus_to == Some(owner) {
            response.request_focus();
            self.navigator.focus_to = None;
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn page_disclosure(
        &mut self,
        ui: &mut egui::Ui,
        tree: &wireframe::page_tree::PageTree,
        owner: Id,
        caption: &str,
        row: Rect,
        indent: f32,
        has_children: bool,
    ) -> bool {
        if !has_children {
            return false;
        }
        let rect = Rect::from_center_size(
            egui::pos2(row.left() + 20. + indent, row.center().y),
            Vec2::new(18., 24.),
        );
        let response = ui.interact(
            rect,
            ui.id().with(("page-disclosure", owner)),
            Sense::click(),
        );
        self.navigator.focus_ids.insert(response.id);
        let closed = self.navigator.collapsed.contains(&owner);
        response.widget_info(|| {
            egui::WidgetInfo::labeled(
                egui::WidgetType::Button,
                ui.is_enabled(),
                format!("{} {caption}", if closed { "Expand" } else { "Collapse" }),
            )
        });
        let center = rect.center();
        let points = if closed {
            vec![
                center + Vec2::new(-2., -3.),
                center + Vec2::new(1., 0.),
                center + Vec2::new(-2., 3.),
            ]
        } else {
            vec![
                center + Vec2::new(-3., -2.),
                center + Vec2::new(0., 1.),
                center + Vec2::new(3., -2.),
            ]
        };
        ui.painter().add(egui::Shape::line(
            points,
            Stroke::new(1.5, Color32::from_gray(155)),
        ));
        if response.clicked() {
            response.request_focus();
            self.end_pointer_gestures();
            let page = self.navigator.toggle(tree, owner, self.editor.page);
            self.editor.switch_page(page);
            true
        } else {
            false
        }
    }

    fn navigator_tools(&mut self, ui: &mut egui::Ui) {
        let max_height = (ui.available_height() * 0.45).max(60.);
        let height_id = ui.id().with("navigator-tools-content-height");
        let measured_height = ui
            .data(|data| data.get_temp::<f32>(height_id))
            .unwrap_or(max_height);
        egui::Panel::bottom("navigator-tools")
            // ScrollArea reports the natural content height, even when clipped.
            // Reserve it on the next frame so expanding sections can grow too.
            .exact_size(measured_height.min(max_height) + 16.)
            .frame(
                egui::Frame::NONE
                    .fill(Color32::from_gray(238))
                    .inner_margin(8),
            )
            .show(ui, |ui| {
                let scroll = egui::ScrollArea::vertical()
                    .id_salt("navigator-tools-scroll")
                    .max_height(max_height)
                    .show(ui, |ui| {
                        self.project_versions_menu(ui);
                        ui.separator();
                        if !self.editor.in_source_workspace() {
                            self.components_sidebar(ui);
                        }
                        egui::CollapsingHeader::new("Layers")
                            .default_open(false)
                            .show(ui, |ui| {
                                if self.image_crop.is_some() {
                                    ui.disable();
                                }
                                let controls = self.editor.current().controls.clone();
                                for c in controls.iter().rev() {
                                    let label = format!(
                                        "{}{}",
                                        if c.locked { "[locked] " } else { "" },
                                        self.editor.selection_label(c.id)
                                    );
                                    if ui_text::selectable(
                                        ui,
                                        self.editor.selection.contains(&c.id),
                                        &label,
                                        ui.available_width(),
                                    )
                                    .clicked()
                                    {
                                        self.editor.select(c.id, ui.input(|i| i.modifiers.shift));
                                    }
                                }
                            });
                    });
                ui.data_mut(|data| data.insert_temp(height_id, scroll.content_size.y));
                if (scroll.content_size.y - measured_height).abs() > 0.5 {
                    ui.ctx().request_repaint();
                }
            });
    }
}

pub(super) struct PageCard {
    pub paper: Rect,
    pub preview: Rect,
    pub caption: Rect,
}
impl PageCard {
    fn new(row: Rect) -> Self {
        let paper = Rect::from_min_max(row.min + Vec2::new(30., 6.), row.max - Vec2::new(16., 6.));
        let preview = Rect::from_min_max(paper.min, paper.max - Vec2::new(0., 28.));
        let caption = Rect::from_min_max(
            egui::pos2(paper.left() + 8., preview.bottom()),
            paper.max - Vec2::new(8., 4.),
        );
        Self {
            paper,
            preview,
            caption,
        }
    }
}

fn view_button(ui: &mut egui::Ui, tiles: bool, selected: bool) -> egui::Response {
    let label = if tiles { "Thumbnails" } else { "List" };
    let (rect, response) = ui.allocate_exact_size(Vec2::new(24., 28.), Sense::click());
    if response.hovered() || response.has_focus() {
        let visuals = ui.style().interact(&response);
        ui.painter().rect(
            rect,
            2,
            visuals.bg_fill,
            visuals.bg_stroke,
            StrokeKind::Inside,
        );
    }
    let color = if !ui.is_enabled() {
        Color32::GRAY
    } else if selected {
        Color32::from_rgb(65, 155, 221)
    } else {
        Color32::from_gray(50)
    };
    let stroke = Stroke::new(1., color);
    let center = rect.center();
    if tiles {
        ui.painter().rect_stroke(
            Rect::from_center_size(center, Vec2::splat(11.)),
            0,
            stroke,
            StrokeKind::Inside,
        );
    } else {
        for y in [-5., 0., 5.] {
            ui.painter().line_segment(
                [center + Vec2::new(-6., y), center + Vec2::new(6., y)],
                stroke,
            );
        }
    }
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Button, ui.is_enabled(), selected, label)
    });
    response.on_hover_text(if tiles {
        "Show page thumbnails"
    } else {
        "Show page list"
    })
}

pub(super) fn view_header(ui: &mut egui::Ui, thumbnails: &mut bool, heading: Color32) -> bool {
    let mut changed = false;
    let (row, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 36.), Sense::hover());
    ui.painter().rect_filled(row, 0, heading);
    let label = Rect::from_min_max(row.min + Vec2::new(16., 0.), row.max - Vec2::new(60., 0.));
    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(label)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
        |ui| {
            ui.label(
                egui::RichText::new("Wireframes").font(wireframe::fonts::panel_heading_font()),
            );
        },
    );
    let controls = Rect::from_min_max(
        egui::pos2(row.right() - 56., row.top()),
        row.max - Vec2::new(8., 0.),
    );
    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(controls)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
        |ui| {
            ui.spacing_mut().item_spacing.x = 0.;
            if view_button(ui, true, *thumbnails).clicked() {
                *thumbnails = true;
                changed = true;
            }
            if view_button(ui, false, !*thumbnails).clicked() {
                *thumbnails = false;
                changed = true;
            }
        },
    );
    changed
}

impl WireframeApp {
    pub(super) fn paint_page_card(
        &self,
        ui: &egui::Ui,
        row: Rect,
        surface: Id,
        caption: &str,
        vertical_padding: f32,
    ) -> PageCard {
        let card = PageCard::new(row);
        ui.painter().add(
            egui::epaint::Shadow {
                offset: [0, 1],
                blur: 5,
                spread: 0,
                color: Color32::from_black_alpha(65),
            }
            .as_shape(card.paper, 0),
        );
        ui.painter().rect_filled(card.paper, 0, Color32::WHITE);
        let paper = card.preview;
        let page = self.editor.rendered_page(surface).unwrap_or(Page {
            id: surface,
            name: caption.to_owned(),
            notes: String::new(),
            controls: vec![],
        });
        let bounds = scene::page_bounds(&page);
        let scale = ((paper.width() - 28.) / bounds.w.max(1.))
            .min((paper.height() - 2. * vertical_padding) / bounds.h.max(1.));
        let origin = paper.center()
            - Vec2::new(bounds.w, bounds.h) * scale / 2.
            - Vec2::new(bounds.x, bounds.y) * scale;
        let painter = ui.painter().with_clip_rect(paper);
        if let Ok(drawing) = self.editor.rendered_scene(page.id) {
            Self::paint_scene(&painter, &drawing, origin, scale);
        }
        ui_text::paint(ui, card.caption, caption, 12., Color32::from_gray(55), true);
        card
    }
}
