// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub type Id = u64;
pub const FORMAT_VERSION: u32 = 1;

/// Round a coordinate to the nearest unit, with half ties toward positive infinity.
pub fn round_coordinate(value: f32) -> f32 {
    (f64::from(value) + 0.5).floor() as f32
}

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
        self.resized_with_minimum(handle, delta, aspect, 8.)
    }
    pub fn resized_with_minimum(
        self,
        handle: (i8, i8),
        delta: (f32, f32),
        aspect: bool,
        minimum: f32,
    ) -> Self {
        self.resized_with_minimum_size(handle, delta, aspect, (minimum, minimum))
    }
    pub fn resized_with_minimum_size(
        self,
        handle: (i8, i8),
        delta: (f32, f32),
        aspect: bool,
        minimum: (f32, f32),
    ) -> Self {
        let (hx, hy) = handle;
        let mut w = if hx == 0 {
            self.w
        } else {
            (self.w + delta.0 * hx as f32).clamp(minimum.0, 10_000_f32.max(minimum.0))
        };
        let mut h = if hy == 0 {
            self.h
        } else {
            (self.h + delta.1 * hy as f32).clamp(minimum.1, 10_000_f32.max(minimum.1))
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
            let scale = scale.max(minimum.0 / self.w).max(minimum.1 / self.h);
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
    Subtitle,
    Popover,
    SiteMap,
    FormattingToolbar,
    StreetMap,
    ModalOverlay,
    VerticalRule,
    NumericStepper,
    ColorPicker,
    HelpButton,

    TextBlock,
    TextLine,
    TagCloud,

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
    Shape,
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
    Tablet,
    Smartphone,
    MobileKeyboard,
    MobileMenu,
    MobilePicker,
    Calendar,
    DateChooser,
    DatePicker,
    TimePicker,
    HorizontalSlider,
    VerticalSlider,
    HorizontalScrollbar,
    VerticalScrollbar,
    HorizontalSplitter,
    VerticalSplitter,
    VolumeSlider,
    Playback,
    Webcam,
    Video,
    CoverFlow,
    ColumnChart,
    BarChart,
    LineChart,
    PieChart,
    HorizontalBrace,
    VerticalBrace,
    RedCross,
    ScratchOut,
    Callout,
    StickyNote,
    Tooltip,
    Panel,
    FieldSet,
    List,
    Table,
    Tabs,
    VerticalTabs,
    Progress,
    Toggle,
    Link,
}
impl Kind {
    /// Whether Enter belongs to the control's editable text rather than form submission.
    pub fn multiline_text(self) -> bool {
        matches!(
            self,
            Self::Accordion
                | Self::Alert
                | Self::Browser
                | Self::Callout
                | Self::CheckboxGroup
                | Self::RadioGroup
                | Self::Dropdown
                | Self::HorizontalBrace
                | Self::VerticalBrace
                | Self::Table
                | Self::MobileMenu
                | Self::MobilePicker
                | Self::List
                | Self::Menu
                | Self::Paragraph
                | Self::Popover
                | Self::Rectangle
                | Self::Shape
                | Self::Ellipse
                | Self::SiteMap
                | Self::StickyNote
                | Self::TagCloud
                | Self::TextArea
                | Self::Tooltip
                | Self::Tree
                | Self::VerticalTabs
                | Self::MultilineButton
        )
    }
    /// Stable document/catalog identifier, independent of the display label.
    pub fn catalog_id(self) -> &'static str {
        match self {
            Self::Button => "button",
            Self::Label => "label",
            Self::Heading => "heading",
            Self::Subtitle => "subtitle",
            Self::Popover => "popover",
            Self::SiteMap => "site_map",
            Self::FormattingToolbar => "formatting_toolbar",
            Self::StreetMap => "map",
            Self::ModalOverlay => "modal_overlay",
            Self::VerticalRule => "vertical_rule",
            Self::NumericStepper => "numeric_stepper",
            Self::ColorPicker => "color_picker",
            Self::HelpButton => "help_button",

            Self::TextBlock => "text_block",
            Self::TextLine => "text_line",
            Self::TagCloud => "tag_cloud",

            Self::Paragraph => "paragraph",
            Self::TextInput => "text_input",
            Self::TextArea => "text_area",
            Self::Checkbox => "checkbox",
            Self::Radio => "radio",
            Self::CheckboxGroup => "checkbox_group",
            Self::RadioGroup => "radio_group",
            Self::Menu => "menu",
            Self::MenuBar => "menu_bar",
            Self::Tree => "tree",
            Self::Accordion => "accordion",
            Self::Alert => "alert",
            Self::AppBar => "app_bar",
            Self::Dropdown => "dropdown",
            Self::Search => "search",
            Self::Rectangle => "rectangle",
            Self::Shape => "shape",
            Self::Ellipse => "ellipse",
            Self::Line => "line",
            Self::Arrow => "arrow",
            Self::Image => "image",
            Self::Icon => "icon",
            Self::IconLabel => "icon_label",
            Self::CircleButton => "circle_button",
            Self::PointedButton => "pointed_button",
            Self::MultilineButton => "multiline_button",
            Self::ButtonBar => "button_bar",
            Self::Breadcrumbs => "breadcrumbs",
            Self::LinkBar => "link_bar",
            Self::Browser => "browser",
            Self::Phone => "phone",
            Self::Tablet => "tablet",
            Self::Smartphone => "smartphone",
            Self::MobileKeyboard => "mobile_keyboard",
            Self::MobileMenu => "mobile_menu",
            Self::MobilePicker => "mobile_picker",
            Self::Calendar => "calendar",
            Self::DateChooser => "date_chooser",
            Self::DatePicker => "date_picker",
            Self::TimePicker => "time_picker",
            Self::HorizontalSlider => "horizontal_slider",
            Self::VerticalSlider => "vertical_slider",
            Self::HorizontalScrollbar => "horizontal_scrollbar",
            Self::VerticalScrollbar => "vertical_scrollbar",
            Self::HorizontalSplitter => "horizontal_splitter",
            Self::VerticalSplitter => "vertical_splitter",
            Self::VolumeSlider => "volume_slider",
            Self::Playback => "playback",
            Self::Webcam => "webcam",
            Self::Video => "video",
            Self::CoverFlow => "cover_flow",
            Self::ColumnChart => "column_chart",
            Self::BarChart => "bar_chart",
            Self::LineChart => "line_chart",
            Self::PieChart => "pie_chart",
            Self::HorizontalBrace => "horizontal_brace",
            Self::VerticalBrace => "vertical_brace",
            Self::RedCross => "red_cross",
            Self::ScratchOut => "scratch_out",

            Self::Callout => "callout",
            Self::StickyNote => "sticky_note",
            Self::Tooltip => "tooltip",

            Self::Panel => "panel",
            Self::FieldSet => "field_set",
            Self::List => "list",
            Self::Table => "table",
            Self::Tabs => "tabs",
            Self::VerticalTabs => "vertical_tabs",
            Self::Progress => "progress",
            Self::Toggle => "toggle",
            Self::Link => "link",
        }
    }

    pub const ALL: [Kind; 86] = [
        Self::Button,
        Self::Label,
        Self::Heading,
        Self::Subtitle,
        Self::Popover,
        Self::SiteMap,
        Self::FormattingToolbar,
        Self::StreetMap,
        Self::ModalOverlay,
        Self::VerticalRule,
        Self::NumericStepper,
        Self::ColorPicker,
        Self::HelpButton,
        Self::TextBlock,
        Self::TextLine,
        Self::TagCloud,
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
        Self::Shape,
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
        Self::Tablet,
        Self::Smartphone,
        Self::MobileKeyboard,
        Self::MobileMenu,
        Self::MobilePicker,
        Self::Calendar,
        Self::DateChooser,
        Self::DatePicker,
        Self::TimePicker,
        Self::HorizontalSlider,
        Self::VerticalSlider,
        Self::HorizontalScrollbar,
        Self::VerticalScrollbar,
        Self::HorizontalSplitter,
        Self::VerticalSplitter,
        Self::VolumeSlider,
        Self::Playback,
        Self::Webcam,
        Self::Video,
        Self::CoverFlow,
        Self::ColumnChart,
        Self::BarChart,
        Self::LineChart,
        Self::PieChart,
        Self::HorizontalBrace,
        Self::VerticalBrace,
        Self::RedCross,
        Self::ScratchOut,
        Self::Callout,
        Self::StickyNote,
        Self::Tooltip,
        Self::Panel,
        Self::FieldSet,
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
            Self::Subtitle => "Subtitle",
            Self::Popover => "Popover",
            Self::SiteMap => "Site map",
            Self::FormattingToolbar => "Formatting toolbar",
            Self::StreetMap => "Street map",
            Self::ModalOverlay => "Modal overlay",
            Self::VerticalRule => "Vertical rule",
            Self::NumericStepper => "Numeric stepper",
            Self::ColorPicker => "Color picker",
            Self::HelpButton => "Help button",

            Self::TextBlock => "Text block",
            Self::TextLine => "Text line",
            Self::TagCloud => "Tag cloud",

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
            Self::Shape => "Shape",
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
            Self::Tablet => "Tablet",
            Self::Smartphone => "Smartphone",
            Self::MobileKeyboard => "Mobile keyboard",
            Self::MobileMenu => "Mobile menu",
            Self::MobilePicker => "Mobile picker",
            Self::Calendar => "Calendar",
            Self::DateChooser => "Date chooser",
            Self::DatePicker => "Date picker",
            Self::TimePicker => "Time picker",
            Self::HorizontalSlider => "Horizontal slider",
            Self::VerticalSlider => "Vertical slider",
            Self::HorizontalScrollbar => "Horizontal scrollbar",
            Self::VerticalScrollbar => "Vertical scrollbar",
            Self::HorizontalSplitter => "Horizontal splitter",
            Self::VerticalSplitter => "Vertical splitter",
            Self::VolumeSlider => "Volume slider",
            Self::Playback => "Playback controls",
            Self::Webcam => "Webcam",
            Self::Video => "Video player",
            Self::CoverFlow => "Cover flow",
            Self::ColumnChart => "Column chart",
            Self::BarChart => "Bar chart",
            Self::LineChart => "Line chart",
            Self::PieChart => "Pie chart",
            Self::HorizontalBrace => "Horizontal brace",
            Self::VerticalBrace => "Vertical brace",
            Self::RedCross => "Red cross",
            Self::ScratchOut => "Scratch out",
            Self::Callout => "Callout",
            Self::StickyNote => "Sticky note",
            Self::Tooltip => "Tooltip",
            Self::Panel => "Window",
            Self::FieldSet => "Field set",
            Self::List => "List",
            Self::Table => "Table",
            Self::Tabs => "Tabs",
            Self::VerticalTabs => "Vertical tabs",
            Self::Progress => "Progress bar",
            Self::Toggle => "Toggle",
            Self::Link => "Link",
        }
    }
    /// Display labels for the creation catalog; saved identities and inspector
    /// terminology remain independent.
    pub fn catalog_name(self) -> &'static str {
        match self {
            Self::Label => "Text Label",
            Self::Heading => "Text Title",
            Self::Subtitle => "Text Subtitle",
            Self::SiteMap => "Site Map",
            Self::FormattingToolbar => "Toolbar",
            Self::StreetMap => "Street Map",
            Self::ModalOverlay => "Modal Screen",
            Self::VerticalRule => "V.Rule",
            Self::NumericStepper => "Num. Stepper",
            Self::ColorPicker => "Color Picker",
            Self::HelpButton => "Help Button",
            Self::TextBlock => "Squiggly Block of Text",
            Self::TextLine => "Squiggly Line of Text",
            Self::TagCloud => "Tag Cloud",
            Self::Paragraph => "Text Paragraph",
            Self::TextInput => "Text Input",
            Self::TextArea => "Text Area",
            Self::Radio => "Radio Button",
            Self::CheckboxGroup => "Checkbox Group",
            Self::RadioGroup => "Radio Button Group",
            Self::MenuBar => "Menu Bar",
            Self::Tree => "Tree Pane",
            Self::Alert => "Alert Box",
            Self::AppBar => "App Bar",
            Self::Dropdown => "ComboBox",
            Self::Search => "Search Box",
            Self::Line => "H.Rule",
            Self::IconLabel => "Icon and Text Label",
            Self::CircleButton => "Circle Button",
            Self::PointedButton => "Pointy Button",
            Self::MultilineButton => "Multiline Button",
            Self::ButtonBar => "Button Bar",
            Self::LinkBar => "Link Bar",
            Self::Browser => "Browser Window",
            Self::Phone => "iPhone",
            Self::Tablet => "iPad",
            Self::MobileKeyboard => "iOS Keyboard",
            Self::MobileMenu => "iOS Menu",
            Self::MobilePicker => "iOS Picker",
            Self::DateChooser => "Date Chooser",
            Self::DatePicker => "Date Picker",
            Self::TimePicker => "Time Picker",
            Self::HorizontalSlider => "H.Slider",
            Self::VerticalSlider => "V.Slider",
            Self::HorizontalScrollbar => "H.Scroll Bar",
            Self::VerticalScrollbar => "V.Scroll Bar",
            Self::HorizontalSplitter => "H.Splitter",
            Self::VerticalSplitter => "V.Splitter",
            Self::VolumeSlider => "Volume Slider",
            Self::Playback => "Playback",
            Self::Video => "Video Player",
            Self::CoverFlow => "Cover Flow",
            Self::ColumnChart => "Chart: Column",
            Self::BarChart => "Chart: Bar",
            Self::LineChart => "Chart: Line",
            Self::PieChart => "Chart: Pie",
            Self::HorizontalBrace => "H.Curly Brace",
            Self::VerticalBrace => "V.Curly Brace",
            Self::RedCross => "Red X",
            Self::ScratchOut => "Scratch-Out",
            Self::StickyNote => "Comment",
            Self::FieldSet => "Field Set",
            Self::Table => "Data Grid",
            Self::Tabs => "Tab Bar",
            Self::VerticalTabs => "V.Tabs",
            Self::Progress => "Progress Bar",
            Self::Toggle => "ON/OFF Switch",
            _ => self.name(),
        }
    }
    pub fn category(self) -> &'static str {
        match self {
            Self::FormattingToolbar | Self::VerticalRule => "Layout",
            Self::StreetMap => "Media",
            Self::ModalOverlay | Self::Popover => "Containers",
            Self::SiteMap => "Navigation",
            Self::Callout
            | Self::StickyNote
            | Self::Tooltip
            | Self::HorizontalBrace
            | Self::VerticalBrace
            | Self::RedCross
            | Self::ScratchOut => "Markup",
            Self::ColumnChart | Self::BarChart | Self::LineChart | Self::PieChart => "Charts",
            Self::Label
            | Self::Heading
            | Self::Subtitle
            | Self::TextBlock
            | Self::TextLine
            | Self::TagCloud
            | Self::Paragraph
            | Self::Link
            | Self::Breadcrumbs
            | Self::LinkBar => "Text",
            Self::Icon | Self::IconLabel => "Icons",
            Self::VolumeSlider | Self::Playback | Self::Webcam | Self::Video | Self::CoverFlow => {
                "Media"
            }
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
            | Self::Calendar
            | Self::NumericStepper
            | Self::ColorPicker
            | Self::HelpButton
            | Self::DateChooser
            | Self::HorizontalSlider
            | Self::VerticalSlider
            | Self::Toggle => "Forms",
            Self::Rectangle | Self::Shape | Self::Ellipse | Self::Line | Self::Arrow => "Shapes",
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
            Self::Heading => (300., 44.),
            Self::Subtitle => (140., 28.),
            Self::Popover => (222., 187.),
            Self::SiteMap => (340., 310.),
            Self::FormattingToolbar => (261., 29.),
            Self::StreetMap => (252., 222.),
            Self::ModalOverlay => (318., 240.),
            Self::VerticalRule => (10., 100.),
            Self::NumericStepper => (41., 24.),
            Self::ColorPicker => (26., 28.),
            Self::HelpButton => (18., 18.),

            Self::TextBlock => (275., 80.),
            Self::TextLine => (200., 27.),
            Self::TagCloud => (350., 110.),
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
            Self::Browser => (450., 400.),
            Self::Phone => (240., 440.),
            Self::Tablet => (616., 916.),
            Self::Smartphone => (356., 725.),
            Self::MobileKeyboard => (226., 160.),
            Self::MobileMenu => (215., 381.),
            Self::MobilePicker => crate::mobile_picker::SIZE,
            Self::Calendar => (96., 96.),
            Self::DateChooser => (90., 25.),
            Self::DatePicker | Self::TimePicker => (320., 473.),
            Self::HorizontalSlider => (33., 12.),
            Self::VerticalSlider => (12., 33.),
            Self::HorizontalScrollbar => (100., 16.),
            Self::VerticalScrollbar => (16., 100.),
            Self::HorizontalSplitter => (100., 12.),
            Self::VerticalSplitter => (12., 100.),
            Self::VolumeSlider => (72., 16.),
            Self::Playback => (110., 36.),
            Self::Webcam => (177., 146.),
            Self::Video => (300., 200.),
            Self::CoverFlow => (493., 198.),
            Self::ColumnChart | Self::BarChart | Self::LineChart => (187., 175.),
            Self::PieChart => (180., 180.),
            Self::HorizontalBrace => (200., 80.),
            Self::VerticalBrace => (180., 140.),
            Self::RedCross => (240., 104.),
            Self::ScratchOut => (205., 107.),
            Self::Callout => (39., 39.),
            Self::StickyNote => (109., 123.),
            Self::Tooltip => (87., 33.),
            Self::Panel => (450., 400.),
            Self::FieldSet => (200., 170.),
            Self::List | Self::Table => (300., 180.),
            Self::Tabs => (199., 100.),
            Self::VerticalTabs => (200., 178.),
            Self::Progress => (240., 24.),
            Self::Toggle => (42., 24.),
            Self::Shape => (32., 32.),
            _ => (180., 120.),
        }
    }
    pub fn text_editable(self) -> bool {
        !matches!(
            self,
            Self::Progress
                | Self::Icon
                | Self::Toggle
                | Self::FormattingToolbar
                | Self::StreetMap
                | Self::ModalOverlay
                | Self::VerticalRule
                | Self::ColorPicker
                | Self::HelpButton
                | Self::VolumeSlider
                | Self::Playback
                | Self::Webcam
                | Self::Video
                | Self::CoverFlow
                | Self::ColumnChart
                | Self::BarChart
                | Self::LineChart
                | Self::PieChart
                | Self::RedCross
                | Self::ScratchOut
        )
    }
    pub fn initial_text(self) -> &'static str {
        match self {
            Self::Button | Self::PointedButton => "Button",
            Self::HorizontalBrace | Self::VerticalBrace => crate::markup::DEFAULT_TEXT,
            Self::Callout => "1",
            Self::StickyNote => "A comment",
            Self::Tooltip => "a tooltip",
            Self::IconLabel => "Icon Name",
            Self::MultilineButton => "Multiline Button\nSecond line of text",
            Self::ButtonBar => "One, Two, Three",
            Self::Breadcrumbs => crate::navigation::BREADCRUMBS,
            Self::LinkBar => crate::navigation::LINK_BAR,
            Self::Label => "Some text",
            Self::Heading => "Title",
            Self::Subtitle => "Subtitle",
            Self::Popover => "",
            Self::SiteMap => crate::site_map::DEFAULT,
            Self::FormattingToolbar => "",
            Self::StreetMap => "",
            Self::ModalOverlay => "",
            Self::VerticalRule => "",
            Self::NumericStepper => "3",
            Self::ColorPicker => "",
            Self::HelpButton => "",

            Self::TextBlock | Self::TextLine => "",
            Self::TagCloud => crate::text_controls::CLOUD_DEFAULT,
            Self::Paragraph => "Describe the idea here.\nKeep it simple and make it clear.",
            Self::TextInput => "Enter text…",
            Self::TextArea => "Write something…",
            Self::Checkbox | Self::Radio => "Option",
            Self::CheckboxGroup => crate::choice_group::CHECKBOX,
            Self::RadioGroup => crate::choice_group::RADIO,
            Self::MobileMenu => crate::mobile_menu::DEFAULT,
            Self::MobilePicker => crate::mobile_picker::DEFAULT,
            Self::Calendar
            | Self::DatePicker
            | Self::TimePicker
            | Self::HorizontalSlider
            | Self::VerticalSlider
            | Self::HorizontalScrollbar
            | Self::VerticalScrollbar
            | Self::HorizontalSplitter
            | Self::VerticalSplitter
            | Self::VolumeSlider
            | Self::Playback
            | Self::Webcam
            | Self::Video
            | Self::CoverFlow => "",
            Self::DateChooser => crate::date_controls::CHOOSER_DEFAULT,
            Self::Menu => crate::menu::MENU,
            Self::MenuBar => crate::menu::BAR,
            Self::Tree => crate::tree::DEFAULT,
            Self::Accordion => crate::accordion::DEFAULT,
            Self::Alert => crate::alert::DEFAULT,
            Self::AppBar => crate::app_bar::DEFAULT,
            Self::Dropdown => "Choose an option",
            Self::Search => "Search…",
            Self::Browser => crate::browser::DEFAULT,
            Self::Panel => crate::window::DEFAULT,
            Self::FieldSet => crate::field_set::DEFAULT,
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
            font_size: match kind {
                Kind::Heading => 40.,
                Kind::Subtitle => 24.,
                _ => 15.,
            },
            bold: false,
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
