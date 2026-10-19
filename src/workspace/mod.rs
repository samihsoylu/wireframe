// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Recursive editing owns project data; renderer projections never become saved documents.
mod alignment;
mod app_bar_icons;
mod arrangement;
mod markup_visibility;
mod move_preview;
mod row_icons;
pub use app_bar_icons::AppBarIconSlot;
mod row_selection;
pub use row_icons::ListRowIcon;
mod property_sections;
mod selection_properties;
pub use arrangement::AlignmentAction;
pub use property_sections::PropertySection;
pub use row_selection::{RowSelection, selectable_row_labels};
pub use selection_properties::SelectionProperty;
mod layering;
pub use layering::LayerAction;
mod arrows;
mod clipboard;
mod library_transfer;
mod project_asset_import;
mod project_copy;
pub use project_asset_import::{AssetImportAction, AssetImportChoice, AssetImportEntry};
mod project_library_import;
mod project_page_import;
pub use library_transfer::{
    LibraryClipboard, LibraryImportAction, LibraryImportChoice, LibraryImportCounts,
    LibraryImportEntry, LibraryImportPlan, PreparedLibraryImport,
};
pub use project_copy::{PreparedProjectCopy, ProjectCopyCounts};
pub use project_library_import::ProjectLibraryImportPlan;
pub use project_page_import::{PageImportChoice, PageImportEntry, PageImportPlan};
mod images;
mod links;
mod locking;
pub use images::{ImageImportTarget, ImageTarget, ImportChoice};
pub use links::{LinkAction, LinkTarget};
mod asset_catalog;
mod asset_retention;
pub use asset_catalog::AssetPreviews;
mod catalog_items;
mod catalog_presets;
mod icon_catalog;
pub use catalog_presets::CatalogPreset;
mod components;
pub use catalog_items::{CatalogCategory, CatalogEntry, CatalogItem};
mod component_previews;
pub use component_previews::ComponentPreviews;
mod libraries;
mod library_lifecycle;
mod source_actions;
mod source_move;
pub use libraries::ComponentUsage;
mod crop;
mod overrides;
mod page_clipboard;
mod page_order;
mod pages;
pub use page_clipboard::PageClipboard;
mod permanent;
mod properties;
mod quick_draw;
mod resize;
pub use resize::SelectionGeometry;
mod reviews;
mod text_controls;
mod text_link_transfer;
mod text_links;
pub use text_links::TextLink;
mod trash;
pub use trash::RetainedResource;
mod versions;
mod view;
use crate::{
    catalog,
    document::*,
    model::{self, Bounds, Control, Kind},
};
pub use clipboard::{Clipboard, SourceOrigin};
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
};
pub use view::{ExportSnapshot, Layer, SceneScope};
#[derive(Clone)]
struct Snapshot {
    alternate: Option<Id>,
    document: Project,
    page: Id,
    definition: Option<Id>,
    library: Option<Id>,
    context: NodePath,
    selection: HashSet<Id>,
    selection_order: Vec<Id>,
}
#[derive(Clone)]
struct Change {
    invalidated: bool,
    name: String,
    before: Snapshot,
    after: Snapshot,
    bytes: usize,
    page_history: Option<trash::PageHistory>,
}
pub struct Editor {
    clipboard_session: String,
    markup_visible: bool,
    active_versions: HashMap<Id, Id>,
    pub document: Project,
    pub page: Id,
    pub definition: Option<Id>,
    pub library: Option<Id>,
    pub context: NodePath,
    pub selection: HashSet<Id>,
    selection_order: Vec<Id>,
    saved: Project,
    unsaved: bool,
    history: Vec<Change>,
    cursor: usize,
    pending: Option<(String, Snapshot)>,
    pub error: Option<String>,
    cache: RefCell<view::Cache>,
    fonts: eframe::egui::Context,
    clock: fn() -> crate::tablet::Clock,
    date: fn() -> crate::date_controls::Date,
    frozen_time: Option<(crate::tablet::Clock, crate::date_controls::Date)>,
}
impl Default for Editor {
    fn default() -> Self {
        Self::new(Project::default())
    }
}
impl Editor {
    pub fn new(document: Project) -> Self {
        Self::with_clock(document, crate::tablet::Clock::now)
    }
    /// Supply a clock for reproducible rendering without storing time in projects.
    pub fn with_clock(document: Project, clock: fn() -> crate::tablet::Clock) -> Self {
        let fonts = eframe::egui::Context::default();
        crate::fonts::install_project(&fonts, document.font.as_deref());
        Self {
            markup_visible: true,
            clipboard_session: {
                static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
                format!(
                    "{}-{}-{}",
                    std::process::id(),
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_nanos(),
                    NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
                )
            },
            page: document.first_live_page().id,
            active_versions: HashMap::new(),
            definition: None,
            library: None,
            saved: document.clone(),
            unsaved: false,
            document,
            context: vec![],
            selection: HashSet::new(),
            selection_order: Vec::new(),
            history: vec![],
            cursor: 0,
            pending: None,
            error: None,
            cache: RefCell::new(view::Cache::default()),
            fonts,
            clock,
            date: crate::date_controls::Date::today,
            frozen_time: None,
        }
    }
    /// Stable for this editor session, independent of document history.
    pub fn clipboard_session(&self) -> &str {
        &self.clipboard_session
    }
    /// Inject the local date for repeatable calendar scenes; it is never serialized.
    pub fn with_date(document: Project, date: fn() -> crate::date_controls::Date) -> Self {
        let mut editor = Self::new(document);
        editor.date = date;
        editor
    }
    fn snapshot(&self) -> Snapshot {
        Snapshot {
            alternate: self.active_alternate(self.page),
            document: self.document.clone(),
            page: self.page,
            definition: self.definition,
            library: self.library,
            context: self.context.clone(),
            selection: self.selection.clone(),
            selection_order: self.ordered_selection(),
        }
    }
    fn restore(&mut self, s: Snapshot) {
        self.document = s.document;
        self.page = s.page;
        if let Some(id) = s.alternate {
            self.active_versions.insert(s.page, id);
        } else {
            self.active_versions.remove(&s.page);
        }
        self.definition = s.definition;
        self.library = s.library;
        self.context = s.context;
        self.selection = s.selection;
        self.selection_order = s.selection_order;
        self.reconcile();
    }
    pub fn page_data(&self) -> &Page {
        self.document
            .live_page(self.page)
            .unwrap_or_else(|| self.document.first_live_page())
    }
    pub fn current_mut(&mut self) -> &mut Page {
        let id = self.page_data().id;
        self.document
            .pages
            .iter_mut()
            .find(|page| page.id == id)
            .unwrap()
    }
    pub fn nodes(&self) -> std::borrow::Cow<'_, [Node]> {
        self.context_nodes().unwrap_or_default()
    }
    fn nodes_mut(&mut self) -> &mut Vec<Node> {
        let context = self.context.clone();
        children_mut(&mut self.active_content_mut().nodes, &context)
            .expect("reconciled edit context")
    }
    fn reconcile(&mut self) {
        if self
            .library
            .is_some_and(|id| !self.document.live_libraries().any(|l| l.id == id))
        {
            self.library = None;
            self.context.clear();
            self.selection.clear();
        }
        self.active_versions.retain(|page, alternate| {
            self.document.pages.iter().any(|p| {
                !p.trashed && p.id == *page && p.alternates.iter().any(|a| a.id == *alternate)
            })
        });
        if self
            .definition
            .is_some_and(|id| !self.document.definition_available(id))
        {
            self.definition = None;
            self.context.clear();
            self.selection.clear();
        }
        if self.document.live_page(self.page).is_none() {
            self.page = self.document.first_live_page().id;
            self.context.clear();
        }
        while self.context_nodes().is_none() {
            self.context.pop();
        }
        let ids: HashSet<_> = if self.markup_visible {
            self.nodes().iter().map(|n| n.id).collect()
        } else {
            self.current().controls.iter().map(|c| c.id).collect()
        };
        self.selection.retain(|id| ids.contains(id));
        self.selection_order
            .retain(|id| self.selection.contains(id));
    }
    pub fn dirty(&self) -> bool {
        self.unsaved || self.document != self.saved
    }
    pub fn mark_saved(&mut self) {
        self.saved = self.document.clone();
        self.unsaved = false;
    }
    /// A background write acknowledges its captured document, even if current
    /// edits have advanced. History and selection remain unchanged.
    pub fn mark_saved_snapshot(&mut self, snapshot: &Project) {
        self.saved = snapshot.clone();
        self.unsaved = false;
    }

    /// Restored unnamed documents need a chosen destination even when empty.
    pub fn mark_unsaved(&mut self) {
        self.unsaved = true;
    }

    pub fn undo_name(&self) -> Option<&str> {
        self.cursor
            .checked_sub(1)
            .map(|i| self.history[i].name.as_str())
    }
    pub fn redo_name(&self) -> Option<&str> {
        self.history.get(self.cursor).map(|c| c.name.as_str())
    }
    pub fn begin(&mut self, name: &str) {
        if self.pending.is_none() {
            self.pending = Some((name.into(), self.snapshot()));
        }
    }
    pub fn commit(&mut self) {
        if let Some((name, before)) = self.pending.take() {
            self.record(name, before);
        }
    }
    /// Recalculate an absolute gesture preview from its initial geometry.
    /// Keep the transaction open so release records only one history entry.
    pub fn reset_preview(&mut self) {
        if let Some((_, before)) = &self.pending {
            self.restore(before.clone());
        }
    }
    pub fn cancel(&mut self) {
        if let Some((_, before)) = self.pending.take() {
            self.restore(before);
        }
    }
    fn record(&mut self, name: String, before: Snapshot) {
        self.record_with_page_history(name, before, None);
    }
    fn record_with_page_history(
        &mut self,
        name: String,
        before: Snapshot,
        page_history: Option<trash::PageHistory>,
    ) {
        if before.document == self.document {
            return;
        }
        if let Err(error) = self
            .repair_empty_order(&before)
            .and_then(|()| self.normalize_source_order())
            .and_then(|()| self.document.validate())
        {
            self.restore(before);
            self.error = Some(error);
            return;
        }
        if !self.markup_visible
            && markup_visibility::has_added_markup(
                &before.document,
                &self.document,
                self.surface_id(),
            )
        {
            self.markup_visible = true;
        }
        self.retain_visible_selection();
        self.history.truncate(self.cursor);
        let bytes = serde_json::to_vec(&before.document).map_or(0, |v| v.len())
            + serde_json::to_vec(&self.document).map_or(0, |v| v.len());
        self.history.push(Change {
            invalidated: false,
            name,
            before,
            after: self.snapshot(),
            bytes,
            page_history,
        });
        let mut bytes: usize = self.history.iter().map(|c| c.bytes).sum();
        while self.history.len() > 1 && (self.history.len() > 100 || bytes > 64 * 1024 * 1024) {
            bytes = bytes.saturating_sub(self.history.remove(0).bytes);
        }
        self.cursor = self.history.len();
    }
    pub fn edit(&mut self, name: &str, f: impl FnOnce(&mut Self)) {
        self.commit();
        let before = self.snapshot();
        f(self);
        self.record(name.into(), before);
        self.reconcile();
    }
    pub fn undo(&mut self) {
        self.commit();
        if self.cursor > 0 {
            let index = self.cursor - 1;
            if self.history[index].invalidated {
                let change = self.history.remove(index);
                self.cursor = index;
                self.error = Some(format!(
                    "Cannot undo {}: the resource was permanently deleted",
                    change.name
                ));
                return;
            }
            let target = &self.history[index].before;
            let surface = target
                .definition
                .or(target.library)
                .or(target.alternate)
                .unwrap_or(target.page);
            let reveal = !self.markup_visible
                && markup_visibility::has_added_markup(&self.document, &target.document, surface);
            if reveal {
                self.markup_visible = true;
            }
            if self.replay_page_history(index, true) {
                self.cursor = index;
            } else if reveal {
                self.set_markup_visible(false);
            }
        }
    }
    pub fn redo(&mut self) {
        self.commit();
        if self.cursor < self.history.len() && self.history[self.cursor].invalidated {
            let change = self.history.remove(self.cursor);
            self.error = Some(format!(
                "Cannot redo {}: the resource was permanently deleted",
                change.name
            ));
            return;
        }
        if self.cursor < self.history.len() {
            let target = &self.history[self.cursor].after;
            let surface = target
                .definition
                .or(target.library)
                .or(target.alternate)
                .unwrap_or(target.page);
            let reveal = !self.markup_visible
                && markup_visibility::has_added_markup(&self.document, &target.document, surface);
            if reveal {
                self.markup_visible = true;
            }
            if self.replay_page_history(self.cursor, false) {
                self.cursor += 1;
            } else if reveal {
                self.set_markup_visible(false);
            }
        }
    }
    pub fn switch_page(&mut self, id: Id) {
        self.commit();
        if self.document.live_page(id).is_some() {
            self.page = id;
            self.definition = None;
            self.library = None;
            self.context.clear();
            self.selection.clear();
        }
    }
    pub fn enter(&mut self, id: Id) -> bool {
        self.commit();
        if let Some(definition) = self.owning_source(id) {
            return self.edit_definition(definition).is_ok();
        }
        if self.nodes().iter().any(|n| {
            n.id == id
                && !self.missing_component(id)
                && matches!(
                    n.content,
                    NodeContent::Group { .. } | NodeContent::Instance { .. }
                )
        }) {
            self.context.push(id);
            self.selection.clear();
            true
        } else {
            false
        }
    }
    pub fn leave(&mut self) -> bool {
        self.commit();
        if let Some(id) = self.context.pop() {
            self.selection = HashSet::from([id]);
            self.retain_visible_selection();
            true
        } else if self.definition.take().is_some() || self.library.take().is_some() {
            self.selection.clear();
            true
        } else {
            false
        }
    }
    pub fn is_group(&self, id: Id) -> bool {
        self.nodes()
            .iter()
            .any(|n| n.id == id && matches!(n.content, NodeContent::Group { .. }))
    }
    pub fn add(&mut self, kind: Kind, x: f32, y: f32) {
        self.add_control(kind, None, x, y);
    }
    pub fn add_preset(&mut self, preset: CatalogPreset, x: f32, y: f32) {
        self.add_control(preset.kind(), Some(preset), x, y);
    }
    fn add_control(&mut self, kind: Kind, preset: Option<CatalogPreset>, x: f32, y: f32) {
        if !self.structure_edit_allowed() {
            return;
        }
        self.edit("Add control", |e| {
            let Ok(id) = e.document.allocate_id() else {
                e.error = Some("Project ID space exhausted".into());
                return;
            };
            if matches!(
                kind,
                Kind::Label
                    | Kind::Link
                    | Kind::Paragraph
                    | Kind::Table
                    | Kind::Progress
                    | Kind::Line
                    | Kind::Arrow
                    | Kind::Toggle
                    | Kind::Shape
                    | Kind::Heading
                    | Kind::Popover
                    | Kind::SiteMap
                    | Kind::FormattingToolbar
                    | Kind::StreetMap
                    | Kind::ModalOverlay
                    | Kind::VerticalRule
                    | Kind::NumericStepper
                    | Kind::ColorPicker
                    | Kind::HelpButton
                    | Kind::Subtitle
                    | Kind::TextBlock
                    | Kind::TextLine
                    | Kind::TagCloud
                    | Kind::Checkbox
                    | Kind::Radio
                    | Kind::CheckboxGroup
                    | Kind::RadioGroup
                    | Kind::Menu
                    | Kind::MenuBar
                    | Kind::Tree
                    | Kind::Accordion
                    | Kind::Alert
                    | Kind::AppBar
                    | Kind::Browser
                    | Kind::Panel
                    | Kind::Phone
                    | Kind::Tablet
                    | Kind::Smartphone
                    | Kind::MobileKeyboard
                    | Kind::MobileMenu
                    | Kind::MobilePicker
                    | Kind::Calendar
                    | Kind::DatePicker
                    | Kind::TimePicker
                    | Kind::HorizontalSlider
                    | Kind::VerticalSlider
                    | Kind::HorizontalScrollbar
                    | Kind::VerticalScrollbar
                    | Kind::HorizontalSplitter
                    | Kind::VerticalSplitter
                    | Kind::VolumeSlider
                    | Kind::Playback
                    | Kind::Webcam
                    | Kind::Video
                    | Kind::CoverFlow
                    | Kind::ColumnChart
                    | Kind::BarChart
                    | Kind::LineChart
                    | Kind::PieChart
                    | Kind::HorizontalBrace
                    | Kind::VerticalBrace
                    | Kind::RedCross
                    | Kind::ScratchOut
                    | Kind::Callout
                    | Kind::StickyNote
                    | Kind::Tooltip
                    | Kind::DateChooser
                    | Kind::FieldSet
                    | Kind::Button
                    | Kind::TextInput
                    | Kind::TextArea
                    | Kind::Search
                    | Kind::Dropdown
                    | Kind::Image
                    | Kind::Icon
                    | Kind::IconLabel
                    | Kind::CircleButton
                    | Kind::PointedButton
                    | Kind::MultilineButton
                    | Kind::ButtonBar
                    | Kind::Breadcrumbs
                    | Kind::LinkBar
                    | Kind::Tabs
                    | Kind::VerticalTabs
                    | Kind::List
            ) {
                let kind_id = match kind {
                    Kind::Label => "label",
                    Kind::Link => "link",
                    Kind::Paragraph => "paragraph",
                    Kind::Table => "table",
                    Kind::Progress => "progress",
                    Kind::Line => "horizontal_rule",
                    Kind::Arrow => "arrow",
                    Kind::Toggle => "toggle",
                    Kind::Shape => "shape",
                    Kind::Checkbox => "checkbox",
                    Kind::Radio => "radio",
                    Kind::CheckboxGroup => "checkbox_group",
                    Kind::RadioGroup => "radio_group",
                    Kind::Menu => "menu",
                    Kind::MenuBar => "menu_bar",
                    Kind::Tree => "tree",
                    Kind::Accordion => "accordion",
                    Kind::Alert => "alert",
                    Kind::AppBar => "app_bar",
                    Kind::Browser => "browser",
                    Kind::Panel => "window",
                    Kind::FieldSet => "field_set",
                    Kind::Phone => "phone",
                    Kind::Tablet => "tablet",
                    Kind::Smartphone => "smartphone",
                    Kind::MobileKeyboard => "mobile_keyboard",
                    Kind::MobileMenu => "mobile_menu",
                    Kind::MobilePicker => "mobile_picker",
                    Kind::Calendar => "calendar",
                    Kind::DateChooser => "date_chooser",
                    Kind::DatePicker => "date_picker",
                    Kind::TimePicker => "time_picker",
                    Kind::HorizontalSlider => "horizontal_slider",
                    Kind::VerticalSlider => "vertical_slider",
                    Kind::HorizontalScrollbar => "horizontal_scrollbar",
                    Kind::VerticalScrollbar => "vertical_scrollbar",
                    Kind::HorizontalSplitter => "horizontal_splitter",
                    Kind::VerticalSplitter => "vertical_splitter",
                    Kind::VolumeSlider => "volume_slider",
                    Kind::Playback => "playback",
                    Kind::Webcam => "webcam",
                    Kind::Video => "video",
                    Kind::CoverFlow => "cover_flow",
                    Kind::ColumnChart => "column_chart",
                    Kind::BarChart => "bar_chart",
                    Kind::LineChart => "line_chart",
                    Kind::PieChart => "pie_chart",
                    Kind::Heading => "heading",
                    Kind::Subtitle => "subtitle",
                    Kind::Popover => "popover",
                    Kind::SiteMap => "site_map",
                    Kind::FormattingToolbar => "formatting_toolbar",
                    Kind::StreetMap => "map",
                    Kind::ModalOverlay => "modal_overlay",
                    Kind::VerticalRule => "vertical_rule",
                    Kind::NumericStepper => "numeric_stepper",
                    Kind::ColorPicker => "color_picker",
                    Kind::HelpButton => "help_button",

                    Kind::TextBlock => "text_block",
                    Kind::TextLine => "text_line",
                    Kind::TagCloud => "tag_cloud",
                    Kind::HorizontalBrace => "horizontal_brace",
                    Kind::VerticalBrace => "vertical_brace",
                    Kind::RedCross => "red_cross",
                    Kind::ScratchOut => "scratch_out",

                    Kind::Callout => "callout",
                    Kind::StickyNote => "sticky_note",
                    Kind::Tooltip => "tooltip",
                    Kind::Button => "button",
                    Kind::TextInput => "text_input",
                    Kind::TextArea => "text_area",
                    Kind::Search => "search",
                    Kind::Dropdown => "dropdown",
                    Kind::Image => "image",
                    Kind::Icon => "icon",
                    Kind::IconLabel => "icon_label",
                    Kind::CircleButton => "circle_button",
                    Kind::PointedButton => "pointed_button",
                    Kind::MultilineButton => "multiline_button",
                    Kind::ButtonBar => "button_bar",
                    Kind::Breadcrumbs => "breadcrumbs",
                    Kind::LinkBar => "link_bar",
                    Kind::Tabs => "tabs",
                    Kind::VerticalTabs => "vertical_tabs",
                    Kind::List => "list",
                    _ => unreachable!(),
                };
                let parent = e.context_origin();
                let mut node = catalog::get(kind_id)
                    .unwrap()
                    .create(id, Point::new(x - parent.x, y - parent.y));
                if let Some(preset) = preset {
                    preset.apply(&mut node);
                }
                e.nodes_mut().push(node);
                e.selection = HashSet::from([id]);
                e.append_order(id);
                return;
            }
            let mut old = model::Document {
                next_id: id + 1,
                ..model::Document::default()
            };
            old.pages[0].controls.push(Control::new(id, kind, x, y));
            match Project::try_from(old) {
                Ok(mut p) => {
                    let mut node = p.pages[0].content.nodes.remove(0);
                    let parent = e.context_origin();
                    node.origin.x -= parent.x;
                    node.origin.y -= parent.y;
                    e.nodes_mut().push(node);
                    e.selection = HashSet::from([id]);
                    e.append_order(id);
                }
                Err(err) => e.error = Some(err),
            }
        });
    }
    pub fn add_page(&mut self) {
        self.edit("New page", |e| {
            if let Ok(id) = e.document.allocate_id() {
                e.document.pages.push(Page {
                    trashed: false,
                    id,
                    name: format!("Wireframe {}", e.document.live_pages().count() + 1),
                    notes: String::new(),
                    parent: None,
                    content: Content::default(),
                    alternates: vec![],
                });
                e.page = id;
                e.definition = None;
                e.library = None;
                e.context.clear();
                e.selection.clear();
            }
        });
    }
    pub fn delete_page(&mut self) {
        if self.in_source_workspace() {
            return;
        }
        if self.document.live_pages().count() < 2 {
            return;
        }
        self.edit("Delete page", |e| {
            let id = e.page;
            e.document.pages.retain(|p| p.id != id);
            e.document.remove_unowned_reviews();
            e.document.page_imports.remove(&id);
            for p in &mut e.document.pages {
                if p.parent == Some(id) {
                    p.parent = None;
                }
                clear_links(&mut p.content.nodes, id);
                for a in &mut p.alternates {
                    clear_links(&mut a.content.nodes, id);
                }
            }
            for d in &mut e.document.definitions {
                clear_links(&mut d.content.nodes, id);
            }
            for library in &mut e.document.libraries {
                clear_links(&mut library.content.nodes, id);
            }
            e.page = e.document.first_live_page().id;
            e.context.clear();
            e.selection.clear();
        });
    }
    pub fn select(&mut self, id: Id, toggle: bool) {
        if !self.nodes().iter().any(|n| n.id == id)
            || (!self.markup_visible && !self.current().controls.iter().any(|c| c.id == id))
        {
            return;
        }
        self.selection_order = self.ordered_selection();
        if !toggle {
            self.selection.clear();
            self.selection_order.clear();
        }
        if !toggle || !self.selection.remove(&id) {
            self.selection.insert(id);
            self.selection_order.push(id);
        } else {
            self.selection_order.retain(|selected| *selected != id);
        }
    }
    pub fn select_all(&mut self) {
        let visible: Option<HashSet<_>> =
            (!self.markup_visible).then(|| self.current().controls.iter().map(|c| c.id).collect());
        self.selection_order = self
            .nodes()
            .iter()
            .filter(|n| !n.locked && visible.as_ref().is_none_or(|ids| ids.contains(&n.id)))
            .map(|n| n.id)
            .collect();
        self.selection = self
            .nodes()
            .iter()
            .filter(|n| !n.locked && visible.as_ref().is_none_or(|ids| ids.contains(&n.id)))
            .map(|n| n.id)
            .collect();
    }
    pub fn select_area(&mut self, b: Bounds, previous: &HashSet<Id>) {
        self.selection_order.retain(|id| previous.contains(id));
        self.selection = previous.clone();
        let own_locks: HashSet<_> = self
            .nodes()
            .iter()
            .filter(|n| n.locked)
            .map(|n| n.id)
            .collect();
        for c in self.current().controls {
            if !own_locks.contains(&c.id)
                && b.contains(c.bounds.x, c.bounds.y)
                && b.contains(c.bounds.x + c.bounds.w, c.bounds.y + c.bounds.h)
            {
                self.selection.insert(c.id);
            }
        }
        self.retain_visible_selection();
    }
    pub fn bounds(&self) -> Option<Bounds> {
        self.selected_views()
            .iter()
            .map(|c| c.bounds)
            .reduce(Bounds::union)
    }
    pub fn movable_bounds(&self) -> Option<Bounds> {
        self.selected_views()
            .iter()
            .filter(|c| !c.locked)
            .map(|c| c.bounds)
            .reduce(Bounds::union)
    }
    pub fn selected_views(&self) -> Vec<Control> {
        if self.selection.is_empty() {
            return Vec::new();
        }
        self.current()
            .controls
            .into_iter()
            .filter(|c| self.selection.contains(&c.id))
            .collect()
    }
    pub fn selected_mut(&mut self, mut f: impl FnMut(&mut Control)) {
        for mut c in self.selected_views() {
            if !c.locked {
                f(&mut c);
                self.update_control(c);
            }
        }
    }
    pub fn update_control(&mut self, c: Control) {
        let Some(old) = self.current().controls.into_iter().find(|v| v.id == c.id) else {
            return;
        };
        let mut path = self.context.clone();
        path.push(c.id);
        let Ok(layout) = self.layout_page(self.surface_id()) else {
            return;
        };
        let Some(geometry) = view::find_layout(&layout.nodes, &path) else {
            return;
        };
        let parent = self.context_origin();
        let mut requested = c.bounds;
        if let Some(node) = self.nodes().iter().find(|n| n.id == c.id)
            && let NodeContent::Control { kind, .. } = &node.content
            && let Some(spec) = catalog::get(kind)
        {
            let (w, h) = spec.resized((old.bounds.w, old.bounds.h), (requested.w, requested.h));
            if w == old.bounds.w && requested.w != old.bounds.w {
                requested.x = old.bounds.x;
                requested.w = w;
            }
            if h == old.bounds.h && requested.h != old.bounds.h {
                requested.y = old.bounds.y;
                requested.h = h;
            }
        }
        let Some(mut edited) = self.nodes().iter().find(|n| n.id == c.id).cloned() else {
            return;
        };
        let before = edited.clone();
        {
            let node = &mut edited;
            {
                if !old.locked
                    && old.bounds != c.bounds
                    && let Err(error) = transform(
                        &self.document,
                        node,
                        geometry,
                        parent,
                        parent,
                        old.bounds,
                        requested,
                    )
                {
                    self.error = Some(error);
                    return;
                }
                if matches!(node.content, NodeContent::Control { .. }) {
                    if c.text != old.text {
                        if matches!(
                            c.kind,
                            Kind::List
                                | Kind::CheckboxGroup
                                | Kind::RadioGroup
                                | Kind::Menu
                                | Kind::MenuBar
                                | Kind::MobileMenu
                                | Kind::Tree
                                | Kind::SiteMap
                                | Kind::Accordion
                                | Kind::Alert
                                | Kind::ButtonBar
                                | Kind::Breadcrumbs
                                | Kind::LinkBar
                                | Kind::Tabs
                                | Kind::VerticalTabs
                        ) {
                            let baseline =
                                self.structured_property_baseline(c.id).unwrap_or_default();
                            let apply = if matches!(c.kind, Kind::Tabs | Kind::VerticalTabs) {
                                crate::tabs::apply_text
                            } else if matches!(c.kind, Kind::Breadcrumbs | Kind::LinkBar) {
                                crate::navigation::apply_text
                            } else if matches!(c.kind, Kind::CheckboxGroup | Kind::RadioGroup) {
                                crate::choice_group::apply_text
                            } else if c.kind == Kind::MobileMenu {
                                crate::mobile_menu::apply_text
                            } else if matches!(c.kind, Kind::Menu | Kind::MenuBar) {
                                crate::menu::apply_text
                            } else if c.kind == Kind::Alert {
                                crate::alert::apply_text
                            } else if c.kind == Kind::Accordion {
                                crate::accordion::apply_text
                            } else if c.kind == Kind::SiteMap {
                                crate::site_map::apply_text
                            } else if c.kind == Kind::Tree {
                                crate::tree::apply_text
                            } else if c.kind == Kind::ButtonBar {
                                crate::button_bar::apply_text
                            } else {
                                crate::list::apply_text
                            };
                            if let Err(error) = apply(node, &c.text, &baseline) {
                                self.error = Some(error);
                                return;
                            }
                        } else {
                            node.properties.insert("text".into(), Value::Text(c.text));
                        }
                    }
                    if c.font_size != old.font_size {
                        node.properties
                            .insert("font_size".into(), Value::Number(c.font_size as f64));
                    }
                    if c.bold != old.bold {
                        node.properties.insert("bold".into(), Value::Bool(c.bold));
                    }
                    if c.fill != old.fill {
                        node.properties.insert(
                            "fill".into(),
                            Value::Color([c.fill[0], c.fill[1], c.fill[2], 255]),
                        );
                    }
                    if c.checked != old.checked {
                        let state = if c.kind == Kind::Toggle {
                            if c.checked { "on" } else { "off" }
                        } else if c.checked {
                            "selected"
                        } else {
                            "normal"
                        };
                        node.properties
                            .insert("state".into(), Value::Choice(state.into()));
                        node.properties.remove("checked");
                    }
                }
                if c.link != old.link {
                    node.properties.insert(
                        "link".into(),
                        c.link.map_or(Value::Null, |id| Value::Link(Link::Page(id))),
                    );
                }
            }
        }
        if c.locked != old.locked {
            edited.locked = c.locked;
        }
        if let Err(error) = self.preserve_text_anchor(&before, &mut edited) {
            self.error = Some(error);
            return;
        }
        if self.editing_instance() {
            if let Err(error) = self.write_local_node(c.id, &before, &edited) {
                self.error = Some(error);
            }
        } else if let Some(node) = self.nodes_mut().iter_mut().find(|n| n.id == c.id) {
            *node = edited;
        }
    }
    pub fn move_by(&mut self, dx: f32, dy: f32) {
        self.edit("Move", |e| {
            e.selected_mut(|c| {
                c.bounds.x = (c.bounds.x + dx).clamp(-50_000., 50_000.);
                c.bounds.y = (c.bounds.y + dy).clamp(-50_000., 50_000.);
            })
        });
    }
    pub fn delete(&mut self) {
        if self.source_owner_locked() || !self.structure_edit_allowed() {
            return;
        }
        self.edit("Delete controls", |e| {
            let ids: HashSet<_> = e
                .nodes()
                .iter()
                .filter(|n| e.selection.contains(&n.id) && !n.locked)
                .map(|n| n.id)
                .collect();
            if e.definition.is_none()
                && e.context.is_empty()
                && let Some(library) = e.library
            {
                let library = e
                    .document
                    .libraries
                    .iter_mut()
                    .find(|l| l.id == library)
                    .unwrap();
                let definitions: HashSet<_> = library
                    .sources
                    .iter()
                    .filter(|s| ids.contains(&s.node))
                    .map(|s| s.definition)
                    .collect();
                library.sources.retain(|s| !ids.contains(&s.node));
                e.document
                    .definitions
                    .retain(|d| !definitions.contains(&d.id));
            }
            e.nodes_mut().retain(|n| !ids.contains(&n.id));
            let context = e.context.clone();
            e.active_content_mut().paint_order.retain(|p| {
                !(p.starts_with(&context)
                    && p.get(context.len()).is_some_and(|id| ids.contains(id)))
            });
            e.selection.clear();
        });
    }
    pub fn group(&mut self) {
        if self.source_owner_locked() {
            return;
        }
        if self.at_library_root() {
            if let Err(error) = self.group_library_selection() {
                self.error = Some(error);
            }
            return;
        }
        if !self.structure_edit_allowed() {
            return;
        }
        let ids: HashSet<_> = self
            .nodes()
            .iter()
            .filter(|n| self.selection.contains(&n.id) && !n.locked)
            .map(|n| n.id)
            .collect();
        if ids.len() < 2 {
            return;
        }
        self.edit("Group", |e| {
            let order = e.leaf_order();
            let Ok(id) = e.document.allocate_id() else {
                return;
            };
            let b = e
                .current()
                .controls
                .into_iter()
                .filter(|c| ids.contains(&c.id))
                .map(|c| c.bounds)
                .reduce(Bounds::union)
                .unwrap();
            let parent = e.context_origin();
            let origin = Point::new(b.x - parent.x, b.y - parent.y);
            let nodes = e.nodes_mut();
            let index = nodes.iter().position(|n| ids.contains(&n.id)).unwrap();
            let mut grouped = vec![];
            nodes.retain_mut(|n| {
                if ids.contains(&n.id) {
                    let mut child = n.clone();
                    child.origin.x -= origin.x;
                    child.origin.y -= origin.y;
                    grouped.push(child);
                    false
                } else {
                    true
                }
            });
            nodes.insert(index, Node::group(id, origin, grouped));
            let context = e.context.clone();
            e.active_content_mut().paint_order = order
                .into_iter()
                .map(|mut p| {
                    if p.starts_with(&context)
                        && p.get(context.len()).is_some_and(|n| ids.contains(n))
                    {
                        p.insert(context.len(), id);
                    }
                    p
                })
                .collect();
            e.selection = HashSet::from([id]);
        });
    }
    pub fn ungroup(&mut self) {
        if self.source_owner_locked() {
            return;
        }
        if self.at_library_root() {
            if let Err(error) = self.ungroup_library_selection() {
                self.error = Some(error);
            }
            return;
        }
        if !self.structure_edit_allowed() {
            return;
        }
        if !self.nodes().iter().any(|n| {
            self.selection.contains(&n.id)
                && !n.locked
                && matches!(n.content, NodeContent::Group { .. })
        }) {
            return;
        }
        self.edit("Ungroup", |e| {
            let order = e.leaf_order();
            let selected = e.selection.clone();
            let context = e.context.clone();
            let mut remap = HashMap::new();
            let mut selection = selected.clone();
            let mut removed = HashSet::new();
            let nodes = std::mem::take(e.nodes_mut());
            let mut result = vec![];
            let mut used: HashSet<_> = nodes.iter().map(|n| n.id).collect();
            for n in nodes {
                if selected.contains(&n.id)
                    && !n.locked
                    && matches!(n.content, NodeContent::Group { .. })
                {
                    let id = n.id;
                    match n.clone().ungroup() {
                        Ok(mut children) => {
                            removed.insert(id);
                            selection.remove(&id);
                            for child in &mut children {
                                let old = child.id;
                                if used.contains(&old)
                                    && let Ok(new) = e.document.allocate_id()
                                {
                                    child.id = new;
                                }
                                used.insert(child.id);
                                remap.insert((id, old), child.id);
                                selection.insert(child.id);
                            }
                            result.extend(children);
                        }
                        Err(error) => {
                            e.error = Some(error);
                            result.push(n);
                        }
                    }
                } else {
                    result.push(n);
                }
            }
            *e.nodes_mut() = result;
            e.active_content_mut().paint_order = order
                .into_iter()
                .filter_map(|mut p| {
                    // Empty groups are paint units too; their removed paths
                    // must not survive after the wrapper is dissolved.
                    if p.starts_with(&context)
                        && p.len() == context.len() + 1
                        && removed.contains(&p[context.len()])
                    {
                        return None;
                    }
                    if p.starts_with(&context)
                        && p.len() > context.len() + 1
                        && let Some(id) = remap.get(&(p[context.len()], p[context.len() + 1]))
                    {
                        p.remove(context.len());
                        p[context.len()] = *id;
                    }
                    Some(p)
                })
                .collect();
            e.selection = selection;
        });
    }
    pub fn reorder(&mut self, front: bool) {
        self.arrange(if front {
            LayerAction::Front
        } else {
            LayerAction::Back
        });
    }
    fn context_origin(&self) -> Point {
        let resolved;
        let mut nodes = if self.editing_instance() {
            resolved = self
                .document
                .resolve_all(&self.active_content().nodes)
                .unwrap_or_default()
                .into_iter()
                .map(|r| r.root)
                .collect::<Vec<_>>();
            &resolved
        } else {
            &self.active_content().nodes
        };
        let mut point = Point::default();
        for id in &self.context {
            if let Some(n) = nodes.iter().find(|n| n.id == *id) {
                point = point.translated(n.origin);
                if let NodeContent::Group { children } = &n.content {
                    nodes = children;
                }
            }
        }
        point
    }
    fn repair_empty_order(&mut self, before: &Snapshot) -> Result<(), String> {
        let id = self.surface_id();
        if self.surface(id).is_none() {
            return Ok(());
        }
        let content = before.document.surface(id).map(|(_, content)| content);
        if let Some(content) = content {
            self.active_content_mut().repair_empty_order(content)?;
        }
        Ok(())
    }
    fn leaf_order(&self) -> Vec<NodePath> {
        self.active_content().paint_paths().unwrap_or_default()
    }
    fn append_order(&mut self, id: Id) {
        if !self.active_content().paint_order.is_empty() {
            let mut p = self.context.clone();
            p.push(id);
            self.active_content_mut().paint_order.push(p);
        }
    }
}
fn children<'a>(nodes: &'a [Node], path: &[Id]) -> Option<&'a [Node]> {
    if path.is_empty() {
        return Some(nodes);
    }
    let n = nodes.iter().find(|n| n.id == path[0])?;
    match &n.content {
        NodeContent::Group { children: c } => children(c, &path[1..]),
        _ => None,
    }
}
fn children_mut<'a>(nodes: &'a mut Vec<Node>, path: &[Id]) -> Option<&'a mut Vec<Node>> {
    if path.is_empty() {
        return Some(nodes);
    }
    let n = nodes.iter_mut().find(|n| n.id == path[0])?;
    match &mut n.content {
        NodeContent::Group { children: c } => children_mut(c, &path[1..]),
        _ => None,
    }
}
fn clear_links(nodes: &mut [Node], page: Id) {
    fn clear(p: &mut Properties, page: Id) {
        for v in p.values_mut() {
            if let Value::Links(links) = v {
                for target in links {
                    if let Some(Link::Page(id)) = target
                        && *id == page
                    {
                        *target = None;
                    }
                }
            }
            if *v == Value::Link(Link::Page(page)) {
                *v = Value::Null;
            }
        }
    }
    for n in nodes {
        clear(&mut n.properties, page);
        match &mut n.content {
            NodeContent::Group { children } => clear_links(children, page),
            NodeContent::Instance { overrides, .. } => {
                for o in overrides {
                    clear(&mut o.patch.properties, page);
                }
            }
            _ => {}
        }
    }
}
fn transform(
    project: &Project,
    node: &mut Node,
    layout: &crate::layout::LayoutNode,
    parent: Point,
    new_parent: Point,
    from: Bounds,
    to: Bounds,
) -> Result<(), String> {
    if matches!(node.content, NodeContent::Instance { .. }) {
        return overrides::transform_instance(project, node, layout, parent, new_parent, from, to);
    }
    let sx = if from.w > 0. { to.w / from.w } else { 1. };
    let sy = if from.h > 0. { to.h / from.h } else { 1. };
    let world = node.origin.translated(parent);
    let mut next = Point::new(
        to.x + (world.x - from.x) * sx,
        to.y + (world.y - from.y) * sy,
    );
    if node.properties.get(MISSING_COMPONENT) == Some(&Value::Bool(true))
        || matches!(&node.content, NodeContent::Group { children } if children.is_empty())
    {
        node.origin = Point::new(next.x - new_parent.x, next.y - new_parent.y);
        return Ok(());
    }
    if let NodeContent::Control { kind, .. } = &node.content
        && let Some(spec) = catalog::get(kind)
    {
        let same = |a: f32, b: f32| (a - b).abs() < 0.001;
        match spec.resize {
            catalog::ResizeAxes::Horizontal => {
                let height = layout.uncropped.height;
                if same(world.y, from.y) {
                    next.y = to.y;
                } else if same(world.y + height, from.y + from.h)
                    || (same(from.y + from.h, to.y + to.h) && next.y + height > to.y + to.h)
                {
                    next.y = to.y + to.h - height;
                }
            }
            catalog::ResizeAxes::Vertical => {
                let width = layout.uncropped.width;
                if same(world.x, from.x) {
                    next.x = to.x;
                } else if same(world.x + width, from.x + from.w) {
                    next.x = to.x + to.w - width;
                }
            }
            _ => {}
        }
    }
    let resizing = from.w != to.w || from.h != to.h;
    if resizing {
        match node.content {
            NodeContent::Control { .. } => {
                // Round after the parent displacement is included, in the original
                // parent coordinate system. Rounding child-local scales loses pixels.
                next.x = parent.x + crate::model::round_coordinate(next.x - parent.x);
                next.y = parent.y + crate::model::round_coordinate(next.y - parent.y);
            }
            NodeContent::Group { .. } => {
                // Keep the coordinate anchor stable while descendants transform.
                // The visible group rectangle is derived from their rounded union.
                next = world;
            }
            NodeContent::Instance { .. } => unreachable!(),
        }
    }
    node.origin = Point::new(next.x - new_parent.x, next.y - new_parent.y);
    if resizing && let Some((arrow, _)) = crate::arrows::node_geometry(node) {
        let old = Point::new(layout.uncropped.width, layout.uncropped.height);
        let (w, h) = catalog::get("arrow")
            .unwrap()
            .resized((old.x, old.y), (old.x * sx, old.y * sy));
        arrow
            .resize(
                old,
                Bounds::new(
                    node.origin.x,
                    node.origin.y,
                    crate::model::round_coordinate(w),
                    crate::model::round_coordinate(h),
                ),
            )
            .apply(node);
        return Ok(());
    }
    match &mut node.content {
        NodeContent::Control { kind, size } => {
            if from.w != to.w || from.h != to.h {
                let w = layout.uncropped.width;
                let h = layout.uncropped.height;
                let (nw, nh) = catalog::get(kind)
                    .map_or((w * sx, h * sy), |s| s.resized((w, h), (w * sx, h * sy)));
                let axes = catalog::get(kind)
                    .map(|s| s.resize)
                    .unwrap_or(catalog::ResizeAxes::Both);
                if matches!(
                    axes,
                    catalog::ResizeAxes::Both | catalog::ResizeAxes::Horizontal
                ) {
                    size.width = Dimension::Fixed(crate::model::round_coordinate(nw));
                }
                if matches!(
                    axes,
                    catalog::ResizeAxes::Both | catalog::ResizeAxes::Vertical
                ) {
                    size.height = Dimension::Fixed(crate::model::round_coordinate(nh));
                }
            }
        }
        NodeContent::Group { children } => {
            for child in children {
                if let Some(l) = layout
                    .children
                    .iter()
                    .find(|l| l.path.last() == Some(&child.id))
                {
                    // Fixed-axis descendants anchor to the selected crop frame,
                    // including when their artwork lies outside that frame.
                    let group_from = Bounds::new(
                        layout.bounds.x,
                        layout.bounds.y,
                        layout.bounds.width,
                        layout.bounds.height,
                    );
                    let group_to = Bounds::new(
                        to.x + (group_from.x - from.x) * sx,
                        to.y + (group_from.y - from.y) * sy,
                        group_from.w * sx,
                        group_from.h * sy,
                    );
                    transform(project, child, l, world, next, group_from, group_to)?;
                }
            }
        }
        NodeContent::Instance { .. } => unreachable!(),
    }
    Ok(())
}
