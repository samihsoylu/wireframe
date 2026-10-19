// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;

impl WireframeApp {
    pub(super) fn capture_window_close(&mut self, ctx: &egui::Context) {
        if !self.quit_allowed && ctx.input(|i| i.viewport().close_requested()) {
            self.window_close_pending = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
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
}
