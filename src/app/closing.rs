// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;

impl WireframeApp {
    pub(super) fn capture_window_close(&mut self, ctx: &egui::Context) {
        #[cfg(target_os = "macos")]
        let native_request = self
            .native_termination
            .as_ref()
            .is_some_and(|gate| gate.take_request());
        #[cfg(not(target_os = "macos"))]
        let native_request = false;
        if !self.quit_allowed && (native_request || ctx.input(|i| i.viewport().close_requested())) {
            self.window_close_pending = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            if native_request {
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            }
            // Restore a minimized window so its retained input and any unsaved
            // confirmation can run in a UI pass before the close is acknowledged.
            if ctx.input(|i| i.viewport().minimized == Some(true)) {
                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
            }
            ctx.request_repaint();
        }
    }

    pub(super) fn finish_window_close(&mut self, ctx: &egui::Context) {
        if !self.window_close_pending || self.quit_allowed {
            return;
        }
        // Clipboard/save completion wakes the app. Do not decide from a
        // disabled field or from only the prefix of a split input batch.
        if self.save_busy() || self.pending_cut.is_some() {
            return;
        }
        if !self.deferred_text_input.is_empty()
            || self.deferred_text_focus.is_some()
            || !self.deferred_cut_input.is_empty()
        {
            ctx.request_repaint();
            return;
        }
        self.window_close_pending = false;
        self.commit_text_session(ctx);
        self.editor.commit();
        self.request(Pending::Quit, ctx);
    }

    pub(super) fn settle_native_close(&self) {
        #[cfg(target_os = "macos")]
        if !self.quit_allowed
            && !self.window_close_pending
            && self.startup_open.is_none()
            && !matches!(self.pending, Some(Pending::Quit))
            && !self.save_busy()
            && !self.quit_after_pdf
            && !self.image_export.quit_after
            && let Some(gate) = &self.native_termination
        {
            gate.cancel();
        }
    }
}
