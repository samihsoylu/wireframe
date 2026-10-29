// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Transient recognition/copy preview. Deliberately has no Editor reference.
use eframe::egui;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use wireframe_ocr::Recognition;

static RECOGNITION_ACTIVE: AtomicBool = AtomicBool::new(false);
#[cfg(test)]
pub(super) static TEST_RECOGNITION: std::sync::Mutex<()> = std::sync::Mutex::new(());
pub(super) fn available() -> bool {
    !RECOGNITION_ACTIVE.load(Ordering::Acquire)
}
struct RecognitionPermit;
impl RecognitionPermit {
    fn acquire() -> Option<Self> {
        RECOGNITION_ACTIVE
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .ok()
            .map(|_| Self)
    }
}
impl Drop for RecognitionPermit {
    fn drop(&mut self) {
        RECOGNITION_ACTIVE.store(false, Ordering::Release);
    }
}

pub(super) struct Review {
    receiver: Option<mpsc::Receiver<Result<Recognition, wireframe_ocr::Error>>>,
    cancel: Arc<AtomicBool>,
    text: String,
    ready: bool,
    no_text_found: bool,
    copied: bool,
    error: Option<String>,
}

impl Drop for Review {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

impl Review {
    /// Opening this preview is the explicit recognition action. The supplied
    /// bytes are owned by the worker and never assigned to project assets.
    pub(super) fn new(ctx: &egui::Context, bytes: Vec<u8>) -> Self {
        let cancel = Arc::new(AtomicBool::new(false));
        let (sender, receiver) = mpsc::channel();
        let mut review = Self {
            receiver: Some(receiver),
            cancel: cancel.clone(),
            text: String::new(),
            ready: false,
            no_text_found: false,
            copied: false,
            error: None,
        };
        let Some(permit) = RecognitionPermit::acquire() else {
            review.receiver = None;
            review.error = Some("Previous text extraction is finishing. Close this preview and try again in a moment.".into());
            return review;
        };
        let ctx = ctx.clone();
        if std::thread::Builder::new()
            .name("screenshot-text".into())
            .spawn(move || {
                let _permit = permit;
                let result = wireframe_ocr::recognize(&bytes, &cancel);
                let _ = sender.send(result);
                ctx.request_repaint();
            })
            .is_err()
        {
            review.receiver = None;
            review.error =
                Some("Couldn't start text recognition. Close this preview and try again.".into());
        }
        review
    }

    fn poll(&mut self) {
        let Some(receiver) = &self.receiver else {
            return;
        };
        match receiver.try_recv() {
            Ok(result) => {
                self.receiver = None;
                match result {
                    Ok(result) => {
                        self.no_text_found = result.text.is_empty();
                        self.text = result.text;
                        self.ready = true;
                    }
                    Err(error) => self.error = Some(error.to_string()),
                }
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                self.receiver = None;
                self.error =
                    Some("Text recognition stopped. Close this preview and try again.".into());
            }
            Err(mpsc::TryRecvError::Empty) => {}
        }
    }

    /// Returns true on Close/Cancel/Escape. Copying only requests a clipboard
    /// write; editing this local text never changes the project or its history.
    pub(super) fn show(&mut self, ctx: &egui::Context) -> bool {
        self.poll();
        let mut close = false;
        let response = egui::Modal::new(egui::Id::new("screenshot-text-preview")).show(ctx, |ui| {
            ui.set_width((ctx.viewport_rect().width() - 64.).clamp(220., 560.));
            ui.heading("Extract text");
            ui.label("Recognized on this device. English text works best.");
            if self.receiver.is_some() {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("Reading the screenshot…");
                });
                ctx.request_repaint_after(std::time::Duration::from_millis(100));
            }
            if self.ready {
                if self.no_text_found {
                    ui.label(
                        "No text found. Try a clearer screenshot with larger, horizontal text.",
                    );
                } else {
                    ui.label("Review and correct the text before copying.");
                    egui::ScrollArea::vertical()
                        .id_salt("screenshot-text-scroll")
                        .max_height((ctx.viewport_rect().height() - 230.).clamp(80., 320.))
                        .show(ui, |ui| {
                            if ui
                                .add(
                                    egui::TextEdit::multiline(&mut self.text)
                                        .desired_width(f32::INFINITY)
                                        .desired_rows(12),
                                )
                                .changed()
                            {
                                self.copied = false;
                            }
                        });
                }
            }
            if let Some(error) = &self.error {
                ui.colored_label(egui::Color32::DARK_RED, error);
            }
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(
                        self.ready && !self.text.trim().is_empty(),
                        egui::Button::new("Copy text"),
                    )
                    .clicked()
                {
                    ctx.copy_text(self.text.clone());
                    self.copied = true;
                }
                if self.copied {
                    ui.label("Copied");
                }
                close = ui
                    .button(if self.receiver.is_some() {
                        "Cancel"
                    } else {
                        "Close"
                    })
                    .clicked();
            });
        });
        close |= response.is_top_modal
            && !response.any_popup_open
            && ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
        if close {
            self.cancel.store(true, Ordering::Relaxed);
        }
        close
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn completed(text: &str) -> Review {
        let (sender, receiver) = mpsc::channel();
        sender
            .send(Ok(Recognition {
                text: text.into(),
                confidence: 90,
            }))
            .unwrap();
        Review {
            receiver: Some(receiver),
            cancel: Arc::new(AtomicBool::new(false)),
            text: String::new(),
            ready: false,
            no_text_found: false,
            copied: false,
            error: None,
        }
    }
    #[test]
    fn results_remain_transient_and_escape_cancels() {
        let ctx = egui::Context::default();
        let mut review = completed("Project overview");
        let mut closed = false;
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            closed = review.show(ui.ctx());
        });
        output.textures_delta.clear();
        assert!(!closed);
        assert_eq!(review.text, "Project overview");
        assert!(review.ready);
        assert!(review.receiver.is_none());
        assert!(!review.cancel.load(Ordering::Relaxed));
        let input = egui::RawInput {
            events: vec![egui::Event::Key {
                key: egui::Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| {
            closed = review.show(ui.ctx());
        });
        output.textures_delta.clear();
        assert!(closed);
        assert!(review.cancel.load(Ordering::Relaxed));
    }
    #[test]
    fn empty_results_and_disconnected_worker_have_clear_states() {
        let mut empty = completed("");
        empty.poll();
        assert!(empty.no_text_found);
        assert!(empty.ready);
        let (sender, receiver) = mpsc::channel();
        drop(sender);
        empty.receiver = Some(receiver);
        empty.poll();
        assert!(
            empty
                .error
                .as_deref()
                .is_some_and(|text| text.contains("stopped"))
        );
    }
    #[test]
    fn closing_a_preview_cancels_its_worker() {
        let review = completed("text");
        let cancel = review.cancel.clone();
        drop(review);
        assert!(cancel.load(Ordering::Relaxed));
    }
    #[test]
    fn cancelled_and_reopened_previews_wait_for_the_previous_worker_to_exit() {
        let _guard = TEST_RECOGNITION.lock().unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while !available() {
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let permit = RecognitionPermit::acquire().unwrap();
        let (release, held) = mpsc::channel();
        let (started, ready) = mpsc::channel();
        let worker = std::thread::spawn(move || {
            let _permit = permit;
            started.send(()).unwrap();
            held.recv_timeout(std::time::Duration::from_secs(3))
                .unwrap();
        });
        ready
            .recv_timeout(std::time::Duration::from_secs(3))
            .unwrap();
        let old = completed("old result");
        let cancel = old.cancel.clone();
        drop(old);
        assert!(cancel.load(Ordering::Relaxed));
        let ctx = egui::Context::default();
        for _ in 0..32 {
            let new = Review::new(&ctx, vec![]);
            assert!(new.receiver.is_none());
            assert!(
                new.error
                    .as_deref()
                    .unwrap()
                    .contains("Previous text extraction")
            );
            assert!(!available());
        }
        release.send(()).unwrap();
        worker.join().unwrap();
        assert!(available());
        let mut next = Review::new(&ctx, b"not an image".to_vec());
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        while next.receiver.is_some() {
            next.poll();
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(next.error.as_deref().unwrap().contains("Choose a PNG"));
        while !available() {
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
}
