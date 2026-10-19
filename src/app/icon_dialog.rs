// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use wireframe::document::{IconSource, Value};

pub(super) struct IconDialog {
    targets: Vec<Id>,
    row: Option<usize>,
    app_bar: Option<(usize, Vec<Option<IconSource>>)>,
    document: Project,
    session: String,
    page: Id,
    alternate: Option<Id>,
    context: Vec<Id>,
    definition: Option<Id>,
    library: Option<Id>,
    chosen: Option<IconSource>,
    size: usize,
    query: String,
    category: String,
}
const SIZES: [&str; 6] = ["16", "24", "32", "48", "64", "128"];

impl IconDialog {
    fn matches(&self, editor: &Editor) -> bool {
        self.session == editor.clipboard_session()
            && self.document == editor.document
            && self.page == editor.page
            && self.alternate == editor.active_alternate(editor.page)
            && self.context == editor.context
            && self.definition == editor.definition
            && self.library == editor.library
    }
}

impl WireframeApp {
    pub(super) fn begin_icon_dialog(&mut self, target: Id) {
        self.begin_icon_dialog_targets(vec![target]);
    }

    pub(super) fn begin_selection_icon_dialog(&mut self) {
        self.begin_icon_dialog_targets(self.editor.ordered_selection());
    }

    fn begin_icon_dialog_targets(&mut self, targets: Vec<Id>) {
        let Some(&target) = targets.first() else {
            return;
        };
        if targets.iter().any(|id| {
            self.editor
                .control_properties(*id)
                .is_none_or(|(s, _)| s.property("icon").is_none())
        }) {
            return;
        }
        self.end_pointer_gestures();
        let Some((spec, values)) = self.editor.control_properties(target) else {
            return;
        };
        if spec.property("icon").is_none() {
            return;
        }
        let chosen = match values.get("icon") {
            Some(Value::Icon(source)) => Some(source.clone()),
            _ => None,
        };
        let size = match values.get("icon_size") {
            Some(Value::Choice(size)) => SIZES.iter().position(|s| s == size).unwrap_or(1),
            _ => 1,
        };
        self.icon_dialog = Some(IconDialog {
            targets,
            row: None,
            app_bar: None,
            document: self.editor.document.clone(),
            session: self.editor.clipboard_session().into(),
            page: self.editor.page,
            alternate: self.editor.active_alternate(self.editor.page),
            context: self.editor.context.clone(),
            definition: self.editor.definition,
            library: self.editor.library,
            chosen,
            size,
            query: String::new(),
            category: String::new(),
        });
    }

    pub(super) fn begin_row_icon_dialog(&mut self, targets: Vec<Id>, row: usize) {
        let Some(entry) = self
            .editor
            .list_row_icons(&targets)
            .and_then(|rows| rows.into_iter().find(|entry| entry.row == row))
        else {
            return;
        };
        self.end_pointer_gestures();
        self.icon_dialog = Some(IconDialog {
            targets: entry.targets,
            row: Some(row),
            app_bar: None,
            document: self.editor.document.clone(),
            session: self.editor.clipboard_session().to_owned(),
            page: self.editor.page,
            alternate: self.editor.active_alternate(self.editor.page),
            context: self.editor.context.clone(),
            definition: self.editor.definition,
            library: self.editor.library,
            chosen: entry.icon,
            size: 1,
            query: String::new(),
            category: String::new(),
        });
    }

    pub(super) fn begin_app_bar_icon_dialog(&mut self, targets: Vec<Id>, index: usize) {
        let Some(slots) = self.editor.app_bar_icon_slots(&targets) else {
            return;
        };
        if index > slots.len() {
            return;
        }
        self.end_pointer_gestures();
        let chosen = slots.get(index).and_then(|slot| slot.icon.clone());
        self.icon_dialog = Some(IconDialog {
            targets,
            row: None,
            app_bar: Some((index, slots.into_iter().map(|slot| slot.icon).collect())),
            document: self.editor.document.clone(),
            session: self.editor.clipboard_session().to_owned(),
            page: self.editor.page,
            alternate: self.editor.active_alternate(self.editor.page),
            context: self.editor.context.clone(),
            definition: self.editor.definition,
            library: self.editor.library,
            chosen,
            size: 1,
            query: String::new(),
            category: String::new(),
        });
    }

    fn apply_icon_dialog(&mut self, draft: &IconDialog) -> Result<(), String> {
        if !draft.matches(&self.editor) {
            return Err("The icon target changed. Open the icon picker again.".into());
        }
        let source = draft.chosen.clone().ok_or("Choose an icon first")?;
        match &source {
            IconSource::Catalog(id) if wireframe::icons::get(id).is_none() => {
                return Err("Icon no longer exists".into());
            }
            IconSource::Asset(id) if self.editor.document.live_asset(*id).is_none() => {
                return Err("Image no longer exists".into());
            }
            _ => {}
        }
        if let Some(row) = draft.row {
            self.editor.begin("Change row icon");
            let result = self
                .editor
                .set_list_row_icon(&draft.targets, row, Some(source));
            if result.is_ok() {
                self.editor.commit();
            } else {
                self.editor.cancel();
            }
            return result;
        }
        if let Some((index, icons)) = &draft.app_bar {
            let mut icons = icons.clone();
            if *index == icons.len() {
                icons.push(Some(source));
            } else {
                icons[*index] = Some(source);
            }
            self.editor.begin("Change app bar icons");
            let result = self.editor.replace_app_bar_icons(&draft.targets, icons);
            if result.is_ok() {
                self.editor.commit();
            } else {
                self.editor.cancel();
            }
            return result;
        }
        let size = Value::Choice(SIZES[draft.size].into());
        if draft.targets.iter().all(|id| {
            self.editor.control_properties(*id).is_some_and(|(_, v)| {
                v.get("icon") == Some(&Value::Icon(source.clone()))
                    && v.get("icon_size") == Some(&size)
            })
        }) {
            return Ok(());
        }
        self.editor.begin("Change icon");
        let result = draft.targets.iter().try_for_each(|&id| {
            self.editor
                .set_control_property(id, "icon", Value::Icon(source.clone()))?;
            self.editor
                .set_control_property(id, "icon_size", size.clone())
        });
        if result.is_ok() {
            self.editor.commit();
        } else {
            self.editor.cancel();
        }
        result
    }

    pub(super) fn icon_dialog_ui(&mut self, ctx: &egui::Context) {
        let Some(mut draft) = self.icon_dialog.take() else {
            return;
        };
        if !draft.matches(&self.editor) {
            return;
        }
        let mut close = false;
        let mut accept = false;
        let mut query_id = None;
        egui::Modal::new(egui::Id::new("icon-library-dialog")).show(ctx, |ui| {
            ui.set_width(680_f32.min((ctx.content_rect().width() - 40.).max(280.)));
            ui.vertical_centered(|ui| {
                ui.heading("Icon Library");
                ui.small("Font Awesome Free · CC BY 4.0");
            });
            ui.separator();
            let height = (ctx.content_rect().height() - 180.).clamp(160., 500.);
            ui.allocate_ui_with_layout(
                Vec2::new(ui.available_width(), height),
                egui::Layout::left_to_right(egui::Align::Min),
                |ui| {
                    ui.allocate_ui_with_layout(
                        Vec2::new(170_f32.min(ui.available_width() * 0.35), height),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            egui::ScrollArea::vertical()
                                .id_salt("icon-categories")
                                .max_height(height)
                                .show(ui, |ui| {
                                    ui.selectable_value(&mut draft.category, String::new(), "All");
                                    for category in wireframe::icons::categories() {
                                        ui.selectable_value(
                                            &mut draft.category,
                                            category.id.clone(),
                                            &category.label,
                                        );
                                    }
                                    ui.selectable_value(
                                        &mut draft.category,
                                        "assets".into(),
                                        "Assets",
                                    );
                                });
                        },
                    );
                    ui.separator();
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::splat(4.);
                        let columns = ((ui.available_width() / 35.).floor() as usize).max(1);
                        if draft.category == "assets" {
                            let query = draft.query.to_lowercase();
                            let assets: Vec<_> = self
                                .editor
                                .document
                                .live_assets()
                                .filter(|asset| {
                                    asset.name.to_lowercase().contains(&query)
                                        && wireframe::images::dimensions(&asset.bytes).is_ok()
                                })
                                .collect();
                            egui::ScrollArea::vertical()
                                .id_salt("icon-grid")
                                .max_height(height)
                                .show_rows(ui, 35., assets.len().div_ceil(columns), |ui, rows| {
                                    for row in rows {
                                        ui.horizontal(|ui| {
                                            for asset in
                                                assets.iter().skip(row * columns).take(columns)
                                            {
                                                let source = IconSource::Asset(asset.id);
                                                let (rect, response) = ui.allocate_exact_size(
                                                    Vec2::splat(31.),
                                                    egui::Sense::click(),
                                                );
                                                if draft.chosen.as_ref() == Some(&source) {
                                                    ui.painter().rect_filled(
                                                        rect,
                                                        3.,
                                                        ui.visuals().selection.bg_fill,
                                                    );
                                                }
                                                ui.scope_builder(
                                                    egui::UiBuilder::new()
                                                        .max_rect(rect.shrink(6.)),
                                                    |ui| {
                                                        super::icon_picker::preview(ui, asset, 19.)
                                                    },
                                                );
                                                response.widget_info(|| {
                                                    egui::WidgetInfo::labeled(
                                                        egui::WidgetType::Button,
                                                        true,
                                                        &asset.name,
                                                    )
                                                });
                                                if response.on_hover_text(&asset.name).clicked() {
                                                    draft.chosen = Some(source);
                                                }
                                            }
                                        });
                                    }
                                });
                            if assets.is_empty() {
                                ui.label("No matching images");
                            }
                        } else {
                            let category = wireframe::icons::categories()
                                .iter()
                                .find(|c| c.id == draft.category);
                            let mut found = wireframe::icons::search(&draft.query, "");
                            found.retain(|icon| category.is_none_or(|c| c.contains(icon)));
                            egui::ScrollArea::vertical()
                                .id_salt("icon-grid")
                                .max_height(height)
                                .show_rows(ui, 35., found.len().div_ceil(columns), |ui, rows| {
                                    for row in rows {
                                        ui.horizontal(|ui| {
                                            for icon in
                                                found.iter().skip(row * columns).take(columns)
                                            {
                                                let source = IconSource::Catalog(icon.id.clone());
                                                let (rect, response) = ui.allocate_exact_size(
                                                    Vec2::splat(31.),
                                                    egui::Sense::click(),
                                                );
                                                if draft.chosen.as_ref() == Some(&source)
                                                    || response.hovered()
                                                {
                                                    ui.painter().rect_filled(
                                                        rect,
                                                        3.,
                                                        ui.visuals().selection.bg_fill,
                                                    );
                                                }
                                                icon.paint(
                                                    ui.painter(),
                                                    rect.shrink(6.),
                                                    ui.visuals().text_color(),
                                                );
                                                let label = icon.catalog_name();
                                                response.widget_info(|| {
                                                    egui::WidgetInfo::labeled(
                                                        egui::WidgetType::Button,
                                                        true,
                                                        &label,
                                                    )
                                                });
                                                if response.on_hover_text(label).clicked() {
                                                    draft.chosen = Some(source);
                                                }
                                            }
                                        });
                                    }
                                });
                            if found.is_empty() {
                                ui.label("No matching icons");
                            }
                        }
                    });
                },
            );
            ui.separator();
            ui.horizontal_wrapped(|ui| {
                let response = ui.add(
                    TextInput::singleline(&mut draft.query)
                        .hint_text("Search icons…")
                        .desired_width(180.)
                        .autofocus(true),
                );
                query_id = Some(response.id);
                if draft.row.is_some() {
                    ui.label("Row icons follow the text size.");
                } else if draft.app_bar.is_some() {
                    ui.label("App bar icons use a fixed size.");
                } else {
                    ui.vertical(|ui| {
                        ui.spacing_mut().slider_width = 220.;
                        ui.add(
                            egui::Slider::new(&mut draft.size, 0..=5)
                                .show_value(false)
                                .text("Icon size"),
                        );
                        ui.horizontal(|ui| {
                            for (index, label) in
                                ["XS", "S", "M", "L", "XL", "XXL"].iter().enumerate()
                            {
                                ui.add_sized(
                                    [30., 18.],
                                    egui::Button::selectable(draft.size == index, *label),
                                )
                                .clicked()
                                .then(|| draft.size = index);
                            }
                        });
                    });
                }
                close |= ui.button("Close").clicked();
                accept |= ui
                    .add_enabled(draft.chosen.is_some(), egui::Button::new("Select"))
                    .clicked();
            });
            close |= ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape));
            accept |= draft.chosen.is_some()
                && ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter));
        });
        if accept && !close {
            if let Err(error) = self.apply_icon_dialog(&draft) {
                self.error = Some(error);
            }
            close = true;
        }
        if close {
            if let Some(id) = query_id {
                wireframe::text_entry::forget(ctx, id);
            }
            if let Some(id) = self.canvas_focus_id {
                ctx.memory_mut(|m| m.request_focus(id));
            }
        } else {
            self.icon_dialog = Some(draft);
        }
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod selection_tests {
    use super::*;
    #[test]
    fn multi_control_icon_draft_is_provisional_atomic_and_cancellable() {
        let ctx = egui::Context::default();
        let mut app = WireframeApp::workspace(&ctx, None);
        app.editor.add(Kind::Button, 50., 50.);
        app.editor.add(Kind::Button, 200., 50.);
        app.editor.select_all();
        let before = app.editor.document.clone();
        let ids = app.editor.ordered_selection();
        app.begin_selection_icon_dialog();
        let mut draft = app.icon_dialog.take().unwrap();
        draft.chosen = Some(IconSource::Catalog("star-solid".into()));
        draft.size = 3;
        assert_eq!(app.editor.document, before);
        app.apply_icon_dialog(&draft).unwrap();
        let after = app.editor.document.clone();
        for id in ids {
            let (_, v) = app.editor.control_properties(id).unwrap();
            assert_eq!(
                v.get("icon"),
                Some(&Value::Icon(IconSource::Catalog("star-solid".into())))
            );
            assert_eq!(v.get("icon_size"), Some(&Value::Choice("48".into())));
        }
        app.editor.undo();
        assert_eq!(app.editor.document, before);
        let redo = app.editor.redo_name().map(str::to_owned);
        app.begin_selection_icon_dialog();
        let mut draft = app.icon_dialog.take().unwrap();
        draft.chosen = Some(IconSource::Catalog("star-solid".into()));
        draft.size = 5;
        app.icon_dialog = Some(draft);
        let mut out = ctx.run_ui(
            egui::RawInput {
                events: vec![egui::Event::Key {
                    key: Key::Escape,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: Modifiers::NONE,
                }],
                ..Default::default()
            },
            |ui| app.icon_dialog_ui(ui.ctx()),
        );
        out.textures_delta.clear();
        assert!(app.icon_dialog.is_none());
        assert_eq!(app.editor.document, before);
        assert_eq!(app.editor.redo_name(), redo.as_deref());
        app.editor.redo();
        assert_eq!(app.editor.document, after);
    }
}
