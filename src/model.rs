// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub type Id = u64;
pub const FORMAT_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Bounds {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}
impl Bounds {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }
    pub fn contains(self, x: f32, y: f32) -> bool {
        x >= self.x && x <= self.x + self.w && y >= self.y && y <= self.y + self.h
    }
    pub fn intersects(self, b: Self) -> bool {
        self.x <= b.x + b.w
            && self.x + self.w >= b.x
            && self.y <= b.y + b.h
            && self.y + self.h >= b.y
    }
    pub fn union(self, b: Self) -> Self {
        let x = self.x.min(b.x);
        let y = self.y.min(b.y);
        Self::new(
            x,
            y,
            (self.x + self.w).max(b.x + b.w) - x,
            (self.y + self.h).max(b.y + b.h) - y,
        )
    }
    pub fn resized(self, handle: (i8, i8), delta: (f32, f32), aspect: bool) -> Self {
        let (hx, hy) = handle;
        let mut w = if hx == 0 {
            self.w
        } else {
            (self.w + delta.0 * hx as f32).clamp(8., 10_000.)
        };
        let mut h = if hy == 0 {
            self.h
        } else {
            (self.h + delta.1 * hy as f32).clamp(8., 10_000.)
        };
        if aspect {
            let sx = w / self.w;
            let sy = h / self.h;
            let scale = if hx == 0 {
                sy
            } else if hy == 0 || (sx - 1.).abs() > (sy - 1.).abs() {
                sx
            } else {
                sy
            };
            w = self.w * scale;
            h = self.h * scale;
        }
        let x = if hx < 0 {
            self.x + self.w - w
        } else if hx == 0 {
            self.x + (self.w - w) / 2.
        } else {
            self.x
        };
        let y = if hy < 0 {
            self.y + self.h - h
        } else if hy == 0 {
            self.y + (self.h - h) / 2.
        } else {
            self.y
        };
        Self::new(x, y, w, h)
    }
    pub fn valid(self) -> bool {
        [self.x, self.y, self.w, self.h]
            .iter()
            .all(|v| v.is_finite() && v.abs() <= 100_000.)
            && self.w >= 1.
            && self.h >= 1.
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kind {
    Button,
    Label,
    Heading,
    Paragraph,
    TextInput,
    TextArea,
    Checkbox,
    Radio,
    CheckboxGroup,
    RadioGroup,
    Menu,
    MenuBar,
    Tree,
    Accordion,
    Alert,
    AppBar,
    Dropdown,
    Search,
    Rectangle,
    Ellipse,
    Line,
    Arrow,
    Image,
    Icon,
    IconLabel,
    CircleButton,
    PointedButton,
    MultilineButton,
    ButtonBar,
    Breadcrumbs,
    LinkBar,
    Browser,
    Phone,
    Panel,
    List,
    Table,
    Tabs,
    VerticalTabs,
    Progress,
    Toggle,
    Link,
}
impl Kind {
    pub const ALL: [Kind; 41] = [
        Self::Button,
        Self::Label,
        Self::Heading,
        Self::Paragraph,
        Self::TextInput,
        Self::TextArea,
        Self::Checkbox,
        Self::Radio,
        Self::CheckboxGroup,
        Self::RadioGroup,
        Self::Menu,
        Self::MenuBar,
        Self::Tree,
        Self::Accordion,
        Self::Alert,
        Self::AppBar,
        Self::Dropdown,
        Self::Search,
        Self::Rectangle,
        Self::Ellipse,
        Self::Line,
        Self::Arrow,
        Self::Image,
        Self::Icon,
        Self::IconLabel,
        Self::CircleButton,
        Self::PointedButton,
        Self::MultilineButton,
        Self::ButtonBar,
        Self::Breadcrumbs,
        Self::LinkBar,
        Self::Browser,
        Self::Phone,
        Self::Panel,
        Self::List,
        Self::Table,
        Self::Tabs,
        Self::VerticalTabs,
        Self::Progress,
        Self::Toggle,
        Self::Link,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::Button => "Button",
            Self::Label => "Text label",
            Self::Heading => "Heading",
            Self::Paragraph => "Paragraph",
            Self::TextInput => "Text input",
            Self::TextArea => "Text area",
            Self::Checkbox => "Checkbox",
            Self::Radio => "Radio button",
            Self::CheckboxGroup => "Checkbox group",
            Self::RadioGroup => "Radio group",
            Self::Menu => "Menu",
            Self::MenuBar => "Menu bar",
            Self::Tree => "Tree pane",
            Self::Accordion => "Accordion",
            Self::Alert => "Alert dialog",
            Self::AppBar => "App bar",
            Self::Dropdown => "Dropdown",
            Self::Search => "Search box",
            Self::Rectangle => "Rectangle",
            Self::Ellipse => "Ellipse",
            Self::Line => "Line",
            Self::Arrow => "Arrow",
            Self::Image => "Image",
            Self::Icon => "Icon",
            Self::IconLabel => "Icon and label",
            Self::CircleButton => "Circle button",
            Self::PointedButton => "Pointed button",
            Self::MultilineButton => "Multiline button",
            Self::ButtonBar => "Button bar",
            Self::Breadcrumbs => "Breadcrumbs",
            Self::LinkBar => "Link bar",
            Self::Browser => "Browser",
            Self::Phone => "Phone",
            Self::Panel => "Panel",
            Self::List => "List",
            Self::Table => "Table",
            Self::Tabs => "Tabs",
            Self::VerticalTabs => "Vertical tabs",
            Self::Progress => "Progress bar",
            Self::Toggle => "Toggle",
            Self::Link => "Link",
        }
    }
    pub fn category(self) -> &'static str {
        match self {
            Self::Label
            | Self::Heading
            | Self::Paragraph
            | Self::Link
            | Self::Breadcrumbs
            | Self::LinkBar => "Text",
            Self::Icon | Self::IconLabel => "Icons",
            Self::Button
            | Self::CircleButton
            | Self::PointedButton
            | Self::MultilineButton
            | Self::ButtonBar
            | Self::TextInput
            | Self::TextArea
            | Self::Checkbox
            | Self::Radio
            | Self::CheckboxGroup
            | Self::RadioGroup
            | Self::Dropdown
            | Self::Search
            | Self::Toggle => "Forms",
            Self::Rectangle | Self::Ellipse | Self::Line | Self::Arrow => "Shapes",
            _ => "Layout",
        }
    }
    pub fn size(self) -> (f32, f32) {
        match self {
            Self::Button => (120., 38.),
            Self::Icon => (24., 24.),
            Self::CircleButton => (48., 48.),
            Self::PointedButton => (69., 27.),
            Self::MultilineButton => (118., 66.),
            Self::ButtonBar => (159., 21.),
            Self::Breadcrumbs => (210., 21.),
            Self::LinkBar => (226., 21.),
            Self::IconLabel => (65., 63.),
            Self::Label | Self::Link => (140., 28.),
            Self::Heading => (300., 48.),
            Self::Paragraph => (300., 100.),
            Self::TextInput | Self::Dropdown | Self::Search => (220., 38.),
            Self::TextArea => (260., 120.),
            Self::Checkbox | Self::Radio => (180., 28.),
            Self::CheckboxGroup => (155., 149.),
            Self::RadioGroup => (165., 181.),
            Self::Menu => (150., 167.),
            Self::MenuBar => (192., 32.),
            Self::Tree => (300., 285.),
            Self::Accordion => (150., 186.),
            Self::Alert => (200., 102.),
            Self::AppBar => (320., 44.),
            Self::Line | Self::Arrow => (180., 20.),
            Self::Browser => (600., 420.),
            Self::Phone => (240., 440.),
            Self::Panel => (300., 220.),
            Self::List | Self::Table => (300., 180.),
            Self::Tabs => (199., 100.),
            Self::VerticalTabs => (200., 178.),
            Self::Progress => (240., 24.),
            Self::Toggle => (54., 28.),
            _ => (180., 120.),
        }
    }
    pub fn initial_text(self) -> &'static str {
        match self {
            Self::Button | Self::PointedButton => "Button",
            Self::IconLabel => "Icon Name",
            Self::MultilineButton => "Multiline Button\nSecond line of text",
            Self::ButtonBar => "One, Two, Three",
            Self::Breadcrumbs => crate::navigation::BREADCRUMBS,
            Self::LinkBar => crate::navigation::LINK_BAR,
            Self::Label => "Some text",
            Self::Heading => "Your next great idea",
            Self::Paragraph => "Describe the idea here.\nKeep it simple and make it clear.",
            Self::TextInput => "Enter text…",
            Self::TextArea => "Write something…",
            Self::Checkbox | Self::Radio => "Option",
            Self::CheckboxGroup => crate::choice_group::CHECKBOX,
            Self::RadioGroup => crate::choice_group::RADIO,
            Self::Menu => crate::menu::MENU,
            Self::MenuBar => crate::menu::BAR,
            Self::Tree => crate::tree::DEFAULT,
            Self::Accordion => crate::accordion::DEFAULT,
            Self::Alert => crate::alert::DEFAULT,
            Self::AppBar => crate::app_bar::DEFAULT,
            Self::Dropdown => "Choose an option",
            Self::Search => "Search…",
            Self::Browser => "https://example.com",
            Self::Panel => "Panel title",
            Self::List => "First item\nSecond item\nThird item\nFourth item",
            Self::Table => {
                "Name,Status,Owner\nHomepage,Draft,Alex\nSettings,Ready,Sam\nProfile,Draft,Jo"
            }
            Self::Tabs => crate::tabs::HORIZONTAL,
            Self::VerticalTabs => crate::tabs::VERTICAL,
            Self::Link => "Learn more",
            _ => "",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Control {
    pub id: Id,
    pub kind: Kind,
    pub bounds: Bounds,
    pub text: String,
    pub font_size: f32,
    pub bold: bool,
    pub fill: [u8; 3],
    pub checked: bool,
    pub locked: bool,
    pub group: Option<Id>,
    pub link: Option<Id>,
}
impl Control {
    pub fn new(id: Id, kind: Kind, x: f32, y: f32) -> Self {
        let (w, h) = kind.size();
        Self {
            id,
            kind,
            bounds: Bounds::new(x, y, w, h),
            text: kind.initial_text().into(),
            font_size: if kind == Kind::Heading { 28. } else { 15. },
            bold: kind == Kind::Heading,
            fill: [255, 255, 255],
            checked: false,
            locked: false,
            group: None,
            link: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Page {
    pub id: Id,
    pub name: String,
    pub notes: String,
    pub controls: Vec<Control>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Document {
    pub format_version: u32,
    pub title: String,
    pub next_id: Id,
    pub sketch: bool,
    pub pages: Vec<Page>,
}
impl Default for Document {
    fn default() -> Self {
        Self {
            format_version: FORMAT_VERSION,
            title: "Untitled project".into(),
            next_id: 2,
            sketch: true,
            pages: vec![Page {
                id: 1,
                name: "Wireframe 1".into(),
                notes: String::new(),
                controls: Vec::new(),
            }],
        }
    }
}
impl Document {
    pub fn allocate_id(&mut self) -> Id {
        let id = self.next_id;
        self.next_id += 1;
        id
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.format_version != FORMAT_VERSION {
            return Err(format!(
                "Unsupported document version {}",
                self.format_version
            ));
        }
        if self.pages.is_empty() || self.pages.len() > 1000 {
            return Err("A project must have 1–1000 pages".into());
        }
        if self.title.len() > 10000 {
            return Err("Project title is too long".into());
        }
        let mut ids = HashSet::new();
        let mut groups = HashSet::new();
        let mut count = 0;
        let page_ids: HashSet<_> = self.pages.iter().map(|p| p.id).collect();
        for page in &self.pages {
            if page.id == 0 || !ids.insert(page.id) {
                return Err("Duplicate or zero ID".into());
            }
            if page.name.len() > 10000 || page.notes.len() > 1_000_000 {
                return Err("Page text is too long".into());
            }
            let mut page_groups = HashSet::new();
            for c in &page.controls {
                count += 1;
                if c.id == 0 || !ids.insert(c.id) {
                    return Err("Duplicate or zero ID".into());
                }
                if !c.bounds.valid()
                    || !c.font_size.is_finite()
                    || !(6.0..=120.0).contains(&c.font_size)
                {
                    return Err("Invalid control geometry or font size".into());
                }
                if c.text.len() > 100_000 {
                    return Err("Control text is too long".into());
                }
                if let Some(g) = c.group {
                    if g == 0 {
                        return Err("Invalid group ID".into());
                    }
                    page_groups.insert(g);
                }
                if let Some(link) = c.link
                    && !page_ids.contains(&link)
                {
                    return Err("Link points to a missing page".into());
                }
            }
            for g in page_groups {
                if !groups.insert(g) {
                    return Err("A group spans multiple pages".into());
                }
            }
        }
        if count > 50_000 {
            return Err("Project exceeds 50,000 controls".into());
        }
        if groups.iter().any(|g| ids.contains(g)) {
            return Err("Group and object IDs overlap".into());
        }
        if self.next_id == u64::MAX
            || ids
                .iter()
                .chain(groups.iter())
                .any(|id| *id >= self.next_id)
        {
            return Err("Invalid next ID".into());
        }
        Ok(())
    }
}
