// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use crate::{
    catalog::Context,
    layout::{self, Layout, LayoutNode},
};
use eframe::egui;
#[derive(Default)]
pub(super) struct Cache {
    project: Option<Project>,
    pages: HashMap<Id, Result<Layout, String>>,
    scenes: HashMap<Id, Result<crate::render::Scene, String>>,
}
#[derive(Clone)]
pub struct Layer {
    pub control: Control,
    /// This control's visible frame before its ancestors clip it.
    pub frame: Bounds,
    pub clip: Option<Bounds>,
    pub path: NodePath,
    pub properties: Properties,
}
impl Editor {
    pub fn layout_page(&self, id: Id) -> Result<Layout, String> {
        let mut cache = self.cache.borrow_mut();
        if cache.project.as_ref() != Some(&self.document) {
            if cache.project.as_ref().and_then(|p| p.font.as_ref()) != self.document.font.as_ref() {
                crate::fonts::install_project(&self.fonts, self.document.font.as_deref());
            }
            cache.project = Some(self.document.clone());
            cache.pages.clear();
            cache.scenes.clear();
        }
        if let Some(layout) = cache.pages.get(&id) {
            return layout.clone();
        }
        let (_, content) = self.surface(id).ok_or("Workspace no longer exists")?;
        let mut result = Err("Font layout did not run".into());
        let output = self.fonts.run_ui(egui::RawInput::default(), |ui| {
            let measurer = layout::EguiMeasurer {
                context: ui.ctx(),
                regular: crate::fonts::family(false, false),
                bold: crate::fonts::family(true, false),
            };
            result = layout::Engine {
                project: &self.document,
                measurer: &measurer,
                context: Context::default(),
            }
            .layout(content);
        });
        output.drop_without_applying_deltas();
        cache.pages.insert(id, result.clone());
        result
    }
    pub fn rendered_scene(&self, id: Id) -> Result<crate::render::Scene, String> {
        self.layout_page(id)?;
        if let Some(scene) = self.cache.borrow().scenes.get(&id) {
            return scene.clone();
        }
        let layers = self.layers(id)?;
        let (title, _) = self.surface(id).ok_or("Workspace no longer exists")?;
        let mut result = Err("Drawing did not run".into());
        let output = self.fonts.run_ui(egui::RawInput::default(), |ui| {
            let fonts = layout::EguiMeasurer {
                context: ui.ctx(),
                regular: crate::fonts::family(false, false),
                bold: crate::fonts::family(true, false),
            };
            result = crate::render::Scene::build(
                title,
                &layers,
                self.document.settings.sketch,
                &fonts,
                &self.document.assets,
            );
        });
        output.drop_without_applying_deltas();
        if let Ok(scene) = &mut result {
            scene.font = self.document.font.clone();
        }
        self.cache.borrow_mut().scenes.insert(id, result.clone());
        result
    }
    pub fn current(&self) -> model::Page {
        let page = self.page_data();
        let controls = self
            .layout_page(self.surface_id())
            .map(|layout| {
                let nodes = if self.context.is_empty() {
                    &layout.nodes
                } else {
                    find_layout(&layout.nodes, &self.context)
                        .map(|n| &n.children)
                        .unwrap_or(&layout.nodes)
                };
                nodes.iter().map(control).collect()
            })
            .unwrap_or_default();
        model::Page {
            id: self.surface_id(),
            name: self
                .surface(self.surface_id())
                .map_or_else(|| page.name.clone(), |(name, _)| name.to_owned()),
            notes: if self.definition.is_some() {
                String::new()
            } else {
                page.notes.clone()
            },
            controls,
        }
    }
    pub fn layers(&self, id: Id) -> Result<Vec<Layer>, String> {
        let layout = self.layout_page(id)?;
        Ok(layout
            .leaves()
            .into_iter()
            .map(|(node, _)| Layer {
                frame: bounds(node.bounds),
                control: {
                    let mut c = control(node);
                    c.bounds = bounds(node.uncropped);
                    c
                },
                clip: path_clip(&layout.nodes, &node.path).map(bounds),
                path: node.path.clone(),
                properties: node.properties.clone(),
            })
            .collect())
    }
    pub fn rendered_page(&self, id: Id) -> Result<model::Page, String> {
        let (name, _) = self.surface(id).ok_or("Workspace no longer exists")?;
        Ok(model::Page {
            id,
            name: name.into(),
            notes: self
                .document
                .pages
                .iter()
                .find(|p| p.id == id)
                .map(|p| p.notes.clone())
                .unwrap_or_default(),
            controls: self.layers(id)?.into_iter().map(|l| l.control).collect(),
        })
    }
    pub fn hit(&self, x: f32, y: f32) -> Option<Control> {
        let controls = self.current().controls;
        if let Ok(layers) = self.layers(self.surface_id()) {
            for layer in layers.into_iter().rev() {
                if layer.path.starts_with(&self.context)
                    && layer.control.bounds.contains(x, y)
                    && layer.clip.is_none_or(|b| b.contains(x, y))
                    && let Some(id) = layer.path.get(self.context.len())
                    && let Some(control) = controls.iter().find(|c| c.id == *id)
                {
                    return Some(control.clone());
                }
            }
        }
        controls
            .into_iter()
            .rev()
            .find(|c| c.group.is_some() && c.bounds.contains(x, y))
    }
    pub fn selection_label(&self, id: Id) -> String {
        match self.nodes().iter().find(|n| n.id == id).map(|n| &n.content) {
            Some(NodeContent::Group { .. }) => "Group".into(),
            Some(NodeContent::Instance { definition, .. }) => self
                .document
                .definitions
                .iter()
                .find(|d| d.id == *definition)
                .map(|d| format!("Component: {}", d.name))
                .unwrap_or("Missing component".into()),
            Some(NodeContent::Control { kind, .. }) => {
                catalog::get(kind).map_or_else(|| kind.clone(), |s| s.name.into())
            }
            None => "Control".into(),
        }
    }
}
pub(super) fn find_layout<'a>(nodes: &'a [LayoutNode], path: &[Id]) -> Option<&'a LayoutNode> {
    let node = nodes.iter().find(|n| n.path.last() == path.first())?;
    if path.len() == 1 {
        Some(node)
    } else {
        find_layout(&node.children, &path[1..])
    }
}
fn path_clip(nodes: &[LayoutNode], path: &[Id]) -> Option<layout::Rect> {
    let node = nodes.iter().find(|n| n.path.last() == path.first())?;
    let next = if path.len() > 1 {
        path_clip(&node.children, &path[1..])
    } else {
        None
    };
    match (node.clip, next) {
        (Some(a), Some(b)) => Some(a.intersect(b)),
        (Some(a), None) | (None, Some(a)) => Some(a),
        (None, None) => None,
    }
}
fn bounds(b: layout::Rect) -> Bounds {
    Bounds::new(b.x, b.y, b.width, b.height)
}
fn control(node: &LayoutNode) -> Control {
    let mut supported = true;
    let kind = match node.kind.as_deref().unwrap_or("") {
        "button" => Kind::Button,
        "label" => Kind::Label,
        "heading" => Kind::Heading,
        "paragraph" => Kind::Paragraph,
        "text_input" => Kind::TextInput,
        "text_area" => Kind::TextArea,
        "checkbox" => Kind::Checkbox,
        "radio" => Kind::Radio,
        "checkbox_group" => Kind::CheckboxGroup,
        "radio_group" => Kind::RadioGroup,
        "menu" => Kind::Menu,
        "menu_bar" => Kind::MenuBar,
        "dropdown" => Kind::Dropdown,
        "search" => Kind::Search,
        "ellipse" => Kind::Ellipse,
        "line" | "horizontal_rule" => Kind::Line,
        "arrow" => Kind::Arrow,
        "image" => Kind::Image,
        "icon" => Kind::Icon,
        "icon_label" => Kind::IconLabel,
        "circle_button" => Kind::CircleButton,
        "pointed_button" => Kind::PointedButton,
        "multiline_button" => Kind::MultilineButton,
        "button_bar" => Kind::ButtonBar,
        "breadcrumbs" => Kind::Breadcrumbs,
        "link_bar" => Kind::LinkBar,
        "browser" => Kind::Browser,
        "phone" => Kind::Phone,
        "panel" | "window" => Kind::Panel,
        "list" => Kind::List,
        "table" => Kind::Table,
        "tabs" => Kind::Tabs,
        "vertical_tabs" => Kind::VerticalTabs,
        "progress" => Kind::Progress,
        "toggle" => Kind::Toggle,
        "link" => Kind::Link,
        "rectangle" | "shape" | "" => Kind::Rectangle,
        _ => {
            supported = false;
            Kind::Rectangle
        }
    };
    let mut c = Control::new(
        *node.path.last().unwrap(),
        kind,
        node.uncropped.x,
        node.uncropped.y,
    );
    c.bounds = bounds(node.bounds);
    c.locked = node.locked;
    c.group = (node.kind.is_none()
        || node.properties.get(MISSING_COMPONENT) == Some(&Value::Bool(true)))
    .then_some(c.id);
    let p = &node.properties;
    c.text = match p.get("text") {
        Some(Value::Text(t)) => t.clone(),
        _ => String::new(),
    };
    if let Some(Value::Number(v)) = p.get("font_size") {
        c.font_size = *v as f32;
    }
    c.bold = matches!(p.get("bold"), Some(Value::Bool(true)));
    if let Some(Value::Color(v)) = p.get("fill") {
        c.fill = [v[0], v[1], v[2]];
    }
    c.checked = matches!(p.get("state"),Some(Value::Choice(v)) if v=="selected"||v=="disabled_selected"||v=="on");
    if !supported {
        c.text = format!(
            "Unsupported control: {}",
            node.kind.as_deref().unwrap_or("")
        );
    }
    c.link = match p.get("link") {
        Some(Value::Link(Link::Page(id))) => Some(*id),
        _ => None,
    };
    c
}
