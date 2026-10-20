// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use eframe::egui;
impl Editor {
    /// Conversion changes structure, so component instances and locked owners
    /// keep their current draft until the user accepts or cancels it normally.
    pub fn can_transform_text(&self, id: Id) -> bool {
        self.selection.len() == 1
            && self.selection.contains(&id)
            && self.require_structure_edit().is_ok()
            && !self.position_locked(id)
    }

    /// Replace a single-line text control with its multiline counterpart.
    /// The caller commits the old draft first, making conversion its own Undo.
    pub fn transform_text_control(&mut self, id: Id) -> Result<Option<Id>, String> {
        if !self.can_transform_text(id) {
            return Ok(None);
        }
        let source = self
            .nodes()
            .iter()
            .find(|n| n.id == id)
            .cloned()
            .ok_or("Text target no longer exists")?;
        let NodeContent::Control { kind, .. } = &source.content else {
            return Ok(None);
        };
        let target = match kind.as_str() {
            "text_input" => "text_area",
            "label" => "paragraph",
            _ => return Ok(None),
        };
        let spec = catalog::get(target).ok_or("Unknown text control")?;
        let properties = catalog::get(kind)
            .ok_or("Unknown text control")?
            .effective(&source, &self.property_context())
            .map_err(|e| e.to_string())?;
        // Preflight ID exhaustion before opening the history operation.
        let mut allocation = self.document.clone();
        let new_id = allocation.allocate_id().map_err(|e| e.to_string())?;
        let mut replacement = spec.create(new_id, source.origin);
        replacement.markup = source.markup;
        replacement.crop = source.crop;
        for property in &spec.properties {
            if let Some(value) = properties.get(property.key) {
                replacement
                    .properties
                    .insert(property.key.into(), value.clone());
            }
        }
        self.edit("Convert to multiline", |e| {
            e.document
                .allocate_id()
                .expect("ID allocation was preflighted");
            if let Some(node) = e.nodes_mut().iter_mut().find(|n| n.id == id) {
                *node = replacement;
            }
            let context = e.context.clone();
            for path in &mut e.active_content_mut().paint_order {
                if path.starts_with(&context) && path.get(context.len()) == Some(&id) {
                    path[context.len()] = new_id;
                }
            }
            e.selection = std::collections::HashSet::from([new_id]);
        });
        Ok(Some(new_id))
    }

    /// Automatic text width grows around its alignment anchor. Explicit moves
    /// and fixed-width controls retain their requested geometry.
    pub(super) fn preserve_text_anchor(
        &self,
        before: &Node,
        after: &mut Node,
    ) -> Result<(), String> {
        let NodeContent::Control { kind, size } = &after.content else {
            return Ok(());
        };
        if !matches!(kind.as_str(), "label" | "heading" | "subtitle")
            || size.width != Dimension::Auto
            || before.origin != after.origin
            || (before.properties.get("text") == after.properties.get("text")
                && before.properties.get("font_size") == after.properties.get("font_size")
                && before.content == after.content)
        {
            return Ok(());
        }
        let properties = catalog::get(kind)
            .ok_or("Unknown text control")?
            .effective(after, &self.property_context())
            .map_err(|e| e.to_string())?;
        if kind == "label"
            && crate::text_controls::LabelOrientation::from_properties(&properties)
                != crate::text_controls::LabelOrientation::Upright
        {
            return Ok(());
        }
        let align = match properties.get("align") {
            Some(Value::Choice(value)) if value == "center" || value == "right" => value.as_str(),
            _ => return Ok(()),
        };
        // Refresh project fonts before measuring a prospective node. This uses
        // the same layout engine as the canvas without changing the document.
        self.layout_page(self.surface_id())?;
        let title = self
            .surface(self.surface_id())
            .map_or("", |(title, _)| title);
        let mut widths = Err("Text measurement did not run".to_owned());
        self.fonts
            .run_ui(egui::RawInput::default(), |ui| {
                let measurer = crate::layout::EguiMeasurer {
                    context: ui.ctx(),
                    regular: crate::fonts::family(false, false),
                    bold: crate::fonts::family(true, false),
                };
                let engine = crate::layout::Engine {
                    project: &self.document,
                    measurer: &measurer,
                    context: catalog::Context::default(),
                };
                let width = |node: &Node| {
                    engine
                        .layout_named(
                            &Content {
                                nodes: vec![node.clone()],
                                paint_order: vec![vec![node.id]],
                            },
                            title,
                        )
                        .map(|layout| layout.nodes[0].uncropped.width)
                };
                widths = width(before).and_then(|old| width(after).map(|new| (old, new)));
            })
            .drop_without_applying_deltas();
        let (old, new) = widths?;
        let offset = if align == "center" {
            (old / 2.).floor() - (new / 2.).floor()
        } else {
            old - new
        };
        after.origin.x += offset;
        Ok(())
    }
    /// Render a text draft in an isolated projection. Recovery and history keep
    /// the committed project until the editing session is accepted.
    pub fn text_preview_scene(&self, id: Id, text: &str) -> Result<crate::render::Scene, String> {
        let text = self.control_text_from_edit(id, text)?;
        let mut preview = Self::with_clock(self.document.clone(), self.clock);
        preview.date = self.date;
        preview.markup_visible = self.markup_visible;
        preview.frozen_time = self.frozen_time;
        preview.active_versions = self.active_versions.clone();
        preview.fonts = self.fonts.clone();
        preview.restore(self.snapshot());
        let mut control = preview
            .current()
            .controls
            .into_iter()
            .find(|c| c.id == id)
            .ok_or("Text target no longer exists")?;
        control.text = text;
        preview.update_control(control);
        preview.editing_scene()
    }
    /// Prepare an editor session. Placeholder conversion is a committed operation,
    /// so cancelling the subsequent text draft leaves the conversion undoable.
    pub fn prepare_text_edit(&mut self, id: Id) -> Result<Option<(Id, String)>, String> {
        let Some(view) = self.current().controls.into_iter().find(|c| c.id == id) else {
            return Ok(None);
        };
        if view.group.is_some() || !view.kind.text_editable() {
            return Ok(None);
        }
        let target = match view.kind {
            Kind::TextBlock => Some("paragraph"),
            Kind::TextLine => Some("subtitle"),
            _ => None,
        };
        if let Some(target) = target {
            if self.position_locked(id) {
                return Ok(None);
            }
            self.require_structure_edit()?;
            self.edit("Convert to text", |e| {
                if let Some(node) = e.nodes_mut().iter_mut().find(|n| n.id == id) {
                    // Retain identity, origin, ordering, markup and crop. References to
                    // this object remain valid throughout conversion and undo/redo.
                    node.content = NodeContent::Control {
                        kind: target.into(),
                        size: Size {
                            width: Dimension::Fixed(view.bounds.w),
                            height: if target == "subtitle" {
                                Dimension::Auto
                            } else {
                                Dimension::Fixed(view.bounds.h)
                            },
                        },
                    };
                    node.properties.remove("text");
                }
            });
        }
        self.current()
            .controls
            .into_iter()
            .find(|c| c.id == id)
            .map(|c| {
                self.control_text_for_edit(id, &c.text)
                    .map(|text| (id, text))
            })
            .transpose()
    }
}
