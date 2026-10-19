// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Read-only recovery discovery; external paths may wait on OS access or a mount.
use super::*;
use std::sync::mpsc::{self, Receiver, TryRecvError};

#[derive(Default)]
pub(super) struct Found {
    offers: Vec<CopySummary>,
    errors: Vec<String>,
}
impl Found {
    fn read(copies: &WorkingCopies) -> Self {
        let mut found = Self::default();
        for result in [copies.list(), copies.list_archives()] {
            match result {
                Ok(entries) => {
                    for entry in entries {
                        match entry.summary {
                            Ok(s)
                                if s.has_local_changes
                                    || s.named_path.as_ref().is_some_and(|f| !f.exists()) =>
                            {
                                found.offers.push(s)
                            }
                            Ok(_) => {}
                            Err(e) => found.errors.push(format!(
                                "A recovery record was retained but couldn't be read: {e}"
                            )),
                        }
                    }
                }
                Err(e) => found.errors.push(e.to_string()),
            }
        }
        found
    }
}
pub(super) struct Scan {
    receiver: Receiver<Found>,
    show: bool,
    again: bool,
}
impl Scan {
    fn start(
        ctx: egui::Context,
        show: bool,
        read: impl FnOnce() -> Found + Send + 'static,
    ) -> Result<Self, String> {
        let (sender, receiver) = mpsc::channel();
        std::thread::Builder::new()
            .name("wireframe-recovery-scan".into())
            .spawn(move || {
                let _ = sender.send(read());
                ctx.request_repaint();
            })
            .map_err(|e| e.to_string())?;
        Ok(Self {
            receiver,
            show,
            again: false,
        })
    }
}
impl WireframeApp {
    pub(in crate::app) fn refresh_recoveries(&mut self, show: bool) {
        let Some(p) = &mut self.persistence else {
            return;
        };
        if let Some(scan) = &mut p.discovery {
            // Never add another OS-blocked worker. Discard this older result
            // and run one fresh scan when it completes.
            scan.again = true;
            scan.show |= show;
            return;
        }
        let copies = p.copies.clone();
        match Scan::start(p.ctx.clone(), show, move || Found::read(&copies)) {
            Ok(scan) => p.discovery = Some(scan),
            Err(error) => {
                p.discovery_errors =
                    vec![format!("Could not check local recovery records: {error}")];
                p.chooser |= show;
            }
        }
    }
    pub(in crate::app) fn dismiss_recoveries(&mut self) {
        if let Some(p) = &mut self.persistence {
            p.chooser = false;
            if let Some(scan) = &mut p.discovery {
                scan.show = false;
            }
        }
    }
    pub(in crate::app) fn poll_recoveries(&mut self) {
        let idle = self.file.is_none()
            && !self.editor.dirty()
            && !self.canvas_input_blocked()
            && self.drag.is_none()
            && self.library_drag.is_none()
            && self.library_press.is_none();
        let Some(p) = &mut self.persistence else {
            return;
        };
        let Some(scan) = &p.discovery else {
            return;
        };
        let found = match scan.receiver.try_recv() {
            Ok(found) => found,
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => Found {
                offers: vec![],
                errors: vec![
                    "The recovery check stopped unexpectedly. Existing records are unchanged."
                        .into(),
                ],
            },
        };
        let scan = p.discovery.take().unwrap();
        if scan.again {
            self.refresh_recoveries(scan.show);
            return;
        }
        p.offers = found
            .offers
            .into_iter()
            .filter(|offer| Some(&offer.id) != p.id.as_ref())
            .collect();
        p.discovery_errors = found.errors;
        let has_results = !p.offers.is_empty() || !p.discovery_errors.is_empty();
        let may_offer = scan.show
            && has_results
            && idle
            && !wireframe::text_entry::is_focused(&p.ctx)
            && !p.ctx.text_edit_focused()
            && !p.ctx.any_popup_open()
            // Discovery is polled before widgets receive this frame's input.
            // A first click/keystroke must get its target, not a new modal.
            && !p.ctx.input(|i| {
                i.pointer.any_down()
                    || i.pointer.any_pressed()
                    || i.events.iter().any(|e| {
                        matches!(e,
                            egui::Event::Text(_) | egui::Event::Paste(_)
                            | egui::Event::Ime(_) | egui::Event::AccessKitActionRequest(_)
                            | egui::Event::MouseWheel { .. }
                            | egui::Event::Key { pressed: true, .. })
                    })
            });
        p.chooser |= may_offer;
        if !p.discovery_errors.is_empty()
            && !p.chooser
            && self.startup_open.is_none()
            && p.busy.is_none()
        {
            self.status = "Recovery check needs attention · File → Recover work".into();
        }
    }
}

#[cfg(test)]
pub(super) mod tests;
