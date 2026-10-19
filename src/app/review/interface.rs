// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use wireframe::review::{Color, Tip};

pub(super) enum Action {
    New,
    Reply(Id),
    Edit(Id),
    Delete(Id),
    Restore(Id),
    Read(Id, bool),
    Like(Id, bool),
    Link(String),
    Post,
    Cancel,
}
impl WireframeApp {
    pub(in crate::app) fn review_panel(&mut self, ui: &mut egui::Ui) {
        chrome::prepare_panel_resize(
            ui,
            egui::Id::new("review-panel"),
            chrome::PanelEdge::Right,
            220. ..=650.,
            !self.canvas_input_blocked(),
        );
        egui::Panel::right("review-panel")
            .resizable(true)
            .default_size(300.)
            .size_range(220. ..=650.)
            .frame(egui::Frame::NONE.fill(Color32::from_gray(238)))
            .show(ui, |ui| self.review_contents(ui));
    }
    pub(in crate::app) fn review_contents(&mut self, ui: &mut egui::Ui) {
        self.sync_reviews();
        let surface = self.editor.surface_id();
        if !self.editor.document.live_review_surface(surface) {
            return;
        }
        ui.add_enabled_ui(
            !self.canvas_input_blocked() && !(self.present && self.presentation_view.settings_open),
            |ui| {
                if self.present {
                    let heading = chrome::PRESENTATION_HEADING;
                    chrome::panel_header(ui, "Comments", heading, 56., |ui| {
                        self.review_header_actions(ui, true)
                    });
                } else {
                    let compact = ui.available_width() < 240.;
                    let heading = chrome::panel_heading(ui, &self.menus);
                    chrome::panel_header(
                        ui,
                        "Comments",
                        heading,
                        if compact { 56. } else { 112. },
                        |ui| self.review_header_actions(ui, compact),
                    );
                }
                let mut action = None;
                let mut composer = self.review_surface().draft.take();
                // Keep both discussion and submission reachable even when Notes
                // leaves a short presentation pane. Fields scroll above pinned actions.
                let desired: f32 = if let Some(draft) = &composer {
                    if draft.text.value.lines().count() > 4 {
                        288.
                    } else {
                        188.
                    }
                } else if self.review_surface().error.is_some() {
                    126.
                } else {
                    48.
                };
                let height = desired.min((ui.available_height() - 64.).max(40.));
                egui::Panel::bottom(egui::Id::new(("review-compose", surface)))
                    .exact_size(height)
                    .frame(egui::Frame::NONE.inner_margin(8))
                    .show(ui, |ui| {
                        if let Some(composer) = &mut composer {
                            self.review_composer(ui, composer, &mut action);
                        } else if ui
                            .add_sized(
                                [ui.available_width(), 30.],
                                egui::Button::new("Add a Comment"),
                            )
                            .clicked()
                        {
                            action = Some(Action::New);
                        }
                        if composer.is_none()
                            && let Some(error) = &self.review_surface().error
                        {
                            ui.colored_label(Color32::DARK_RED, error);
                        }
                    });
                self.review_surface().draft = composer;
                let state = self.review_surface();
                let discussion = state.observed.clone();
                let marker_names = state.marker_names();
                let restorable: BTreeSet<_> = state.deleted.keys().copied().collect();
                let actor = self.review.identity.author.id.clone();
                egui::ScrollArea::vertical()
                    .id_salt(("review-rows", surface))
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let rows = match discussion.rows(&restorable) {
                            Ok(rows) => rows,
                            Err(error) => {
                                ui.colored_label(Color32::DARK_RED, error);
                                return;
                            }
                        };
                        if rows.is_empty() {
                            ui.add_space(12.);
                            ui.weak("No comments yet.");
                        }
                        for row in rows {
                            let comment = row.comment;
                            ui.push_id(comment.id, |ui| {
                                egui::Frame::NONE
                                    .inner_margin(egui::Margin {
                                        left: (8 + row.depth.min(4) * 12) as i8,
                                        right: 8,
                                        top: 8,
                                        bottom: 8,
                                    })
                                    .show(ui, |ui| {
                                        if comment.deleted {
                                            ui.horizontal_wrapped(|ui| {
                                                ui.weak("Comment deleted");
                                                if restorable.contains(&comment.id)
                                                    && ui.link("Undo").clicked()
                                                {
                                                    action = Some(Action::Restore(comment.id));
                                                }
                                            });
                                            return;
                                        }
                                        ui.horizontal_wrapped(|ui| {
                                            if !comment.read_by.contains(&actor) {
                                                ui.colored_label(ACCENT, "●")
                                                    .on_hover_text("Unread");
                                            }
                                            ui.strong(&comment.author.name);
                                            if comment.imported {
                                                ui.weak("Imported");
                                            }
                                            ui.menu_button("…", |ui| {
                                                if comment.author.id == actor
                                                    && !comment.imported
                                                    && ui.button("Edit comment").clicked()
                                                {
                                                    action = Some(Action::Edit(comment.id));
                                                    ui.close();
                                                }
                                                let read = comment.read_by.contains(&actor);
                                                if ui
                                                    .button(if read {
                                                        "Mark unread"
                                                    } else {
                                                        "Mark read"
                                                    })
                                                    .clicked()
                                                {
                                                    action = Some(Action::Read(comment.id, !read));
                                                    ui.close();
                                                }
                                                if ui.button("Delete comment").clicked() {
                                                    action = Some(Action::Delete(comment.id));
                                                    ui.close();
                                                }
                                            });
                                        });
                                        let date = i64::try_from(comment.created_ms)
                                            .ok()
                                            .and_then(chrono::DateTime::from_timestamp_millis)
                                            .map(|date| {
                                                date.with_timezone(&chrono::Local)
                                                    .format("%e %b %Y, %H:%M")
                                                    .to_string()
                                            })
                                            .unwrap_or_default();
                                        ui.small(format!(
                                            "{date}{}",
                                            if comment.edited_ms.is_some() {
                                                " · Edited"
                                            } else {
                                                ""
                                            }
                                        ));
                                        review_body(
                                            ui,
                                            &comment.body.text,
                                            &marker_names,
                                            &mut action,
                                        );
                                        ui.horizontal_wrapped(|ui| {
                                            if ui.link("Reply").clicked() {
                                                action = Some(Action::Reply(comment.id));
                                            }
                                            let liked = comment.liked_by.contains(&actor);
                                            let label = if comment.liked_by.is_empty() {
                                                "Like".into()
                                            } else {
                                                format!(
                                                    "{} ({})",
                                                    if liked { "Unlike" } else { "Like" },
                                                    comment.liked_by.len()
                                                )
                                            };
                                            if ui.link(label).clicked() {
                                                action = Some(Action::Like(comment.id, !liked));
                                            }
                                        });
                                    });
                                ui.separator();
                            });
                        }
                    });
                if let Some(action) = action {
                    self.review_action(action);
                }
            },
        );
    }
    pub(super) fn review_action(&mut self, action: Action) {
        let actor = if matches!(action, Action::Read(..) | Action::Like(..)) {
            match self.review.identity.save() {
                Ok(author) => author.id,
                Err(error) => {
                    self.review_surface().error = Some(error);
                    return;
                }
            }
        } else {
            self.review.identity.author.id.clone()
        };
        match action {
            Action::New => self.begin_review(None, None),
            Action::Reply(id) => self.begin_review(Some(id), None),
            Action::Edit(id) => self.begin_review(None, Some(id)),
            Action::Delete(id) => {
                if let Some(token) = self.change_review(|d| d.delete(id)) {
                    let state = self.review_surface();
                    state.deleted.insert(id, token);
                    state.selected = None;
                    if let Some(draft) = &mut state.draft
                        && (draft.draft.comment == Some(id) || draft.draft.parent == Some(id))
                    {
                        draft.stale = true;
                    }
                }
            }
            Action::Restore(id) => {
                if let Some(token) = self.review_surface().deleted.get(&id).cloned()
                    && self.change_review(|d| d.restore_deleted(&token)).is_some()
                {
                    self.review_surface().deleted.remove(&id);
                }
            }
            Action::Read(id, read) => {
                self.change_review(|d| d.mark_read(id, &actor, read));
            }
            Action::Like(id, liked) => {
                self.change_review(|d| d.mark_liked(id, &actor, liked));
            }
            Action::Link(target) => self.review_link(&target),
            Action::Post => self.post_review(),
            Action::Cancel => self.cancel_review(),
        }
    }
    fn review_composer(
        &mut self,
        ui: &mut egui::Ui,
        composer: &mut Composer,
        action: &mut Option<Action>,
    ) {
        egui::Panel::bottom(egui::Id::new(("review-submit", composer.input)))
            .exact_size(72.)
            .frame(egui::Frame::NONE)
            .show(ui, |ui| {
                self.review_insert_tools(ui, composer);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add_enabled(
                            !composer.stale && !composer.text.value.trim().is_empty(),
                            egui::Button::new(
                                egui::RichText::new(if composer.draft.comment.is_some() {
                                    "Save comment"
                                } else {
                                    "Post"
                                })
                                .color(Color32::WHITE),
                            )
                            .fill(chrome::toolbar_palette().selected),
                        )
                        .on_hover_text("Command/Ctrl+Enter")
                        .clicked()
                    {
                        *action = Some(Action::Post);
                    }
                    if ui.button("Cancel").clicked() {
                        *action = Some(Action::Cancel);
                    }
                });
            });
        egui::ScrollArea::vertical()
            .id_salt(("review-compose-fields", composer.input))
            .auto_shrink([false, false])
            .min_scrolled_height(0.)
            .show(ui, |ui| {
                self.review_composer_fields(ui, composer);
                if let Some(error) = &self.review_surface().error {
                    ui.colored_label(Color32::DARK_RED, error);
                }
            });
    }
    fn review_composer_fields(&mut self, ui: &mut egui::Ui, composer: &mut Composer) {
        let label = ui.strong(if composer.draft.comment.is_some() {
            "Edit comment"
        } else if composer.draft.parent.is_some() {
            "Reply"
        } else {
            "New comment"
        });
        if composer.stale {
            ui.colored_label(
                Color32::DARK_RED,
                "This discussion changed. Copy your draft before cancelling and reopening it.",
            );
            if ui.button("Copy draft").clicked() {
                ui.ctx().copy_text(composer.text.value.clone());
            }
        }
        let output = egui::ScrollArea::vertical()
            .id_salt(("review-text-scroll", self.editor.surface_id()))
            .max_height((ui.clip_rect().height() - 32.).clamp(48., 160.))
            .min_scrolled_height(48.)
            .animated(false)
            .show(ui, |ui| {
                let mut output = egui::TextEdit::multiline(&mut composer.text.value)
                    .id(composer.input)
                    .desired_rows(4)
                    .desired_width(f32::INFINITY)
                    .char_limit(wireframe::review::MAX_BODY_BYTES)
                    .hint_text("Write a comment…")
                    .show(ui);
                if composer.focus {
                    output.response.request_focus();
                    let index = composer.text.value
                        [..composer.caret.min(composer.text.value.len())]
                        .chars()
                        .count();
                    let cursor = egui::text::CCursor::new(index);
                    output
                        .state
                        .cursor
                        .set_char_range(Some(egui::text::CCursorRange::one(cursor)));
                    output.state.clone().store(ui.ctx(), output.response.id);
                    let rect = output.galley.pos_from_cursor(cursor).translate(
                        output.galley_pos.to_vec2() - egui::vec2(output.galley.rect.left(), 0.),
                    );
                    ui.scroll_to_rect(rect, None);
                    composer.focus = false;
                } else if let Some(range) = output.cursor_range {
                    composer.caret = composer
                        .text
                        .value
                        .char_indices()
                        .nth(range.primary.index.into())
                        .map_or(composer.text.value.len(), |(i, _)| i);
                }
                #[cfg(test)]
                {
                    composer.caret_visible =
                        output.state.cursor.char_range().is_some_and(|range| {
                            let rect = output.galley.pos_from_cursor(range.primary).translate(
                                output.galley_pos.to_vec2()
                                    - egui::vec2(output.galley.rect.left(), 0.),
                            );
                            ui.clip_rect().contains(rect.center())
                        });
                }
                output
            })
            .inner;
        egui::Response::clone(&output.response).labelled_by(label.id);
    }
    fn review_insert_tools(&mut self, ui: &mut egui::Ui, composer: &mut Composer) {
        let compact = ui.available_width() < 240.;
        ui.add_enabled_ui(!composer.stale, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.;
                let add = ui.small_button(if compact { "Callout" } else { "Add Callout" });
                add.widget_info(|| {
                    egui::WidgetInfo::labeled(
                        egui::WidgetType::Button,
                        ui.is_enabled(),
                        "Add Callout",
                    )
                });
                if add.clicked() {
                    let position = self.world(egui::pos2(
                        self.canvas_rect.right() - 65.,
                        add.rect.center().y.clamp(
                            self.canvas_rect.top() + 40.,
                            self.canvas_rect.bottom().max(self.canvas_rect.top() + 80.) - 40.,
                        ),
                    ));
                    let result = composer
                        .text
                        .encode(composer.caret)
                        .and_then(|(text, caret)| {
                            composer.draft.body.text = text;
                            composer.draft.add_callout(
                                &self.review_surface().observed,
                                wireframe::document::Point {
                                    x: position.x,
                                    y: position.y,
                                },
                                caret,
                            )
                        });
                    match result {
                        Ok(id) => {
                            let old = composer.text.value.len();
                            if let Err(error) = composer.text.show_source(&composer.draft.body.text)
                            {
                                self.review_surface().error = Some(error);
                            }
                            composer.caret = (composer.caret
                                + composer.text.value.len().saturating_sub(old))
                            .min(composer.text.value.len());
                            composer.focus = true;
                            self.review_surface().selected = Some(id);
                        }
                        Err(error) => self.review_surface().error = Some(error),
                    }
                }
                ui.menu_button(if compact { "Link" } else { "Link to page" }, |ui| {
                    egui::ScrollArea::vertical()
                        .max_height(240.)
                        .show(ui, |ui| {
                            for page in self.editor.document.live_pages() {
                                for (id, name) in std::iter::once((page.id, page.name.clone()))
                                    .chain(
                                        page.alternates
                                            .iter()
                                            .map(|a| (a.id, format!("{} / {}", page.name, a.name))),
                                    )
                                {
                                    if ui.button(&name).clicked() {
                                        let token =
                                            composer.text.alias(&name, &format!("surface:{id}"));
                                        let insertion = format!(" {token} ");
                                        composer.text.value.insert_str(composer.caret, &insertion);
                                        composer.caret += insertion.len();
                                        composer.focus = true;
                                        ui.close();
                                    }
                                }
                            }
                        });
                });
                if let Some(selected) = self.review_surface().selected
                    && let Some(marker) = composer
                        .draft
                        .body
                        .callouts
                        .iter_mut()
                        .find(|m| m.id == selected)
                {
                    let response = ui.small_button("…");
                    if response.clicked() {
                        composer.callout_settings = !composer.callout_settings;
                    }
                    // A separate open state keeps the form alive when a nested
                    // ComboBox opens egui's single memory-owned menu popup.
                    egui::Popup::from_response(&response)
                        .open_bool(&mut composer.callout_settings)
                        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                        .show(|ui| {
                            ui.set_min_width(220.);
                            ui.horizontal_wrapped(|ui| {
                                ui.label(format!("Callout {}", marker.label));
                                egui::ComboBox::new("review-tip", "Tip")
                                    .selected_text(format!("{:?}", marker.tip))
                                    .show_ui(ui, |ui| {
                                        for tip in [
                                            Tip::Left,
                                            Tip::Top,
                                            Tip::Right,
                                            Tip::Bottom,
                                            Tip::None,
                                        ] {
                                            ui.selectable_value(
                                                &mut marker.tip,
                                                tip,
                                                format!("{tip:?}"),
                                            );
                                        }
                                    });
                                egui::ComboBox::new("review-color", "Color")
                                    .selected_text(format!("{:?}", marker.color))
                                    .show_ui(ui, |ui| {
                                        for color in [
                                            Color::Blue,
                                            Color::Green,
                                            Color::Yellow,
                                            Color::Red,
                                            Color::Gray,
                                        ] {
                                            ui.selectable_value(
                                                &mut marker.color,
                                                color,
                                                format!("{color:?}"),
                                            );
                                        }
                                    });
                            });
                            ui.horizontal(|ui| {
                                let label = ui.label("X");
                                ui.add(
                                    egui::DragValue::new(&mut marker.position.x).range(-1e7..=1e7),
                                )
                                .labelled_by(label.id);
                                let label = ui.label("Y");
                                ui.add(
                                    egui::DragValue::new(&mut marker.position.y).range(-1e7..=1e7),
                                )
                                .labelled_by(label.id);
                            });
                        });
                    response.widget_info(|| {
                        egui::WidgetInfo::labeled(
                            egui::WidgetType::Button,
                            ui.is_enabled(),
                            "Callout settings",
                        )
                    });
                    response.on_hover_text("Callout settings");
                }
            });
        });
    }
}
fn review_body(
    ui: &mut egui::Ui,
    text: &str,
    names: &BTreeMap<Id, String>,
    action: &mut Option<Action>,
) {
    use wireframe::rich_text::Inline;
    let rich = wireframe::rich_text::RichText::parse(text, false);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 0.;
        for item in rich.items {
            match item {
                Inline::Span { text, style } => {
                    let text = style
                        .target
                        .as_deref()
                        .and_then(|t| t.strip_prefix("callout:"))
                        .and_then(|id| id.parse::<Id>().ok())
                        .and_then(|id| names.get(&id))
                        .map_or(text, |name| format!("({name})"));
                    let mut label = egui::RichText::new(text);
                    if let Some(size) = style.size {
                        label = label.size(f32::from(size));
                    }
                    if let Some(color) = style.color.as_deref().map(|value| {
                        wireframe::rich_text::painting::parse_color(value)
                            .unwrap_or(egui::Color32::BLACK)
                    }) {
                        label = label.color(color);
                    }
                    if style.bold {
                        label = label.strong();
                    }
                    if style.italic {
                        label = label.italics();
                    }
                    if style.underline {
                        label = label.underline();
                    }
                    if style.strike {
                        label = label.strikethrough();
                    }
                    if style.disabled {
                        label = label.weak();
                    }
                    if let Some(target) = style.target.as_deref().filter(|_| style.link) {
                        if target.starts_with("surface:") || target.starts_with("callout:") {
                            if ui.add(egui::Link::new(label)).clicked() {
                                *action = Some(Action::Link(target.into()));
                            }
                        } else if target.starts_with("https://")
                            || target.starts_with("http://")
                            || target.starts_with("mailto:")
                        {
                            ui.hyperlink_to(label, target);
                        } else {
                            ui.label(label);
                        }
                    } else {
                        ui.label(label);
                    }
                }
                Inline::Break(_) => {
                    ui.end_row();
                }
                Inline::Bullet => {
                    ui.label("• ");
                }
            }
        }
    });
}

impl WireframeApp {
    fn review_header_actions(&mut self, ui: &mut egui::Ui, compact: bool) {
        let hide = if self.present {
            chrome::close_button(ui, "Hide Comments", "Hide Comments (C)")
        } else {
            ui.button("×")
        };
        hide.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), "Hide Comments")
        });
        if hide.on_hover_text("Hide Comments").clicked() {
            if self.present {
                self.presentation_view
                    .session
                    .toggle(wireframe::presentation::Aid::Comments);
            } else {
                self.review.open = false;
            }
        }
        let (reviewer_menu, _) =
            egui::containers::menu::MenuButton::new(if compact { "…" } else { "Reviewer" })
                .config(
                    egui::containers::menu::MenuConfig::new()
                        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside),
                )
                .ui(ui, |ui| {
                    let label = ui.label("Reviewer name");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.review.identity.name)
                            .id_salt("reviewer-name")
                            .char_limit(128),
                    )
                    .labelled_by(label.id);
                    ui.small("Saved locally. No account required.");
                    if ui.button("Save name").clicked() {
                        match self.review.identity.save() {
                            Ok(_) => ui.close(),
                            Err(error) => self.review_surface().error = Some(error),
                        }
                    }
                });
        reviewer_menu.widget_info(|| {
            egui::WidgetInfo::labeled(
                egui::WidgetType::Button,
                ui.is_enabled(),
                "Reviewer settings",
            )
        });
        reviewer_menu.on_hover_text("Reviewer settings");
    }
}
