// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use review::Resource;

pub(in crate::app) enum Action {
    None,
    Close,
    Back,
    CancelReview,
    Open(usize),
    Local,
    Review,
    Choose(Choice),
    Apply,
}
pub(super) fn show(ctx: &egui::Context, browser: &mut Browser, destination: &Editor) -> Action {
    let mut action = Action::None;
    let response = egui::Modal::new(egui::Id::new("more-controls"))
        .frame(
            egui::Frame::NONE
                .corner_radius(10)
                .fill(chrome::PANEL)
                .inner_margin(16),
        )
        .show(ctx, |ui| {
            ui.set_width((ctx.content_rect().width() - 64.).clamp(280., 1100.));
            ui.horizontal(|ui| {
                ui.heading("More Controls");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Close").clicked() {
                        action = Action::Close;
                    }
                });
            });
            ui.separator();
            if let Some(error) = &browser.error {
                ui.colored_label(Color32::from_rgb(150, 40, 25), error);
                if let Some(preview) = &browser.preview
                    && ui.button("Reload preview").clicked()
                {
                    action = Action::Open(preview.index);
                }
            }
            if browser.loading.is_some() {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("Loading project…");
                });
            } else if browser.review.is_some() {
                review_view(ui, browser, destination, &mut action);
            } else if browser.preview.is_some() {
                preview_view(ui, browser, &mut action);
            } else {
                list_view(ui, browser, &mut action);
            }
        });
    if response.is_top_modal
        && !response.any_popup_open
        && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape))
    {
        action = if browser.review.is_some() {
            Action::CancelReview
        } else if browser.preview.is_some() {
            Action::Back
        } else {
            Action::Close
        };
    }
    action
}
fn list_view(ui: &mut egui::Ui, browser: &mut Browser, action: &mut Action) {
    ui.horizontal(|ui| {
        let search = ui.add(
            egui::TextEdit::singleline(&mut browser.catalog.query)
                .hint_text("Search projects")
                .desired_width(330.),
        );
        if browser.focus_search {
            search.request_focus();
            browser.focus_search = false;
        }
        if !browser.catalog.query.is_empty() && ui.button("Clear").clicked() {
            browser.catalog.query.clear();
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("Open local project…").clicked() {
                *action = Action::Local;
            }
        });
    });
    ui.add_space(8.);
    ui.horizontal_top(|ui| {
        ui.vertical(|ui| {
            ui.set_width(145.);
            for category in browser.catalog.categories() {
                if ui
                    .selectable_label(browser.catalog.category == category, &category)
                    .clicked()
                {
                    browser.catalog.category = category;
                }
            }
        });
        ui.separator();
        let results = browser.catalog.results();
        egui::ScrollArea::vertical()
            .id_salt("more-controls-results")
            .max_height((ui.ctx().content_rect().height() - 220.).max(180.))
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                    if results.is_empty() {
                        ui.label("No projects match your search.");
                    }
                    for index in results {
                        let entry = browser.catalog.entries[index].clone();
                        ui.push_id(&entry.id, |ui| {
                            ui.horizontal_top(|ui| {
                                let (rect, response) =
                                    ui.allocate_exact_size(Vec2::new(190., 125.), Sense::click());
                                if ui.is_rect_visible(rect) {
                                    let scene = browser
                                        .thumbnails
                                        .entry(entry.id.clone())
                                        .or_insert_with(|| {
                                            entry.load().and_then(|snapshot| {
                                                let id = snapshot
                                                    .initial_resource()
                                                    .ok_or("No preview resource")?;
                                                Editor::new(snapshot.project)
                                                    .rendered_scene(id)
                                                    .map(Arc::new)
                                            })
                                        });
                                    paint(ui, rect, scene);
                                }
                                response.widget_info(|| {
                                    egui::WidgetInfo::labeled(
                                        egui::WidgetType::Button,
                                        true,
                                        format!("Preview {}", entry.title),
                                    )
                                });
                                if response.has_focus() {
                                    ui.painter().rect_stroke(
                                        rect,
                                        0,
                                        Stroke::new(2., ACCENT),
                                        StrokeKind::Inside,
                                    );
                                }
                                if response.clicked() {
                                    *action = Action::Open(index);
                                }
                                ui.vertical(|ui| {
                                    if ui.link(&entry.title).clicked() {
                                        *action = Action::Open(index);
                                    }
                                    ui.label(&entry.description);
                                    ui.weak(&entry.category);
                                });
                            });
                            ui.add_space(10.);
                        });
                    }
                });
            });
    });
}
fn preview_view(ui: &mut egui::Ui, browser: &mut Browser, action: &mut Action) {
    let preview = browser.preview.as_mut().unwrap();
    let results = browser.catalog.results();
    let position = results.iter().position(|i| *i == preview.index);
    ui.horizontal_wrapped(|ui| {
        if ui.button("← Back to results").clicked() {
            *action = Action::Back;
        }
        let previous = position.and_then(|p| p.checked_sub(1)).map(|p| results[p]);
        let next = position.and_then(|p| results.get(p + 1)).copied();
        if ui
            .add_enabled(previous.is_some(), egui::Button::new("Previous"))
            .clicked()
        {
            *action = Action::Open(previous.unwrap());
        }
        if ui
            .add_enabled(next.is_some(), egui::Button::new("Next"))
            .clicked()
        {
            *action = Action::Open(next.unwrap());
        }
        ui.strong(&preview.snapshot.project.title);
    });
    let resources: Vec<_> = preview
        .snapshot
        .project
        .live_pages()
        .flat_map(|p| {
            std::iter::once((p.id, p.name.clone())).chain(
                p.alternates
                    .iter()
                    .map(|a| (a.id, format!("{} / {}", p.name, a.name))),
            )
        })
        .chain(
            preview
                .snapshot
                .project
                .live_libraries()
                .map(|l| (l.id, format!("Library: {}", l.name))),
        )
        .collect();
    ui_text::combo_box(
        ui,
        "preview-resource",
        resources
            .iter()
            .find(|(id, _)| *id == preview.resource)
            .map(|(_, name)| name.as_str())
            .unwrap_or("Select a page"),
        |ui| {
            for (id, name) in &resources {
                if ui_text::selectable_value(ui, &mut preview.resource, *id, name).clicked() {
                    ui.close();
                }
            }
        },
    );
    let height = (ui.ctx().content_rect().height() - 285.).clamp(180., 570.);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), height), Sense::hover());
    paint(
        ui,
        rect,
        &preview
            .editor
            .rendered_scene(preview.resource)
            .map(Arc::new),
    );
    ui.add_space(8.);
    ui.horizontal_wrapped(|ui| {
        ui.label("Import the project's pages, component libraries and images.");
        if ui
            .add(
                egui::Button::new(egui::RichText::new("Import into Project").color(Color32::WHITE))
                    .fill(ACCENT),
            )
            .clicked()
        {
            *action = Action::Review;
        }
    });
}
fn review_view(
    ui: &mut egui::Ui,
    browser: &mut Browser,
    destination: &Editor,
    action: &mut Action,
) {
    let preview = browser.preview.as_ref().unwrap();
    let review = browser.review.as_mut().unwrap();
    review_content(
        ui,
        review,
        destination,
        &preview.editor,
        &format!("Import {}", preview.snapshot.project.title),
        "Import project",
        action,
    );
}

pub(in crate::app) fn review_content(
    ui: &mut egui::Ui,
    review: &mut Review,
    destination: &Editor,
    incoming: &Editor,
    title: &str,
    apply_label: &str,
    action: &mut Action,
) {
    if !review.valid_destination(destination) {
        review.error = Some("The destination project changed. Review the import again.".into());
    }
    ui.strong(title);
    if let Some(error) = &review.error {
        ui.label(error);
        if ui.button("Review choices again").clicked() {
            *action = Action::Review;
        }
    } else if let Some(prepared) = &review.prepared {
        let c = prepared.counts();
        ui.label(format!(
            "Pages: {} added, {} replaced, {} kept",
            c.pages, c.pages_replaced, c.pages_reused
        ));
        ui.label(format!(
            "Component libraries: {} added, {} replaced, {} kept",
            c.libraries, c.libraries_replaced, c.libraries_reused
        ));
        ui.label(format!(
            "Images: {} added, {} replaced, {} kept",
            c.assets_added, c.assets_replaced, c.assets_reused
        ));
        let unresolved = prepared.newly_orphaned_overrides();
        if unresolved > 0 {
            ui.label(format!("{unresolved} additional component edits will refer to missing controls. Their values remain stored. Review your choices to preserve the existing components, or import with these changes."));
        }
        ui.horizontal(|ui| {
            if ui.button("Review choices again").clicked() {
                *action = Action::Review;
            }
            let label = if unresolved > 0 {
                "Import with changes"
            } else {
                apply_label
            };
            if ui
                .add(
                    egui::Button::new(egui::RichText::new(label).color(Color32::WHITE))
                        .fill(ACCENT),
                )
                .clicked()
            {
                *action = Action::Apply;
            }
        });
    } else {
        review.preview_pair(destination, incoming);
        let conflicts = review.conflicts();
        if let Some(c) = conflicts.first() {
            let kind = match c.kind {
                Resource::Asset => "image",
                Resource::Page => "page",
                Resource::Library => "component library",
            };
            ui.label(format!(
                "The {kind} “{}” already exists. Which version would you like to use?",
                c.name
            ));
            if let Some((existing, incoming)) = c.image_availability {
                let state = |trashed| if trashed { "in Trash" } else { "available" };
                ui.label(format!(
                    "Existing image: {}. New image: {}.",
                    state(existing),
                    state(incoming)
                ));
                if existing != incoming {
                    ui.label("Use New also changes availability for controls that use the existing image. Keep Both preserves both states separately.");
                }
            }
            if let Some(pair) = &review.pair {
                let width = (ui.available_width() - ui.spacing().item_spacing.x) / 2.;
                let height = (ui.ctx().content_rect().height() * 0.38).clamp(120., 330.);
                ui.horizontal_top(|ui| {
                    for (title, scene) in ["Existing", "New"].into_iter().zip(&pair.scenes) {
                        ui.vertical(|ui| {
                            ui.set_width(width);
                            ui.label(title);
                            let (rect, _) =
                                ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
                            paint(ui, rect, scene);
                        });
                    }
                });
            }
            if conflicts.len() > 1 {
                ui.checkbox(
                    &mut review.all_remaining,
                    format!(
                        "Apply to the remaining {} {kind} conflicts",
                        conflicts.len() - 1
                    ),
                );
            }
            ui.horizontal_wrapped(|ui| {
                for (label, choice, help) in [
                    ("Keep Both", Choice::Both, "Add an independent copy"),
                    (
                        "Use Existing",
                        Choice::Existing,
                        "Keep the current resource and use it for incoming references",
                    ),
                    (
                        "Use New",
                        Choice::New,
                        "Replace the current content while retaining its identity",
                    ),
                ] {
                    if ui.button(label).on_hover_text(help).clicked() {
                        *action = Action::Choose(choice);
                    }
                }
            });
        }
    }
    ui.separator();
    if ui.button("Cancel import").clicked() {
        *action = Action::CancelReview;
    }
}
fn paint(ui: &egui::Ui, rect: Rect, scene: &Result<Arc<Scene>, String>) {
    ui.painter().rect_filled(rect, 0, Color32::WHITE);
    ui.painter().rect_stroke(
        rect,
        0,
        Stroke::new(1., Color32::from_gray(185)),
        StrokeKind::Inside,
    );
    let content = rect.shrink(12.);
    match scene {
        Ok(scene) if !scene.items.is_empty() => {
            let b = scene.bounds;
            let scale = (content.width() / b.w.max(1.))
                .min(content.height() / b.h.max(1.))
                .min(1.);
            let origin =
                content.center() - Vec2::new(b.w, b.h) * scale / 2. - Vec2::new(b.x, b.y) * scale;
            WireframeApp::paint_scene(&ui.painter().with_clip_rect(content), scene, origin, scale);
        }
        _ => {
            let text = if scene.is_err() {
                "Preview unavailable"
            } else {
                "Empty page"
            };
            ui.painter().text(
                content.center(),
                Align2::CENTER_CENTER,
                text,
                FontId::proportional(14.),
                Color32::GRAY,
            );
        }
    }
}
