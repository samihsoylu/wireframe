// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Headless full-application frames. No native backend consumes their output.
use super::*;
use eframe::App;
use egui::accesskit::{Action, ActionRequest, NodeId, Role, TreeId};
use sha2::{Digest, Sha256};
use std::{collections::VecDeque, path::Path, sync::Arc};

/// Shared with retained full-frame regressions. Caller owns time/events; this
/// function only applies the production raw-input hook and application UI.
pub(super) fn application_frame(
    ctx: &egui::Context,
    app: &mut WireframeApp,
    mut raw: egui::RawInput,
) -> (egui::FullOutput, Option<egui::Response>, Vec<egui::Event>) {
    app.raw_input_hook(ctx, &mut raw);
    let hook_events = raw.events.clone();
    let mut focus = None;
    let output = ctx.run_ui(raw, |ui| {
        app.ui(ui, &mut eframe::Frame::_new_kittest());
        focus = ctx
            .memory(|memory| memory.focused())
            .and_then(|id| ctx.read_response(id));
    });
    (output, focus, hook_events)
}

#[derive(Clone, Debug)]
pub(super) struct LiveNode {
    pub id: NodeId,
    pub role: Role,
    pub name: String,
    pub value: Option<String>,
    pub rect: Option<Rect>,
    pub disabled: bool,
}

/// An external file-drop boundary, never a native chooser.
#[derive(Debug)]
struct ProjectDrop(PathBuf);
impl egui::DroppedFile for ProjectDrop {
    fn path(&self) -> &Path {
        &self.0
    }
    #[cfg(not(target_arch = "wasm32"))]
    fn bytes(&self) -> Result<Vec<u8>, String> {
        std::fs::read(&self.0).map_err(|error| error.to_string())
    }
}

pub(super) struct Driver {
    pub ctx: egui::Context,
    // App/session fields drop before the temporary directory they can write.
    pub app: WireframeApp,
    root: Arc<tempfile::TempDir>,
    size: Vec2,
    frame: u64,
    nodes: Vec<LiveNode>,
    trace: VecDeque<String>,
    pub last_hook_events: Vec<egui::Event>,
}

impl Driver {
    pub fn new() -> Self {
        Self::with_root(Arc::new(
            tempfile::tempdir().expect("isolated scenario directory"),
        ))
    }

    fn with_root(root: Arc<tempfile::TempDir>) -> Self {
        assert!(
            std::env::var_os("WIREFRAME_INPUT_TRACE").is_none(),
            "unset external input trace output for isolated scenarios"
        );
        let ctx = egui::Context::default();
        ctx.set_pixels_per_point(1.);
        ctx.enable_accesskit();
        let mut app = WireframeApp::workspace(&ctx, None);
        app.show_library = false;
        app.show_navigator = false;
        assert!(app.persistence.is_none() && app.recovery.is_none());
        Self {
            ctx,
            app,
            root,
            size: Vec2::new(1360., 1000.),
            frame: 0,
            nodes: Vec::new(),
            trace: VecDeque::new(),
            last_hook_events: Vec::new(),
        }
    }

    /// New application/context/history, same explicitly owned fixture files.
    pub fn fresh(&self) -> Self {
        Self::with_root(self.root.clone())
    }

    pub fn size(&mut self, size: Vec2) {
        self.size = size;
    }

    pub fn path(&self, relative: &str) -> PathBuf {
        let path = Path::new(relative);
        assert!(path.is_relative());
        assert!(
            path.components()
                .all(|part| !matches!(part, std::path::Component::ParentDir))
        );
        self.root
            .path()
            .canonicalize()
            .expect("fixture root exists")
            .join(path)
    }

    pub fn enable_manual_persistence(&mut self) {
        self.app.enable_persistence(&self.ctx, self.path("state"));
        self.app.set_save_mode(false);
        self.check(self.app.error.is_none(), "isolated persistence setup");
    }

    pub fn frame(&mut self, events: Vec<egui::Event>) {
        self.input(events, Modifiers::NONE, true, Vec::new());
    }

    pub fn input(
        &mut self,
        mut events: Vec<egui::Event>,
        modifiers: Modifiers,
        focused: bool,
        dropped_files: Vec<egui::DroppedFileHandle>,
    ) {
        events.insert(0, egui::Event::ModifiersChanged(modifiers));
        let time = self.frame as f64 / 60.;
        let input = format!(
            "frame={} time={time:.6} focused={focused} modifiers={modifiers:?} events={events:?}",
            self.frame
        );
        self.trace.push_back(input);
        if self.trace.len() > 48 {
            self.trace.pop_front();
        }
        let raw = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, self.size)),
            time: Some(time),
            events,
            focused,
            dropped_files,
            ..Default::default()
        };
        let (out, _, hook_events) = application_frame(&self.ctx, &mut self.app, raw);
        self.last_hook_events = hook_events;
        // Only current-frame nodes may identify an action target.
        self.nodes = out
            .platform_output
            .accesskit_update
            .as_ref()
            .map(|update| {
                update
                    .nodes
                    .iter()
                    .map(|(id, node)| LiveNode {
                        id: *id,
                        role: node.role(),
                        name: node
                            .label()
                            .or_else(|| node.value())
                            .unwrap_or_default()
                            .to_owned(),
                        value: node.value().map(str::to_owned),
                        rect: node.bounds().map(|r| {
                            Rect::from_min_max(
                                egui::pos2(r.x0 as f32, r.y0 as f32),
                                egui::pos2(r.x1 as f32, r.y1 as f32),
                            )
                        }),
                        disabled: node.is_disabled(),
                    })
                    .collect()
            })
            .unwrap_or_default();
        out.drop_without_applying_deltas();
        self.frame += 1;
        let state = self.state();
        if let Some(trace) = self.trace.back_mut() {
            trace.push_str(&format!("\n  {state}"));
        }
    }

    pub fn idle(&mut self, frames: usize) {
        for _ in 0..frames {
            self.frame(Vec::new());
        }
    }

    pub fn until(&mut self, label: &str, frames: usize, ready: impl Fn(&Self) -> bool) {
        for _ in 0..frames {
            if ready(self) {
                return;
            }
            self.frame(Vec::new());
        }
        self.check(ready(self), label);
    }

    pub fn key(key: Key, pressed: bool, modifiers: Modifiers) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers,
        }
    }

    pub fn press(&mut self, key: Key, modifiers: Modifiers) {
        self.input(
            vec![Self::key(key, true, modifiers)],
            modifiers,
            true,
            Vec::new(),
        );
    }

    pub fn release(&mut self, key: Key, modifiers: Modifiers) {
        self.input(
            vec![Self::key(key, false, modifiers)],
            modifiers,
            true,
            Vec::new(),
        );
    }

    pub fn tap(&mut self, key: Key, modifiers: Modifiers) {
        self.press(key, modifiers);
        self.release(key, modifiers);
    }

    pub fn pointer(pos: Pos2, pressed: bool, modifiers: Modifiers) -> egui::Event {
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers,
        }
    }

    pub fn click_point(&mut self, point: Pos2) {
        for pressed in [true, false] {
            self.frame(vec![
                egui::Event::PointerMoved(point),
                Self::pointer(point, pressed, Modifiers::NONE),
            ]);
        }
    }

    pub fn unique(&self, name: &str) -> LiveNode {
        let matches: Vec<_> = self
            .nodes
            .iter()
            .filter(|node| node.name == name && !node.disabled)
            .collect();
        assert_eq!(
            matches.len(),
            1,
            "expected one enabled {name:?}; {:?}\n{}",
            self.nodes,
            self.diagnostics()
        );
        matches[0].clone()
    }

    pub fn unique_combo_value(&self, value: &str) -> LiveNode {
        let matches: Vec<_> = self
            .nodes
            .iter()
            .filter(|node| {
                node.role == Role::ComboBox
                    && node.value.as_deref() == Some(value)
                    && !node.disabled
            })
            .collect();
        assert_eq!(
            matches.len(),
            1,
            "expected one enabled ComboBox value {value:?}; {:?}\n{}",
            self.nodes,
            self.diagnostics()
        );
        matches[0].clone()
    }

    /// Deliberate fixture focus, never an implicit substitute for Tab traversal.
    pub fn focus_action(&mut self, id: NodeId) {
        self.frame(vec![egui::Event::AccessKitActionRequest(ActionRequest {
            action: Action::Focus,
            target_tree: TreeId::ROOT,
            target_node: id,
            data: None,
        })]);
    }

    pub fn click(&mut self, name: &str) {
        self.frame(Vec::new());
        let node = self.unique(name);
        let rect = node.rect.expect("current action has bounds");
        self.check(
            rect.intersects(Rect::from_min_size(Pos2::ZERO, self.size)),
            "action is visible",
        );
        self.click_point(rect.center());
    }

    /// Distinguish a newly opened menu action from its still-present toolbar peer.
    pub fn click_menu_action(&mut self, menu: &str, name: &str) {
        self.frame(Vec::new());
        self.check(
            !egui::Popup::is_any_open(&self.ctx),
            "menu targeting starts closed",
        );
        let toolbar = self.unique(name);
        self.check(
            toolbar.role == Role::Button,
            "pre-existing action is a toolbar button",
        );
        let old_ids: Vec<_> = self
            .nodes
            .iter()
            .filter(|node| node.name == name && !node.disabled)
            .map(|node| node.id)
            .collect();
        self.click(menu);
        self.frame(Vec::new());
        let added: Vec<_> = self
            .nodes
            .iter()
            .filter(|node| {
                node.name == name
                    && node.role == Role::Button
                    && !node.disabled
                    && !old_ids.contains(&node.id)
            })
            .cloned()
            .collect();
        assert_eq!(
            added.len(),
            1,
            "expected one newly observed menu {name:?}; old={old_ids:?}; nodes={:?}\n{}",
            self.nodes,
            self.diagnostics()
        );
        let target = added[0].clone();
        // Re-read a complete current frame; neither identity nor bounds may be stale.
        self.frame(Vec::new());
        self.check(
            egui::Popup::is_any_open(&self.ctx),
            "observed menu remains open",
        );
        let current: Vec<_> = self
            .nodes
            .iter()
            .filter(|node| {
                node.name == name
                    && node.role == Role::Button
                    && !node.disabled
                    && !old_ids.contains(&node.id)
            })
            .collect();
        assert_eq!(
            current.len(),
            1,
            "menu target must remain unique; {:?}\n{}",
            self.nodes,
            self.diagnostics()
        );
        self.check(
            current[0].id == target.id && current[0].rect == target.rect,
            "current menu identity and bounds revalidated",
        );
        self.check(
            self.nodes
                .iter()
                .filter(|node| {
                    node.id == toolbar.id
                        && node.name == name
                        && node.role == Role::Button
                        && !node.disabled
                })
                .count()
                == 1,
            "toolbar peer remains separately present",
        );
        let rect = current[0].rect.expect("current menu action has bounds");
        self.check(
            rect.is_positive() && Rect::from_min_size(Pos2::ZERO, self.size).contains_rect(rect),
            "menu target is visible",
        );
        eprintln!(
            "OBSERVED_MENU_ACTION menu={menu:?} action={name:?} toolbar={:?} menu_target={:?} bounds={rect:?}",
            toolbar.id, target.id
        );
        self.click_point(rect.center());
    }

    fn visible_button(&self, name: &str) -> Option<LiveNode> {
        let mut matches = self
            .nodes
            .iter()
            .filter(|node| node.name == name && node.role == Role::Button && !node.disabled);
        let node = matches.next()?;
        if matches.next().is_some()
            || !node.rect.is_some_and(|rect| {
                rect.is_positive() && Rect::from_min_size(Pos2::ZERO, self.size).contains_rect(rect)
            })
        {
            return None;
        }
        Some(node.clone())
    }

    fn dialog_ready(&self, message: &str, actions: &[&str]) -> bool {
        self.ctx.memory(|m| m.top_modal_layer()).is_some()
            // A static Label is observed content, not an enabled action.
            && self.nodes.iter().filter(|node| {
                node.role == Role::Label && node.name == message
            }).count() == 1
            && actions.iter().all(|name| self.visible_button(name).is_some())
    }

    /// Wait for exact dialog content and live actions, then revalidate the target.
    pub fn click_dialog_action(&mut self, message: &str, actions: &[&str], action: &str) {
        self.check(
            actions.contains(&action),
            "requested action belongs to dialog contract",
        );
        self.frame(Vec::new());
        self.until(
            "dialog content and enabled visible actions ready",
            16,
            |d| d.dialog_ready(message, actions),
        );
        let target = self.visible_button(action).unwrap();
        self.frame(Vec::new());
        self.check(
            self.dialog_ready(message, actions),
            "dialog remains current",
        );
        let current = self.visible_button(action).unwrap();
        self.check(
            current.id == target.id && current.rect == target.rect,
            "dialog action identity and bounds revalidated",
        );
        eprintln!(
            "OBSERVED_DIALOG_ACTION message={message:?} action={action:?} target={:?} bounds={:?}",
            current.id, current.rect
        );
        self.click_point(current.rect.unwrap().center());
    }

    /// Observe the captured dialog gone and its underlying action available.
    pub fn wait_dialog_dismissed(&mut self, message: &str, next_action: &str) {
        self.frame(Vec::new());
        self.until(
            "captured dialog dismissed and underlying action ready",
            16,
            |d| {
                !d.nodes.iter().any(|node| node.name == message)
                    && d.visible_button(next_action).is_some()
            },
        );
    }

    pub fn wait_document_ready(&mut self) {
        self.frame(Vec::new());
        self.until("document commands ready after modal dismissal", 16, |d| {
            d.ctx.memory(|m| m.top_modal_layer()).is_none()
                && d.app.error.is_none()
                && d.app.trash.is_none()
                && d.app.startup_open.is_none()
                && !d.app.save_busy()
                && !egui::Popup::is_any_open(&d.ctx)
        });
    }

    pub fn focused(&self) -> Option<NodeId> {
        self.ctx
            .memory(|memory| memory.focused())
            .map(|id| id.accesskit_id())
    }

    pub fn focus_name(&self) -> Option<&str> {
        let id = self.focused()?;
        self.nodes
            .iter()
            .find(|node| node.id == id)
            .map(|node| node.name.as_str())
    }

    pub fn drop_project(&mut self, path: PathBuf) {
        self.check(
            path.starts_with(
                self.root
                    .path()
                    .canonicalize()
                    .expect("fixture root exists"),
            ),
            "drop belongs to scenario root",
        );
        self.input(
            Vec::new(),
            Modifiers::NONE,
            true,
            vec![Arc::new(ProjectDrop(path))],
        );
    }

    /// UI time stays fixed-step; real IO completion has its own explicit bound.
    pub fn wait_io(&mut self, label: &str, ready: impl Fn(&Self) -> bool) {
        let deadline = Instant::now() + Duration::from_secs(10);
        for _ in 0..600 {
            if ready(self) {
                return;
            }
            self.check(Instant::now() < deadline, label);
            self.frame(Vec::new());
            if !ready(self) {
                std::thread::sleep(Duration::from_millis(1));
            }
        }
        self.check(ready(self), label);
    }

    pub fn project(&self, expected: &Project, label: &str) {
        self.project_value(&self.app.editor.document, expected, label);
    }

    pub fn project_value(&self, actual: &Project, expected: &Project, label: &str) {
        // Full equality includes font bytes. Only the failure presentation is
        // condensed; a font payload must not bury geometry/history evidence.
        if actual != expected {
            let concise = |project: &Project| {
                let mut json = serde_json::to_value(project).expect("scenario project JSON");
                if project.font.is_some() {
                    let bytes = serde_json::to_vec(&project.font).expect("scenario font JSON");
                    json["font"] = serde_json::json!({
                        "serialized_bytes": bytes.len(),
                        "sha256": Sha256::digest(&bytes).iter().map(|byte| format!("{byte:02x}")).collect::<String>(),
                    });
                }
                json
            };
            panic!(
                "{label}\nactual={}\nexpected={}\n{}",
                concise(actual),
                concise(expected),
                self.diagnostics()
            );
        }
    }

    pub fn finish_session(&mut self, label: &str) {
        let finished = self.app.finish_session(false);
        self.check(finished, label);
    }

    pub fn check(&self, condition: bool, label: &str) {
        assert!(condition, "{label}\n{}", self.diagnostics());
    }

    fn state(&self) -> String {
        format!(
            "focus={:?}/{:?}/{:?} popup={} text={:?} drag={} open={} save={} pending={} error={:?} page={} selection={:?} undo={:?} redo={:?} dirty={}",
            self.focused(),
            self.focus_name(),
            self.nodes
                .iter()
                .find(|node| Some(node.id) == self.focused())
                .map(|node| node.role),
            egui::Popup::is_any_open(&self.ctx),
            self.app
                .text_edit
                .as_ref()
                .map(|(id, text)| (*id, text.as_str())),
            self.app.drag.is_some(),
            self.app.startup_open.is_some(),
            self.app.save_busy(),
            self.app.pending.is_some(),
            self.app.error,
            self.app.editor.page,
            self.app.editor.selection,
            self.app.editor.undo_name(),
            self.app.editor.redo_name(),
            self.app.editor.dirty()
        )
    }

    pub fn diagnostics(&self) -> String {
        format!(
            "{}\nscenario root={}\n{}",
            self.state(),
            self.root.path().display(),
            self.trace.iter().cloned().collect::<Vec<_>>().join("\n")
        )
    }
}

impl Drop for Driver {
    fn drop(&mut self) {
        if std::thread::panicking() {
            eprintln!("application scenario failure:\n{}", self.diagnostics());
        }
    }
}
