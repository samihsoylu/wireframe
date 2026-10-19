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
    scenes: HashMap<(Id, bool), Result<crate::render::Scene, String>>,
    editing_scene: Option<EditingScene>,
    clocks: HashMap<(Id, NodePath), (ClockLayout, crate::tablet::Clock)>,
    dates: HashMap<(Id, NodePath), (ClockLayout, crate::date_controls::Date)>,
}
/// Owned data for an export worker, including previously displayed dynamic
/// values. Unseen controls use the time captured once for the whole export.
pub struct ExportSnapshot {
    pub(crate) document: Project,
    markup_visible: bool,
    clocks: HashMap<(Id, NodePath), (ClockLayout, crate::tablet::Clock)>,
    dates: HashMap<(Id, NodePath), (ClockLayout, crate::date_controls::Date)>,
    time: (crate::tablet::Clock, crate::date_controls::Date),
}
impl ExportSnapshot {
    pub fn new(document: Project) -> Self {
        Self {
            document,
            markup_visible: true,
            clocks: HashMap::new(),
            dates: HashMap::new(),
            time: (
                crate::tablet::Clock::now(),
                crate::date_controls::Date::today(),
            ),
        }
    }
    /// Explicit export override, independent of subsequent editor preference changes.
    pub fn with_markup(mut self, visible: bool) -> Self {
        self.markup_visible = visible;
        self
    }
    pub(crate) fn into_editor(self) -> Editor {
        let mut editor = Editor::new(self.document);
        editor.markup_visible = self.markup_visible;
        editor.frozen_time = Some(self.time);
        editor.cache.get_mut().clocks = self.clocks;
        editor.cache.get_mut().dates = self.dates;
        editor
    }
}
struct EditingScene {
    surface: Id,
    hide_markup: bool,
    context: NodePath,
    scene: Result<crate::render::Scene, String>,
}
#[derive(Clone, PartialEq)]
struct ClockLayout {
    options: ClockOptions,
    width: f32,
    height: f32,
    sketch: bool,
    font: Option<std::sync::Arc<crate::fonts::ProjectFont>>,
}
#[derive(Clone, PartialEq)]
enum ClockOptions {
    Calendar(f32),
    DatePicker(f32, bool),
    TimePicker,
    Tablet(crate::tablet::Options),
    Smartphone(crate::smartphone::Options),
}
impl ClockOptions {
    fn for_layer(layer: &Layer) -> Option<Self> {
        match layer.control.kind {
            Kind::Calendar => Some(Self::Calendar(layer.control.font_size)),
            Kind::DatePicker => Some(Self::DatePicker(
                layer.control.font_size,
                matches!(layer.properties.get("date_mode"),Some(Value::Choice(s)) if s=="year"),
            )),
            Kind::TimePicker => Some(Self::TimePicker),
            Kind::Tablet => Some(Self::Tablet(crate::tablet::Options::from_properties(
                &layer.properties,
            ))),
            Kind::Smartphone => Some(Self::Smartphone(
                crate::smartphone::Options::from_properties(&layer.properties),
            )),
            _ => None,
        }
    }
}
/// Persistent drawing scope; it never applies the editor's transient dimming.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SceneScope {
    Whole,
    Group(NodePath),
    Selection(Vec<NodePath>),
}
#[derive(Clone)]
pub struct Layer {
    pub control: Control,
    /// This control's visible frame before its ancestors clip it.
    pub frame: Bounds,
    pub clip: Option<Bounds>,
    /// Transient editing emphasis; stored/exported layers remain fully opaque.
    pub opacity: f32,
    /// Only the covering control's own markup exempts it from blocking links.
    pub local_markup: bool,
    /// Container targets applied above their final descendant in paint order.
    pub group_links: Vec<(Bounds, Link)>,
    pub path: NodePath,
    pub properties: Properties,
    pub clock: Option<crate::tablet::Clock>,
    pub date: Option<crate::date_controls::Date>,
}
impl Editor {
    pub fn export_snapshot(&self) -> ExportSnapshot {
        let cache = self.cache.borrow();
        ExportSnapshot {
            document: self.document.clone(),
            markup_visible: self.markup_visible,
            clocks: cache.clocks.clone(),
            dates: cache.dates.clone(),
            time: (self.render_clock(), self.render_date()),
        }
    }
    fn render_clock(&self) -> crate::tablet::Clock {
        self.frozen_time
            .map_or_else(|| (self.clock)(), |time| time.0)
    }
    fn render_date(&self) -> crate::date_controls::Date {
        self.frozen_time
            .map_or_else(|| (self.date)(), |time| time.1)
    }
    pub fn layout_page(&self, id: Id) -> Result<Layout, String> {
        self.layout_surface(id, false)
    }
    fn surface_for_view(&self, id: Id, retained: bool) -> Option<(&str, &Content)> {
        if retained {
            self.document
                .pages
                .iter()
                .find(|p| p.id == id && p.trashed)
                .map(|p| (p.name.as_str(), &p.content))
                .or_else(|| {
                    self.document
                        .libraries
                        .iter()
                        .find(|l| l.id == id && l.trashed)
                        .map(|l| (l.name.as_str(), &l.content))
                })
        } else {
            self.surface(id)
        }
    }
    fn layout_surface(&self, id: Id, retained: bool) -> Result<Layout, String> {
        // Check visibility before consulting a cache populated by Trash previews.
        let (title, content) = self
            .surface_for_view(id, retained)
            .ok_or("Workspace no longer exists")?;
        let mut cache = self.cache.borrow_mut();
        if cache.project.as_ref() != Some(&self.document) {
            if cache.project.as_ref().and_then(|p| p.font.as_ref()) != self.document.font.as_ref() {
                crate::fonts::install_project(&self.fonts, self.document.font.as_deref());
            }
            cache.project = Some(self.document.clone());
            cache
                .dates
                .retain(|(surface, _), _| self.surface(*surface).is_some());
            cache.pages.clear();
            cache.scenes.clear();
            cache.editing_scene = None;
            cache
                .clocks
                .retain(|(surface, _), _| self.surface(*surface).is_some());
        }
        if let Some(layout) = cache.pages.get(&id) {
            return layout.clone();
        }
        let mut result = Err("Font layout did not run".into());
        let output = self.fonts.run_ui(egui::RawInput::default(), |ui| {
            let measurer = layout::EguiMeasurer {
                context: ui.ctx(),
                regular: crate::fonts::family(false, false),
                bold: crate::fonts::family(true, false),
            };
            let engine = layout::Engine {
                project: &self.document,
                measurer: &measurer,
                context: Context::default(),
            };
            result = if self.document.libraries.iter().any(|l| l.id == id) {
                if retained {
                    engine.layout_library_preview(id)
                } else {
                    engine.layout_library(id)
                }
            } else {
                engine.layout_named(content, title)
            };
        });
        output.drop_without_applying_deltas();
        cache.pages.insert(id, result.clone());
        result
    }
    pub fn rendered_scene(&self, id: Id) -> Result<crate::render::Scene, String> {
        self.scene_view(id, None, false, None, false, false)
    }
    /// Presentation filters inherited markup before attaching group links, so
    /// a hidden final child cannot remove its visible siblings' container link.
    pub fn presentation_scene(&self, id: Id, markup: bool) -> Result<crate::render::Scene, String> {
        self.scene_view(id, None, false, None, false, !markup)
    }
    pub fn presentation_extents(&self) -> Result<Vec<crate::presentation::PageExtent>, String> {
        let mut result = vec![];
        for page in self.document.live_pages() {
            for id in std::iter::once(page.id).chain(page.alternates.iter().map(|a| a.id)) {
                let extent = |markup| -> Result<Bounds, String> {
                    let scene = self.presentation_scene(id, markup)?;
                    Ok(if scene.items.is_empty() {
                        Bounds::new(0., 0., 0., 0.)
                    } else {
                        scene.bounds
                    })
                };
                result.push(crate::presentation::PageExtent {
                    surface: id,
                    with_markup: extent(true)?,
                    without_markup: extent(false)?,
                });
            }
        }
        Ok(result)
    }
    /// Read-only official artwork for the recovery dialog. This does not make
    /// a retained resource navigable, editable, presentable or exportable.
    pub fn retained_preview(&self, id: Id) -> Result<crate::render::Scene, String> {
        self.scene_view(id, None, false, None, true, false)
    }
    /// Reveal only the active group and its ancestor masks, without changing
    /// document geometry or the canonical export/presentation scene.
    pub fn editing_scene(&self) -> Result<crate::render::Scene, String> {
        self.scene_view(
            self.surface_id(),
            None,
            !self.context.is_empty(),
            None,
            false,
            !self.markup_visible,
        )
    }
    pub(super) fn rendered_scene_excluding(
        &self,
        id: Id,
        excluded: Option<&[Id]>,
    ) -> Result<crate::render::Scene, String> {
        self.scene_view(
            id,
            excluded,
            id == self.surface_id() && !self.context.is_empty(),
            None,
            false,
            !self.markup_visible,
        )
    }
    /// Apply the editor's markup preference to the complete layout. Filtering
    /// never moves group children, reveals crops, or reorders descendants.
    pub fn export_scene(&self, id: Id, scope: &SceneScope) -> Result<crate::render::Scene, String> {
        let layout = self.layout_page(id)?;
        match scope {
            SceneScope::Whole => {}
            SceneScope::Group(path) => {
                let node = find_layout(&layout.nodes, path)
                    .ok_or("The export group is no longer available")?;
                if node.kind.is_some() {
                    return Err("Choose a group to export".into());
                }
            }
            SceneScope::Selection(paths) => {
                if paths.is_empty()
                    || paths
                        .iter()
                        .any(|path| find_layout(&layout.nodes, path).is_none())
                {
                    return Err("The export selection is empty or no longer available".into());
                }
            }
        }
        self.scene_view(id, None, false, Some(scope), false, !self.markup_visible)
    }
    fn scene_view(
        &self,
        id: Id,
        excluded: Option<&[Id]>,
        editing: bool,
        scope: Option<&SceneScope>,
        retained: bool,
        hide_markup: bool,
    ) -> Result<crate::render::Scene, String> {
        self.layout_surface(id, retained)?;
        let cacheable = excluded.is_none() && scope.is_none();
        if editing
            && cacheable
            && let Some(cached) = &self.cache.borrow().editing_scene
            && cached.surface == id
            && cached.context == self.context
            && cached.hide_markup == hide_markup
        {
            return cached.scene.clone();
        }
        if !editing
            && cacheable
            && let Some(scene) = self.cache.borrow().scenes.get(&(id, hide_markup))
        {
            return scene.clone();
        }
        let mut layers = self.layers_for_view(
            id,
            if editing { &self.context } else { &[] },
            retained,
            hide_markup,
        )?;
        if let Some(path) = excluded {
            layers.retain(|l| !l.path.starts_with(path));
        }
        if let Some(scope) = scope {
            layers.retain(|layer| match scope {
                SceneScope::Whole => true,
                SceneScope::Group(path) => layer.path.starts_with(path) && layer.path != *path,
                SceneScope::Selection(paths) => {
                    paths.iter().any(|path| layer.path.starts_with(path))
                }
            });
        }
        {
            let mut cache = self.cache.borrow_mut();
            let paths = layers
                .iter()
                .filter(|l| ClockOptions::for_layer(l).is_some())
                .map(|l| l.path.clone())
                .collect::<HashSet<_>>();
            if cacheable && !hide_markup {
                cache
                    .clocks
                    .retain(|(surface, path), _| *surface != id || paths.contains(path));
                cache
                    .dates
                    .retain(|(surface, path), _| *surface != id || paths.contains(path));
            }
            for layer in &mut layers {
                let Some(options) = ClockOptions::for_layer(layer) else {
                    continue;
                };
                let signature = ClockLayout {
                    options,
                    width: layer.control.bounds.w,
                    height: layer.control.bounds.h,
                    sketch: self.document.settings.sketch,
                    font: self.document.font.clone(),
                };
                let key = (id, layer.path.clone());
                if matches!(layer.control.kind, Kind::Calendar | Kind::DatePicker) {
                    let record = cache
                        .dates
                        .entry(key)
                        .or_insert_with(|| (signature.clone(), self.render_date()));
                    if record.0 != signature {
                        *record = (signature, self.render_date());
                    }
                    layer.date = Some(record.1);
                    continue;
                }
                let record = cache
                    .clocks
                    .entry(key)
                    .or_insert_with(|| (signature.clone(), self.render_clock()));
                // Location is deliberately absent: moving a control does not
                // relayout its contents or change its displayed clock.
                if record.0 != signature {
                    *record = (signature, self.render_clock());
                }
                layer.clock = Some(record.1);
            }
        }
        let (title, _) = self
            .surface_for_view(id, retained)
            .ok_or("Workspace no longer exists")?;
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
            if (scope.is_some() || hide_markup)
                && !layers.iter().any(|layer| layer.control.kind == Kind::Arrow)
            {
                scene.bounds = layers
                    .iter()
                    .filter_map(|layer| {
                        let b = layer.clip.map_or(layer.control.bounds, |clip| {
                            crate::render::intersect(layer.control.bounds, clip)
                        });
                        (b.w > 0. && b.h > 0.).then_some(b)
                    })
                    .reduce(Bounds::union)
                    .unwrap_or(Bounds::new(0., 0., 0., 0.));
            }
        }
        if cacheable {
            if editing {
                self.cache.borrow_mut().editing_scene = Some(EditingScene {
                    surface: id,
                    hide_markup,
                    context: self.context.clone(),
                    scene: result.clone(),
                });
            } else {
                self.cache
                    .borrow_mut()
                    .scenes
                    .insert((id, hide_markup), result.clone());
            }
        }
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
                let owner_locked = self.source_owner_locked();
                nodes
                    .iter()
                    .filter(|node| self.markup_visible || !node.markup)
                    .map(|node| {
                        let mut c = control(node);
                        c.locked |= owner_locked;
                        c
                    })
                    .collect()
            })
            .unwrap_or_default();
        model::Page {
            id: self.surface_id(),
            name: self
                .surface(self.surface_id())
                .map_or_else(|| page.name.clone(), |(name, _)| name.to_owned()),
            notes: self.document.surface_notes(self.surface_id()).to_owned(),
            controls,
        }
    }
    pub fn layers(&self, id: Id) -> Result<Vec<Layer>, String> {
        self.layers_in_context(id, &[])
    }
    pub fn editing_layers(&self) -> Result<Vec<Layer>, String> {
        self.layers_for_view(
            self.surface_id(),
            &self.context,
            false,
            !self.markup_visible,
        )
    }
    /// Visible editor artwork for framing another page without changing context.
    pub fn visible_editor_layers(&self, id: Id) -> Result<Vec<Layer>, String> {
        self.layers_for_view(id, &[], false, !self.markup_visible)
    }
    fn layers_in_context(&self, id: Id, context: &[Id]) -> Result<Vec<Layer>, String> {
        self.layers_for_view(id, context, false, false)
    }
    fn layers_for_view(
        &self,
        id: Id,
        context: &[Id],
        retained: bool,
        hide_markup: bool,
    ) -> Result<Vec<Layer>, String> {
        let layout = self.layout_surface(id, retained)?;
        let mut layers: Vec<Layer> = layout
            .leaves()
            .into_iter()
            .filter(|(node, _)| !hide_markup || !node.markup)
            .map(|(node, _)| Layer {
                frame: bounds(node.bounds),
                control: {
                    let mut c = control(node);
                    c.bounds = bounds(node.uncropped);
                    if matches!(c.kind, Kind::Calendar | Kind::DatePicker) {
                        c.font_size = self.document.settings.font_size;
                    }
                    c
                },
                clip: path_clip(&layout.nodes, &node.path, context).map(bounds),
                opacity: if node.path == context && node.kind.is_none() && node.children.is_empty()
                {
                    // The placeholder represents a container from outside it;
                    // its own empty editing workspace stays blank.
                    0.
                } else if context.is_empty()
                    || node.path.starts_with(context)
                    || context.starts_with(&node.path)
                {
                    1.
                } else {
                    let parent_depth = node.path.len().saturating_sub(1);
                    let distance = context
                        .len()
                        .checked_sub(parent_depth)
                        .filter(|distance| *distance > 0)
                        .unwrap_or(parent_depth);
                    (0.4 / distance as f32).max(0.1)
                },
                group_links: vec![],
                local_markup: node.local_markup,
                path: node.path.clone(),
                properties: node.properties.clone(),
                clock: None,
                date: None,
            })
            .collect();
        let mut last = HashMap::new();
        for (index, layer) in layers.iter().enumerate() {
            for depth in 1..layer.path.len() {
                last.insert(layer.path[..depth].to_vec(), index);
            }
        }
        fn attach(
            nodes: &[LayoutNode],
            layout: &Layout,
            last: &HashMap<NodePath, usize>,
            layers: &mut [Layer],
            context: &[Id],
        ) {
            for node in nodes {
                attach(&node.children, layout, last, layers, context);
                if let Some(Value::Link(link)) = node.properties.get("link")
                    && let Some(&index) = last.get(&node.path)
                {
                    let frame = path_clip(&layout.nodes, &node.path, context)
                        .map_or(node.bounds, |clip| node.bounds.intersect(clip));
                    if frame.width > 0. && frame.height > 0. {
                        layers[index]
                            .group_links
                            .push((bounds(frame), link.clone()));
                    }
                }
            }
        }
        attach(&layout.nodes, &layout, &last, &mut layers, context);
        Ok(layers)
    }
    pub fn rendered_page(&self, id: Id) -> Result<model::Page, String> {
        let (name, _) = self.surface(id).ok_or("Workspace no longer exists")?;
        Ok(model::Page {
            id,
            name: name.into(),
            notes: self.document.surface_notes(id).to_owned(),
            controls: self.layers(id)?.into_iter().map(|l| l.control).collect(),
        })
    }
    pub fn hit(&self, x: f32, y: f32) -> Option<Control> {
        let controls = self.current().controls;
        if let Ok(layers) = self.editing_layers() {
            for layer in layers.into_iter().rev() {
                if layer.path.starts_with(&self.context)
                    && if layer.control.kind == Kind::Arrow {
                        let b = layer.control.bounds;
                        let arrow =
                            crate::arrows::Arrow::from_properties(&layer.properties, b.w, b.h);
                        let offset = arrow.display_offset(&layer.properties);
                        arrow.distance_to(Point::new(x - b.x + offset.x, y - b.y + offset.y)) < 15.
                    } else {
                        layer.control.bounds.contains(x, y)
                    }
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
fn path_clip(nodes: &[LayoutNode], path: &[Id], revealed: &[Id]) -> Option<layout::Rect> {
    let node = nodes.iter().find(|n| n.path.last() == path.first())?;
    let next = if path.len() > 1 {
        path_clip(&node.children, &path[1..], revealed)
    } else {
        None
    };
    let own = if revealed.starts_with(&node.path) {
        None
    } else {
        node.clip
    };
    match (own, next) {
        (Some(a), Some(b)) => Some(a.intersect(b)),
        (Some(a), None) | (None, Some(a)) => Some(a),
        (None, None) => None,
    }
}
fn bounds(b: layout::Rect) -> Bounds {
    Bounds::new(b.x, b.y, b.width, b.height)
}
pub(super) fn control(node: &LayoutNode) -> Control {
    let mut supported = true;
    let kind = match node.kind.as_deref().unwrap_or("") {
        "button" => Kind::Button,
        "label" => Kind::Label,
        "heading" => Kind::Heading,
        "subtitle" => Kind::Subtitle,
        "formatting_toolbar" => Kind::FormattingToolbar,
        "popover" => Kind::Popover,
        "site_map" => Kind::SiteMap,
        "map" => Kind::StreetMap,
        "modal_overlay" => Kind::ModalOverlay,
        "vertical_rule" => Kind::VerticalRule,
        "numeric_stepper" => Kind::NumericStepper,
        "color_picker" => Kind::ColorPicker,
        "help_button" => Kind::HelpButton,

        "text_block" => Kind::TextBlock,
        "text_line" => Kind::TextLine,
        "tag_cloud" => Kind::TagCloud,

        "paragraph" => Kind::Paragraph,
        "text_input" => Kind::TextInput,
        "text_area" => Kind::TextArea,
        "checkbox" => Kind::Checkbox,
        "radio" => Kind::Radio,
        "checkbox_group" => Kind::CheckboxGroup,
        "radio_group" => Kind::RadioGroup,
        "menu" => Kind::Menu,
        "menu_bar" => Kind::MenuBar,
        "tree" => Kind::Tree,
        "accordion" => Kind::Accordion,
        "alert" => Kind::Alert,
        "app_bar" => Kind::AppBar,
        "dropdown" => Kind::Dropdown,
        "search" => Kind::Search,
        "ellipse" => Kind::Ellipse,
        "shape" => Kind::Shape,
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
        "tablet" => Kind::Tablet,
        "smartphone" => Kind::Smartphone,
        "mobile_keyboard" => Kind::MobileKeyboard,
        "mobile_menu" => Kind::MobileMenu,
        "mobile_picker" => Kind::MobilePicker,
        "calendar" => Kind::Calendar,
        "date_chooser" => Kind::DateChooser,
        "date_picker" => Kind::DatePicker,
        "time_picker" => Kind::TimePicker,
        "horizontal_slider" => Kind::HorizontalSlider,
        "vertical_slider" => Kind::VerticalSlider,
        "horizontal_scrollbar" => Kind::HorizontalScrollbar,
        "vertical_scrollbar" => Kind::VerticalScrollbar,
        "horizontal_splitter" => Kind::HorizontalSplitter,
        "vertical_splitter" => Kind::VerticalSplitter,
        "volume_slider" => Kind::VolumeSlider,
        "playback" => Kind::Playback,
        "webcam" => Kind::Webcam,
        "video" => Kind::Video,
        "cover_flow" => Kind::CoverFlow,
        "column_chart" => Kind::ColumnChart,
        "bar_chart" => Kind::BarChart,
        "line_chart" => Kind::LineChart,
        "pie_chart" => Kind::PieChart,
        "horizontal_brace" => Kind::HorizontalBrace,
        "vertical_brace" => Kind::VerticalBrace,
        "red_cross" => Kind::RedCross,
        "scratch_out" => Kind::ScratchOut,

        "callout" => Kind::Callout,
        "sticky_note" => Kind::StickyNote,
        "tooltip" => Kind::Tooltip,

        "panel" | "window" => Kind::Panel,
        "field_set" => Kind::FieldSet,
        "list" => Kind::List,
        "table" => Kind::Table,
        "tabs" => Kind::Tabs,
        "vertical_tabs" => Kind::VerticalTabs,
        "progress" => Kind::Progress,
        "toggle" => Kind::Toggle,
        "link" => Kind::Link,
        "rectangle" | "" => Kind::Rectangle,
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
