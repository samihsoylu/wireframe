// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Stable catalog identity and destination policy shared by insertion surfaces.
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CatalogItem {
    Control(Kind),
    Preset(CatalogPreset),
    Component(Id),
    Asset(Id),
    Icon(&'static str),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CatalogCategory {
    All,
    Android,
    Assets,
    Big,
    Ios,
    Buttons,
    Common,
    Containers,
    Forms,
    Layout,
    Navigation,
    Media,
    Markup,
    Charts,
    Icons,
    Text,
    Components,
}
impl CatalogCategory {
    pub const ALL: [Self; 15] = [
        Self::All,
        Self::Android,
        Self::Assets,
        Self::Big,
        Self::Buttons,
        Self::Common,
        Self::Containers,
        Self::Forms,
        Self::Icons,
        Self::Ios,
        Self::Layout,
        Self::Markup,
        Self::Media,
        Self::Components,
        Self::Text,
    ];
    /// Old category choices remain visible until the user replaces them.
    pub fn visible(selected: &[Self]) -> Vec<Self> {
        let mut visible = Self::ALL.to_vec();
        for category in selected {
            if !visible.contains(category) {
                visible.push(*category);
            }
        }
        visible
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Android => "Android",
            Self::Assets => "Assets",
            Self::Big => "Big",
            Self::Ios => "iOS",
            Self::Buttons => "Buttons",
            Self::Common => "Common",
            Self::Containers => "Containers",
            Self::Forms => "Forms",
            Self::Layout => "Layout",
            Self::Navigation => "Navigation",
            Self::Media => "Media",
            Self::Markup => "Markup",
            Self::Charts => "Charts",
            Self::Icons => "Icons",
            Self::Text => "Text",
            Self::Components => "Components",
        }
    }
    fn contains(self, item: CatalogItem) -> bool {
        if let CatalogItem::Icon(_) = item {
            return self == Self::Icons;
        }
        if self == Self::All {
            return true;
        }
        let kind = match item {
            CatalogItem::Component(_) => return self == Self::Components,
            CatalogItem::Asset(_) => return self == Self::Assets,
            CatalogItem::Preset(preset) => {
                return match preset {
                    CatalogPreset::IosPointedButton => self == Self::Ios,
                    _ => self == Self::Android,
                };
            }
            CatalogItem::Control(kind) => kind,
            CatalogItem::Icon(_) => unreachable!(),
        };
        use Kind::*;
        let kinds: &[Kind] = match self {
            Self::Android => &[AppBar, CircleButton, DatePicker, Smartphone, TimePicker],
            Self::Big => &[
                Phone,
                Tablet,
                MultilineButton,
                FieldSet,
                ModalOverlay,
                Image,
                StreetMap,
                PieChart,
                ColumnChart,
                BarChart,
                LineChart,
                CoverFlow,
                FormattingToolbar,
                VerticalTabs,
                Video,
                Tabs,
                Webcam,
                Browser,
                Panel,
                Rectangle,
                Heading,
                TextArea,
                SiteMap,
                Smartphone,
            ],
            Self::Buttons => &[
                CheckboxGroup,
                RadioGroup,
                Toggle,
                PointedButton,
                MultilineButton,
                Playback,
                NumericStepper,
                DateChooser,
                HelpButton,
                ButtonBar,
                ColorPicker,
                Dropdown,
                Button,
                Checkbox,
                Radio,
                CircleButton,
            ],
            Self::Common => &[
                CheckboxGroup,
                RadioGroup,
                Image,
                Browser,
                MenuBar,
                IconLabel,
                Icon,
                VerticalScrollbar,
                Dropdown,
                Panel,
                Button,
                Shape,
                Checkbox,
                Radio,
                Rectangle,
                Paragraph,
                Link,
                Label,
                TextInput,
                TextArea,
                TextBlock,
                TextLine,
                CircleButton,
            ],
            Self::Containers => &[
                FieldSet,
                VerticalTabs,
                Tabs,
                Browser,
                Panel,
                Shape,
                Rectangle,
                Phone,
                Tablet,
                Smartphone,
            ],
            Self::Forms => &[
                CheckboxGroup,
                RadioGroup,
                Subtitle,
                Toggle,
                PointedButton,
                FieldSet,
                FormattingToolbar,
                NumericStepper,
                DateChooser,
                Line,
                HorizontalSlider,
                List,
                IconLabel,
                Icon,
                HelpButton,
                ButtonBar,
                ColorPicker,
                Dropdown,
                Button,
                Checkbox,
                Radio,
                Paragraph,
                Link,
                Label,
                TextInput,
                TextArea,
                CircleButton,
            ],
            Self::Ios => &[
                MobilePicker,
                Alert,
                MobileMenu,
                MobileKeyboard,
                Toggle,
                Phone,
                Tablet,
                Popover,
            ],
            Self::Layout => &[
                FieldSet,
                VerticalRule,
                VerticalTabs,
                Tabs,
                Accordion,
                LinkBar,
                Line,
                HorizontalSplitter,
                VerticalSplitter,
                ButtonBar,
                VerticalScrollbar,
                HorizontalScrollbar,
                SiteMap,
            ],
            Self::Markup => &[
                Arrow,
                HorizontalBrace,
                VerticalBrace,
                RedCross,
                ScratchOut,
                Callout,
                StickyNote,
            ],
            Self::Media => &[
                Image,
                StreetMap,
                CoverFlow,
                Video,
                Playback,
                VolumeSlider,
                Webcam,
                VerticalSlider,
                HorizontalSlider,
                IconLabel,
                Icon,
                Progress,
            ],
            Self::Text => &[
                Subtitle,
                Tooltip,
                TagCloud,
                Tree,
                NumericStepper,
                Table,
                Menu,
                Breadcrumbs,
                LinkBar,
                List,
                MenuBar,
                IconLabel,
                Search,
                Dropdown,
                Paragraph,
                Link,
                Heading,
                Label,
                TextInput,
                TextArea,
                TextBlock,
                TextLine,
            ],
            // Compatibility categories retain their old kind filters.
            Self::Navigation | Self::Charts => return kind.category() == self.name(),
            Self::Icons => return false,
            Self::All | Self::Components | Self::Assets => &[],
        };
        kinds.contains(&kind)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CatalogEntry {
    pub item: CatalogItem,
    pub name: String,
    pub enabled: bool,
}

impl Editor {
    /// Union by stable identity, with an empty choice falling back to All.
    pub fn catalog_categories(&self, categories: &[CatalogCategory]) -> Vec<CatalogEntry> {
        let mut entries = self.catalog_entries();
        if categories.contains(&CatalogCategory::Icons) {
            entries.extend(self.icon_catalog_entries());
        }
        entries.retain(|entry| {
            if categories.is_empty() {
                CatalogCategory::All.contains(entry.item)
            } else {
                categories
                    .iter()
                    .any(|category| category.contains(entry.item))
            }
        });
        entries.sort_by_cached_key(|entry| entry.name.to_lowercase());
        entries
    }
    /// Lookup only the requested icon instead of constructing its full catalog
    /// during each gesture validation or commit.
    pub fn catalog_entry(&self, item: CatalogItem) -> Option<CatalogEntry> {
        if let CatalogItem::Icon(id) = item {
            let icon = crate::icons::get(id)?;
            return Some(CatalogEntry {
                item,
                name: icon.catalog_name(),
                enabled: self.require_structure_edit().is_ok(),
            });
        }
        self.catalog_entries()
            .into_iter()
            .find(|entry| entry.item == item)
    }
    fn icon_catalog_entries(&self) -> impl Iterator<Item = CatalogEntry> + '_ {
        let enabled = self.require_structure_edit().is_ok();
        super::icon_catalog::entries()
            .iter()
            .cloned()
            .map(move |mut entry| {
                entry.enabled = enabled;
                entry
            })
    }
    /// Names are presentation only: same-name sources retain distinct identities.
    pub fn catalog_entries(&self) -> Vec<CatalogEntry> {
        let structure_allowed = self.require_structure_edit().is_ok();
        let blocked = self.component_catalog_blocked();
        let components_allowed =
            structure_allowed && (self.library.is_none() || self.definition.is_some());
        let mut entries: Vec<_> = Kind::ALL
            .into_iter()
            .map(|kind| CatalogEntry {
                item: CatalogItem::Control(kind),
                name: kind.catalog_name().into(),
                enabled: structure_allowed,
            })
            .chain(CatalogPreset::ALL.into_iter().map(|preset| CatalogEntry {
                item: CatalogItem::Preset(preset),
                name: preset.catalog_name().into(),
                enabled: structure_allowed,
            }))
            .chain(self.document.live_assets().filter_map(|asset| {
                crate::images::dimensions(&asset.bytes).ok()?;
                Some(CatalogEntry {
                    item: CatalogItem::Asset(asset.id),
                    name: asset.name.clone(),
                    enabled: structure_allowed,
                })
            }))
            .chain(
                self.document
                    .live_definitions()
                    .map(|definition| CatalogEntry {
                        item: CatalogItem::Component(definition.id),
                        name: definition.name.clone(),
                        enabled: components_allowed && !blocked.contains(&definition.id),
                    }),
            )
            .collect();
        // Stable sort retains source order when names compare equally.
        entries.sort_by_cached_key(|entry| entry.name.to_lowercase());
        entries
    }

    /// Sources that would lead back to the current source, including itself.
    /// Build reverse edges once, then traverse each dependency at most once.
    fn component_catalog_blocked(&self) -> HashSet<Id> {
        let Some(active) = self.definition else {
            return HashSet::new();
        };
        let mut dependents: HashMap<Id, Vec<Id>> = HashMap::new();
        for definition in &self.document.definitions {
            let mut dependencies = HashSet::new();
            clipboard::references(
                &definition.content.nodes,
                &mut dependencies,
                &mut HashSet::new(),
            );
            for dependency in dependencies {
                // Retained definitions still belong to the canonical graph.
                // Preserve the model's cycle invariant across later Restore.
                dependents
                    .entry(dependency)
                    .or_default()
                    .push(definition.id);
            }
        }
        let mut blocked = HashSet::from([active]);
        let mut pending = vec![active];
        while let Some(id) = pending.pop() {
            for dependent in dependents.get(&id).into_iter().flatten() {
                if blocked.insert(*dependent) {
                    pending.push(*dependent);
                }
            }
        }
        blocked
    }

    /// Recheck at commit, before committing any unrelated pending edit or
    /// allocating identities. The underlying transaction still validates depth,
    /// size and graph constraints and rolls back failures.
    pub fn insert_catalog_component(
        &mut self,
        definition: Id,
        x: f32,
        y: f32,
    ) -> Result<Id, String> {
        self.require_structure_edit()?;
        if !self.document.definition_available(definition) {
            return Err("Component no longer exists".into());
        }
        if self.library.is_some() && self.definition.is_none() {
            return Err(
                "Choose a component source or wireframe before inserting a component".into(),
            );
        }
        if self.component_catalog_blocked().contains(&definition) {
            return Err(
                "A component cannot contain itself or a component that depends on it".into(),
            );
        }
        if self.pending.is_some() {
            // A rejected insertion must not finalize or discard an in-progress
            // edit. Validate against a disposable destination before the normal
            // transaction commits that edit. This extra copy occurs only when
            // an unrelated model edit is still pending, never during catalog paint.
            let mut check = Editor::new(self.document.clone());
            check.page = self.page;
            check.active_versions = self.active_versions.clone();
            check.definition = self.definition;
            check.library = self.library;
            check.context = self.context.clone();
            check.insert_component(definition, x, y)?;
        }
        self.insert_component(definition, x, y)
    }

    /// Quick Add includes eligible sources while retaining built-in aliases and
    /// exact/prefix/token ranking. Equal names never collapse distinct sources.
    pub fn search_catalog(&self, query: &str) -> Vec<CatalogEntry> {
        let query = query.trim().to_lowercase();
        let words: Vec<_> = query.split_whitespace().collect();

        let mut found: Vec<_> = self
            .catalog_entries()
            .into_iter()
            .chain(self.icon_catalog_entries())
            .filter_map(|entry| {
                if !entry.enabled {
                    return None;
                }
                let name = entry.name.to_lowercase();
                let rank = match entry.item {
                    CatalogItem::Control(kind) => {
                        let spec = catalog::get(kind.catalog_id())?;
                        // Suppress a legacy alias only when a separate canonical
                        // kind exists. A horizontal rule is represented solely
                        // by Line, so its alias must remain searchable.
                        if spec.id != kind.catalog_id()
                            && Kind::ALL.iter().any(|other| other.catalog_id() == spec.id)
                        {
                            return None;
                        }
                        let display_rank = if query.is_empty() || name == query {
                            Some(0)
                        } else if name.starts_with(&query) {
                            Some(2)
                        } else if words.iter().all(|word| name.contains(word)) {
                            Some(3)
                        } else {
                            None
                        };
                        // Keep established alias ranking for existing queries.
                        catalog::search_rank(spec, &query).or(display_rank)?
                    }
                    CatalogItem::Preset(preset) => {
                        let spec = catalog::get(preset.kind().catalog_id())?;
                        let legacy_name = preset.name().to_lowercase();
                        let searchable = format!(
                            "{} {} {}",
                            name,
                            legacy_name,
                            spec.aliases.join(" ").to_lowercase()
                        );
                        if query.is_empty() || name == query || legacy_name == query {
                            0
                        } else if name.starts_with(&query) || legacy_name.starts_with(&query) {
                            2
                        } else if words.iter().all(|word| searchable.contains(word)) {
                            3
                        } else {
                            return None;
                        }
                    }
                    CatalogItem::Icon(id) => {
                        let icon = crate::icons::get(id)?;
                        if !icon.matches_query(&words) {
                            return None;
                        }
                        if query.is_empty() || name == query || id == query {
                            0
                        } else if name.starts_with(&query) {
                            2
                        } else {
                            3
                        }
                    }
                    CatalogItem::Component(_) | CatalogItem::Asset(_) => {
                        if query.is_empty() || name == query {
                            0
                        } else if name.starts_with(&query) {
                            2
                        } else if words.iter().all(|word| name.contains(word)) {
                            3
                        } else {
                            return None;
                        }
                    }
                };
                let sort_name = match entry.item {
                    CatalogItem::Control(kind) => catalog::get(kind.catalog_id())?.name.to_owned(),
                    CatalogItem::Preset(preset) => preset.name().to_owned(),
                    CatalogItem::Component(_) | CatalogItem::Asset(_) | CatalogItem::Icon(_) => {
                        entry.name.clone()
                    }
                };
                Some((rank, sort_name, entry))
            })
            .collect();
        found.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
        found.into_iter().map(|(_, _, entry)| entry).collect()
    }
}
