// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Offline project browsing and isolated preview/review.
use super::*;
use std::{
    collections::HashMap,
    sync::{Arc, mpsc},
};
use wireframe::{
    control_browser::{Catalog, Entry, Snapshot},
    render::Scene,
};
pub(super) mod review;
pub(super) mod view;
use review::{Choice, Review};
type LoadResult = Result<(Option<usize>, Snapshot), String>;

pub(super) struct Preview {
    index: usize,
    snapshot: Snapshot,
    editor: Editor,
    resource: Id,
}
impl Preview {
    fn new(index: usize, snapshot: Snapshot) -> Result<Self, String> {
        let resource = snapshot
            .initial_resource()
            .ok_or("This project has no viewable pages or component libraries")?;
        let editor = Editor::new(snapshot.project.clone());
        Ok(Self {
            index,
            snapshot,
            editor,
            resource,
        })
    }
}
pub(super) struct Browser {
    catalog: Catalog,
    preview: Option<Preview>,
    review: Option<Review>,
    loading: Option<mpsc::Receiver<LoadResult>>,
    thumbnails: HashMap<String, Result<Arc<Scene>, String>>,
    error: Option<String>,
    focus_search: bool,
}
impl Default for Browser {
    fn default() -> Self {
        Self {
            catalog: Catalog::default(),
            preview: None,
            review: None,
            loading: None,
            thumbnails: HashMap::new(),
            error: None,
            focus_search: true,
        }
    }
}
impl Browser {
    fn load(&mut self, ctx: &egui::Context, index: Option<usize>, entry: Entry) {
        let (tx, rx) = mpsc::channel();
        let ctx = ctx.clone();
        self.loading = Some(rx);
        self.error = None;
        self.review = None;
        std::thread::spawn(move || {
            let result = entry.load().map(|snapshot| (index, snapshot));
            let _ = tx.send(result);
            ctx.request_repaint();
        });
    }
    fn poll(&mut self) {
        let Some(receiver) = &self.loading else {
            return;
        };
        let result = match receiver.try_recv() {
            Ok(v) => v,
            Err(mpsc::TryRecvError::Empty) => return,
            Err(_) => Err("Project loading stopped. Try opening it again.".into()),
        };
        self.loading = None;
        match result.and_then(|(index, snapshot)| {
            let index = match index {
                Some(i) => {
                    if matches!(
                        self.catalog.entries[i].source,
                        wireframe::control_browser::Source::Local(_)
                    ) {
                        self.catalog.add_local(&snapshot)?;
                    }
                    i
                }
                None => {
                    self.catalog.query.clear();
                    self.catalog.category = "Local projects".into();
                    self.catalog.add_local(&snapshot)?
                }
            };
            self.thumbnails.remove(&self.catalog.entries[index].id);
            Preview::new(index, snapshot)
        }) {
            Ok(p) => {
                if self.thumbnails.len() >= 32 {
                    self.thumbnails.clear();
                }
                self.thumbnails.insert(
                    self.catalog.entries[p.index].id.clone(),
                    p.editor.rendered_scene(p.resource).map(Arc::new),
                );
                self.preview = Some(p);
                self.error = None;
            }
            Err(e) => self.error = Some(e),
        }
    }
    fn start_review(&mut self, destination: &Editor) {
        let Some(preview) = &self.preview else {
            return;
        };
        match Review::new(destination, &preview.snapshot) {
            Ok(r) => {
                self.review = Some(r);
                self.error = None;
            }
            Err(e) => self.error = Some(e),
        }
    }
}
impl WireframeApp {
    pub(super) fn open_more_controls(&mut self) {
        if self.canvas_input_blocked() || self.image_crop.is_some() || !self.finish_page_notes() {
            return;
        }
        self.end_pointer_gestures();
        self.more_controls = Some(Browser::default());
    }
    pub(super) fn more_controls_dialog(&mut self, ctx: &egui::Context) {
        let Some(mut browser) = self.more_controls.take() else {
            return;
        };
        browser.poll();
        let action = view::show(ctx, &mut browser, &self.editor);
        let mut close = false;
        match action {
            view::Action::None => {}
            view::Action::Close => close = true,
            view::Action::Back => {
                browser.preview = None;
                browser.review = None;
                browser.error = None;
                browser.focus_search = true;
            }
            view::Action::CancelReview => {
                browser.review = None;
                browser.error = None;
            }
            view::Action::Open(i) => browser.load(ctx, Some(i), browser.catalog.entries[i].clone()),
            view::Action::Local => {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Wireframe project", &["wireframe"])
                    .pick_file()
                {
                    browser.load(
                        ctx,
                        None,
                        Entry {
                            id: String::new(),
                            title: String::new(),
                            category: String::new(),
                            description: String::new(),
                            source: wireframe::control_browser::Source::Local(path),
                        },
                    );
                }
            }
            view::Action::Review => browser.start_review(&self.editor),
            view::Action::Choose(choice) => {
                if let Some(review) = &mut browser.review {
                    review.choose(choice);
                }
            }
            view::Action::Apply => {
                let result = browser
                    .review
                    .as_mut()
                    .ok_or_else(|| "No import is ready".to_owned())
                    .and_then(|review| {
                        review.take_prepared(
                            &self.editor,
                            &browser.preview.as_ref().unwrap().snapshot,
                        )
                    });
                match result.and_then(|prepared| self.editor.apply_project_copy(prepared)) {
                    Ok(counts) => {
                        self.collapse_resource_selection();
                        self.status = format!(
                            "Imported project: {} pages added, {} replaced; {} libraries added, {} replaced",
                            counts.pages,
                            counts.pages_replaced,
                            counts.libraries,
                            counts.libraries_replaced
                        );
                        close = true;
                    }
                    Err(error) => browser.error = Some(error),
                }
            }
        }
        if close {
            if let Some(id) = self.canvas_focus_id {
                ctx.memory_mut(|m| m.request_focus(id));
            }
        } else {
            self.more_controls = Some(browser);
        }
    }
}
#[cfg(test)]
mod tests;
