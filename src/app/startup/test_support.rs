// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! A test-only external-worker gate. Real preparation, validation and adoption stay intact.
use super::*;

pub(in crate::app) struct OpenGate {
    release: Option<mpsc::Sender<()>>,
    ready: mpsc::Receiver<()>,
}

impl OpenGate {
    pub(in crate::app) fn wait_ready(&self) {
        self.ready
            .recv_timeout(Duration::from_secs(10))
            .expect("owned Open worker reached prepared boundary");
    }

    pub(in crate::app) fn release(&mut self) {
        self.release
            .take()
            .expect("one Open release")
            .send(())
            .expect("owned Open worker still waiting");
    }
}

impl Drop for OpenGate {
    fn drop(&mut self) {
        // Unwind cannot leave the worker waiting for an unbounded channel read.
        if let Some(release) = self.release.take() {
            let _ = release.send(());
        }
    }
}

pub(in crate::app) fn gated_discard_open(
    app: &mut WireframeApp,
    ctx: &egui::Context,
    path: PathBuf,
) -> OpenGate {
    let copies = app.startup_copies().expect("isolated persistent fixture");
    let source = path.clone();
    let (release, wait) = mpsc::channel();
    let (started, ready) = mpsc::channel();
    let mut job = Opening::spawn(path, ctx.clone(), move || {
        let prepared = persistence::PreparedOpen::read(&copies, &source)?;
        started.send(()).map_err(|error| error.to_string())?;
        wait.recv_timeout(Duration::from_secs(30))
            .map_err(|error| error.to_string())?;
        Ok(Prepared::Persistent(Box::new(prepared)))
    })
    .expect("start owned fixture worker");
    job.phase = Phase::MenuRead { discard: true };
    app.startup_open = Some(job);
    OpenGate {
        release: Some(release),
        ready,
    }
}
