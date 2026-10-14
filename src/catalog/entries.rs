// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;

pub(super) fn build() -> Vec<ControlSpec> {
    use AutoSize as A;
    use Family as F;
    use ResizeAxes as R;
    let mut out = vec![];
    macro_rules! add {
        ($id:literal,$name:literal,$category:literal,$family:ident,$axes:ident,$auto:ident,$size:expr,$aliases:expr) => {
            out.push(configure(ControlSpec {
                id: $id,
                name: $name,
                category: $category,
                aliases: $aliases,
                family: F::$family,
                resize: R::$axes,
                autosize: A::$auto,
                default_size: $size,
                properties: vec![],
            }));
        };
    }
    add!(
        "accordion",
        "Accordion",
        "Containers",
        Rows,
        Both,
        Linked,
        auto(),
        &[]
    );
    add!(
        "alert",
        "Alert dialog",
        "Containers",
        Container,
        Both,
        Linked,
        auto(),
        &["dialog", "message box"]
    );
    add!(
        "arrow",
        "Arrow",
        "Shapes",
        Shape,
        Both,
        None,
        fixed(150., 100.),
        &["connector"]
    );
    add!(
        "app_bar",
        "App bar",
        "Navigation",
        Rows,
        Both,
        Linked,
        auto(),
        &[]
    );
    add!(
        "column_chart",
        "Column chart",
        "Charts",
        Chart,
        Both,
        None,
        fixed(187., 175.),
        &[]
    );
    add!(
        "bar_chart",
        "Bar chart",
        "Charts",
        Chart,
        Both,
        None,
        fixed(187., 175.),
        &[]
    );
    add!(
        "text_block",
        "Text block sketch",
        "Text",
        Decoration,
        Both,
        None,
        fixed(275., 80.),
        &["squiggly text"]
    );
    add!(
        "breadcrumbs",
        "Breadcrumbs",
        "Navigation",
        Rows,
        Computed,
        Intrinsic,
        auto(),
        &["path"]
    );
    add!(
        "browser",
        "Browser window",
        "Containers",
        Container,
        Both,
        None,
        fixed(450., 400.),
        &["browser"]
    );
    add!(
        "button_bar",
        "Button bar",
        "Buttons",
        Rows,
        Both,
        Independent,
        auto(),
        &["segmented control"]
    );
    add!(
        "button",
        "Button",
        "Buttons",
        Button,
        Both,
        Linked,
        auto(),
        &[]
    );
    add!(
        "pointed_button",
        "Pointed button",
        "Buttons",
        Button,
        Horizontal,
        Linked,
        auto(),
        &["back button"]
    );
    add!(
        "multiline_button",
        "Multiline button",
        "Buttons",
        Button,
        Both,
        Linked,
        auto(),
        &[]
    );
    add!(
        "calendar",
        "Calendar",
        "Forms",
        Input,
        Both,
        None,
        auto(),
        &[]
    );
    add!(
        "callout",
        "Callout",
        "Markup",
        Decoration,
        Both,
        Linked,
        auto(),
        &["annotation"]
    );
    add!(
        "rectangle",
        "Rectangle",
        "Shapes",
        Shape,
        Both,
        None,
        fixed(100., 70.),
        &["box"]
    );
    add!(
        "radio",
        "Radio button",
        "Forms",
        Choice,
        Both,
        Linked,
        auto(),
        &["radio"]
    );
    add!(
        "checkbox",
        "Checkbox",
        "Forms",
        Choice,
        Both,
        Linked,
        auto(),
        &["check box"]
    );
    add!(
        "radio_group",
        "Radio group",
        "Forms",
        Rows,
        Both,
        Linked,
        auto(),
        &[]
    );
    add!(
        "checkbox_group",
        "Checkbox group",
        "Forms",
        Rows,
        Both,
        Linked,
        auto(),
        &[]
    );
    add!(
        "circle_button",
        "Circle button",
        "Buttons",
        Button,
        Both,
        Intrinsic,
        auto(),
        &["round icon button"]
    );
    add!(
        "color_picker",
        "Color picker",
        "Forms",
        Input,
        Computed,
        Intrinsic,
        auto(),
        &["colour picker"]
    );
    add!(
        "dropdown",
        "Dropdown",
        "Forms",
        Input,
        Horizontal,
        Linked,
        auto(),
        &["combo box", "combobox", "select"]
    );
    add!(
        "cover_flow",
        "Cover flow",
        "Media",
        Media,
        Both,
        None,
        fixed(493., 198.),
        &["carousel"]
    );
    add!(
        "vertical_brace",
        "Vertical brace",
        "Markup",
        Decoration,
        Both,
        None,
        auto(),
        &[]
    );
    add!(
        "horizontal_brace",
        "Horizontal brace",
        "Markup",
        Decoration,
        Both,
        None,
        auto(),
        &[]
    );
    add!(
        "table",
        "Data grid",
        "Forms",
        Rows,
        Both,
        Linked,
        auto(),
        &["table", "spreadsheet"]
    );
    add!(
        "date_chooser",
        "Date chooser",
        "Forms",
        Input,
        Horizontal,
        Width,
        auto(),
        &["date input"]
    );
    add!(
        "date_picker",
        "Date picker",
        "Mobile",
        Input,
        Both,
        None,
        fixed(320., 473.),
        &[]
    );
    add!(
        "field_set",
        "Field set",
        "Containers",
        Container,
        Both,
        None,
        auto(),
        &["fieldset"]
    );
    add!(
        "formatting_toolbar",
        "Formatting toolbar",
        "Navigation",
        Rows,
        Horizontal,
        None,
        fixed(261., 29.),
        &["toolbar"]
    );
    add!(
        "help_button",
        "Help button",
        "Buttons",
        Button,
        Computed,
        Intrinsic,
        auto(),
        &[]
    );
    add!(
        "icon",
        "Icon",
        "Icons",
        Image,
        Computed,
        Intrinsic,
        auto(),
        &[]
    );
    add!(
        "icon_label",
        "Icon and label",
        "Icons",
        Text,
        Horizontal,
        Width,
        auto(),
        &["icon text"]
    );
    add!(
        "image",
        "Image",
        "Media",
        Image,
        Both,
        Independent,
        auto(),
        &["picture", "photo"]
    );
    add!(
        "tablet",
        "Tablet",
        "Mobile",
        Device,
        Both,
        Linked,
        auto(),
        &[]
    );
    add!(
        "phone",
        "Phone",
        "Mobile",
        Device,
        Both,
        Linked,
        auto(),
        &[]
    );
    add!(
        "smartphone",
        "Smartphone",
        "Mobile",
        Device,
        Both,
        Linked,
        auto(),
        &["android"]
    );
    add!(
        "mobile_keyboard",
        "Mobile keyboard",
        "Mobile",
        Device,
        Both,
        Linked,
        auto(),
        &["keyboard"]
    );
    add!(
        "mobile_menu",
        "Mobile menu",
        "Mobile",
        Rows,
        Horizontal,
        Independent,
        auto(),
        &[]
    );
    add!(
        "mobile_picker",
        "Mobile picker",
        "Mobile",
        Rows,
        Computed,
        Intrinsic,
        auto(),
        &[]
    );
    add!(
        "label",
        "Text label",
        "Text",
        Text,
        Horizontal,
        Width,
        auto(),
        &["label", "text"]
    );
    add!(
        "line_chart",
        "Line chart",
        "Charts",
        Chart,
        Both,
        None,
        fixed(187., 175.),
        &[]
    );
    add!(
        "text_line",
        "Text line sketch",
        "Text",
        Decoration,
        Both,
        None,
        fixed(200., 27.),
        &[]
    );
    add!(
        "link",
        "Link",
        "Text",
        Text,
        Computed,
        Intrinsic,
        auto(),
        &["hyperlink"]
    );
    add!(
        "link_bar",
        "Link bar",
        "Navigation",
        Rows,
        Computed,
        Intrinsic,
        auto(),
        &[]
    );
    add!("list", "List", "Forms", Rows, Both, Linked, auto(), &[]);
    add!(
        "menu",
        "Menu",
        "Navigation",
        Rows,
        Horizontal,
        None,
        auto(),
        &["context menu"]
    );
    add!(
        "menu_bar",
        "Menu bar",
        "Navigation",
        Rows,
        Both,
        Linked,
        auto(),
        &[]
    );
    add!(
        "modal_overlay",
        "Modal overlay",
        "Containers",
        Shape,
        Both,
        None,
        fixed(318., 240.),
        &["modal screen"]
    );
    add!(
        "numeric_stepper",
        "Numeric stepper",
        "Forms",
        Input,
        Horizontal,
        Width,
        auto(),
        &["number input", "spinner"]
    );
    add!(
        "paragraph",
        "Paragraph",
        "Text",
        Text,
        Both,
        None,
        fixed(275., 80.),
        &["text paragraph"]
    );
    add!(
        "pie_chart",
        "Pie chart",
        "Charts",
        Chart,
        Both,
        None,
        fixed(180., 180.),
        &[]
    );
    add!(
        "playback",
        "Playback controls",
        "Media",
        Media,
        Computed,
        Intrinsic,
        auto(),
        &[]
    );
    add!(
        "popover",
        "Popover",
        "Containers",
        Container,
        Both,
        Linked,
        auto(),
        &[]
    );
    add!(
        "progress",
        "Progress bar",
        "Forms",
        Input,
        Both,
        None,
        auto(),
        &["progress"]
    );
    add!(
        "red_cross",
        "Red cross",
        "Markup",
        Decoration,
        Both,
        None,
        fixed(240., 104.),
        &["red x"]
    );
    add!(
        "horizontal_rule",
        "Horizontal rule",
        "Layout",
        Shape,
        Horizontal,
        Height,
        Size {
            width: Dimension::Fixed(100.),
            height: Dimension::Auto
        },
        &["line", "divider"]
    );
    add!(
        "vertical_rule",
        "Vertical rule",
        "Layout",
        Shape,
        Vertical,
        Width,
        Size {
            width: Dimension::Auto,
            height: Dimension::Fixed(100.)
        },
        &[]
    );
    add!(
        "scratch_out",
        "Scratch out",
        "Markup",
        Decoration,
        Both,
        None,
        fixed(205., 107.),
        &[]
    );
    add!(
        "horizontal_scrollbar",
        "Horizontal scrollbar",
        "Layout",
        Input,
        Horizontal,
        None,
        auto(),
        &[]
    );
    add!(
        "vertical_scrollbar",
        "Vertical scrollbar",
        "Layout",
        Input,
        Vertical,
        None,
        auto(),
        &[]
    );
    add!(
        "search",
        "Search box",
        "Forms",
        Input,
        Horizontal,
        Linked,
        auto(),
        &["search"]
    );
    add!(
        "shape",
        "Shape",
        "Shapes",
        Shape,
        Both,
        None,
        fixed(32., 32.),
        &["ellipse", "circle", "diamond", "star", "triangle"]
    );
    add!(
        "site_map",
        "Site map",
        "Navigation",
        Rows,
        Computed,
        Intrinsic,
        auto(),
        &["sitemap"]
    );
    add!(
        "vertical_slider",
        "Vertical slider",
        "Forms",
        Input,
        Vertical,
        None,
        fixed(12., 33.),
        &[]
    );
    add!(
        "horizontal_slider",
        "Horizontal slider",
        "Forms",
        Input,
        Horizontal,
        None,
        fixed(33., 12.),
        &["slider"]
    );
    add!(
        "horizontal_splitter",
        "Horizontal splitter",
        "Layout",
        Shape,
        Horizontal,
        None,
        fixed(100., 12.),
        &[]
    );
    add!(
        "vertical_splitter",
        "Vertical splitter",
        "Layout",
        Shape,
        Vertical,
        None,
        fixed(12., 100.),
        &[]
    );
    add!(
        "sticky_note",
        "Sticky note",
        "Markup",
        Text,
        Both,
        None,
        auto(),
        &["comment", "note"]
    );
    add!(
        "map",
        "Street map",
        "Media",
        Media,
        Both,
        None,
        fixed(252., 222.),
        &["map"]
    );
    add!(
        "toggle",
        "On/off switch",
        "Forms",
        Choice,
        Computed,
        Intrinsic,
        auto(),
        &["toggle", "switch"]
    );
    add!(
        "tabs",
        "Tab bar",
        "Navigation",
        Rows,
        Both,
        None,
        auto(),
        &["tabs"]
    );
    add!(
        "vertical_tabs",
        "Vertical tabs",
        "Navigation",
        Rows,
        Both,
        None,
        auto(),
        &[]
    );
    add!(
        "tag_cloud",
        "Tag cloud",
        "Text",
        Text,
        Both,
        None,
        fixed(350., 110.),
        &[]
    );
    add!(
        "text_area",
        "Text area",
        "Forms",
        Input,
        Both,
        None,
        fixed(200., 140.),
        &["textarea"]
    );
    add!(
        "text_input",
        "Text input",
        "Forms",
        Input,
        Horizontal,
        Width,
        auto(),
        &["textbox", "input"]
    );
    add!(
        "heading",
        "Text title",
        "Text",
        Text,
        Horizontal,
        Width,
        auto(),
        &["heading", "title"]
    );
    add!(
        "subtitle",
        "Text subtitle",
        "Text",
        Text,
        Horizontal,
        Width,
        auto(),
        &["subtitle"]
    );
    add!(
        "tooltip",
        "Tooltip",
        "Markup",
        Container,
        Both,
        None,
        auto(),
        &[]
    );
    add!(
        "tree",
        "Tree pane",
        "Forms",
        Rows,
        Both,
        Linked,
        fixed(300., 285.),
        &["tree"]
    );
    add!(
        "video",
        "Video player",
        "Media",
        Media,
        Both,
        None,
        fixed(300., 200.),
        &["video"]
    );
    add!(
        "volume_slider",
        "Volume slider",
        "Media",
        Input,
        Horizontal,
        None,
        fixed(72., 16.),
        &["volume"]
    );
    add!(
        "webcam",
        "Webcam",
        "Media",
        Media,
        Both,
        None,
        fixed(177., 146.),
        &["camera"]
    );
    add!(
        "window",
        "Window",
        "Containers",
        Container,
        Both,
        None,
        fixed(450., 400.),
        &["panel"]
    );
    add!(
        "time_picker",
        "Time picker",
        "Mobile",
        Input,
        Both,
        None,
        fixed(320., 473.),
        &[]
    );
    out
}
fn configure(mut c: ControlSpec) -> ControlSpec {
    let mut font = number("font_size", "Font size", 13., 6., 120., false);
    font.default = if c.id == "app_bar" {
        DefaultValue::Literal(Value::Number(16.))
    } else {
        DefaultValue::FontSize
    };
    let mut fill = prop("fill", "Fill", PropertyType::Color, Value::Color([255; 4]));
    fill.default = if c.id == "app_bar" {
        DefaultValue::Literal(Value::Color([221, 221, 221, 255]))
    } else if c.id == "button" {
        DefaultValue::ButtonFill
    } else if c.id == "circle_button" {
        DefaultValue::Literal(Value::Color([204, 204, 204, 255]))
    } else {
        DefaultValue::Fill
    };
    let mut ink = prop(
        "text_color",
        "Text color",
        PropertyType::Color,
        Value::Color([0, 0, 0, 255]),
    );
    ink.default = if c.id == "link" {
        DefaultValue::LinkColor
    } else if matches!(
        c.id,
        "dropdown"
            | "icon"
            | "icon_label"
            | "checkbox"
            | "radio"
            | "checkbox_group"
            | "radio_group"
    ) {
        DefaultValue::FormInk
    } else if c.id == "search" {
        DefaultValue::SearchInk
    } else {
        DefaultValue::Ink
    };
    let text = match c.id {
        "button" | "pointed_button" => "Button",
        "button_bar" => "One, Two, Three",
        "breadcrumbs" => crate::navigation::BREADCRUMBS,
        "link_bar" => crate::navigation::LINK_BAR,
        "tabs" => crate::tabs::HORIZONTAL,
        "vertical_tabs" => crate::tabs::VERTICAL,
        "icon_label" => "Icon Name",
        "multiline_button" => "Multiline Button\nSecond line of text",
        "dropdown" => "Dropdown",
        "checkbox" => "Checkbox",
        "radio" => "Radio button",
        "checkbox_group" => crate::choice_group::CHECKBOX,
        "radio_group" => crate::choice_group::RADIO,
        "mobile_menu" => crate::mobile_menu::DEFAULT,
        "mobile_picker" => crate::mobile_picker::DEFAULT,
        "date_chooser" => crate::date_controls::CHOOSER_DEFAULT,
        "menu" => crate::menu::MENU,
        "menu_bar" => crate::menu::BAR,
        "tree" => crate::tree::DEFAULT,
        "accordion" => crate::accordion::DEFAULT,
        "alert" => crate::alert::DEFAULT,
        "app_bar" => crate::app_bar::DEFAULT,
        "browser" => crate::browser::DEFAULT,
        "window" => crate::window::DEFAULT,
        "field_set" => crate::field_set::DEFAULT,
        "label" => "Some text",
        "heading" => "Title",
        "subtitle" => "Subtitle",
        "paragraph" => "Write your ideas here.",
        "text_input" => "",
        "search" => "search",
        "link" => "Link",
        "table" => "Name, Status\nItem one, Ready\nItem two, Draft",
        "list" => "Item One\nItem Two\nItem Three",
        _ => "",
    };
    c.properties = vec![
        prop("text", "Text", PropertyType::Text, Value::Text(text.into())),
        font,
        boolean("bold", "Bold", false),
        boolean("italic", "Italic", false),
        boolean("underline", "Underline", c.id == "link_bar"),
        fill,
        ink,
    ];
    // Preserved for old files. New choice editing uses the richer state property.
    let mut checked = boolean("checked", "Checked", false);
    checked.visible = false;
    c.properties.push(checked);
    let mut link = prop("link", "Link", PropertyType::Link, Value::Null);
    link.nullable = true;
    c.properties.push(link);
    c.properties.push(choice(
        "align",
        "Alignment",
        &["left", "center", "right"],
        if matches!(c.family, Family::Button) || c.id == "alert" {
            "center"
        } else {
            "left"
        },
    ));
    let states: &'static [&'static str] = match c.id {
        "button" | "circle_button" => &["normal", "selected", "focused", "disabled"],
        "checkbox" | "radio" => &[
            "normal",
            "selected",
            "disabled",
            "disabled_selected",
            "indeterminate",
            "disabled_indeterminate",
        ],
        "dropdown" => &[
            "normal",
            "focused",
            "disabled",
            "closed",
            "disabled_closed",
            "focused_closed",
        ],
        "date_chooser" | "search" | "text_area" | "text_input" => {
            &["normal", "focused", "disabled"]
        }
        "pointed_button" | "icon_label" | "label" | "link" | "numeric_stepper" | "paragraph"
        | "vertical_slider" | "horizontal_slider" | "tree" => &["normal", "disabled"],
        "toggle" => &["off", "on"],
        _ => &[],
    };
    if let Some(first) = states.first() {
        c.properties.push(choice("state", "State", states, first));
    }
    if matches!(
        c.id,
        "dropdown"
            | "text_area"
            | "list"
            | "tabs"
            | "vertical_tabs"
            | "tree"
            | "accordion"
            | "browser"
            | "window"
    ) {
        c.properties.extend([
            boolean("vertical_scrollbar", "Scrollbar", false),
            number("value", "Scroll position", 0., 0., 1., false),
        ]);
    }
    if matches!(c.id, "text_input" | "text_area") {
        c.properties.push(number(
            "background_opacity",
            "Fill opacity",
            1.,
            0.,
            1.,
            false,
        ));
        let mut border = prop(
            "border_color",
            "Border color",
            PropertyType::Color,
            Value::Color([0, 0, 0, 255]),
        );
        border.default = DefaultValue::FormInk;
        c.properties.push(border);
        if let Some(text) = c.properties.iter_mut().find(|p| p.key == "text_color") {
            text.default = DefaultValue::FormInk;
        }
    }
    if matches!(c.id, "horizontal_slider" | "vertical_slider") {
        c.properties.push(choice(
            "slider_thumb",
            "Shape",
            &["pointy", "rectangle", "circle"],
            "pointy",
        ));
        let fill = c.properties.iter_mut().find(|p| p.key == "fill").unwrap();
        fill.default = DefaultValue::SliderInk;
        fill.label = "Color";
    }
    if c.id == "date_picker" {
        c.properties
            .push(choice("date_mode", "Kind", &["month", "year"], "month"));
    }
    if c.id == "date_chooser" {
        let mut border = prop(
            "border_color",
            "Border color",
            PropertyType::Color,
            Value::Color([0, 0, 0, 255]),
        );
        border.default = DefaultValue::FormInk;
        c.properties.push(border);
    }
    if c.id == "text_input" {
        c.properties.push(choice(
            "border",
            "Border",
            &["rectangle", "line"],
            "rectangle",
        ));
    }
    if c.id == "dropdown" {
        c.properties
            .push(number("selected_index", "Selection", 0., 0., 10000., true));
        let mut border = prop(
            "border_color",
            "Border color",
            PropertyType::Color,
            Value::Color([0, 0, 0, 255]),
        );
        border.default = DefaultValue::FormInk;
        c.properties.push(border);
    }
    if matches!(c.family, Family::Rows) && c.id != "app_bar" {
        c.properties.push(number(
            "selected_index",
            "Selection",
            if matches!(c.id, "button_bar" | "accordion") {
                0.
            } else {
                -1.
            },
            -1.,
            10000.,
            true,
        ));
    }
    if matches!(c.family, Family::Container | Family::Shape | Family::Image) {
        c.properties.push(choice(
            "border",
            "Border",
            &["none", "solid", "dashed", "dotted"],
            "solid",
        ));
    }
    if c.id == "browser"
        && let Some(border) = c.properties.iter_mut().find(|p| p.key == "border")
    {
        *border = choice(
            "border",
            "Border",
            &["solid", "torn_bottom", "none", "dashed", "dotted"],
            "solid",
        );
    }
    if c.id == "mobile_keyboard" {
        c.properties.extend([
            choice("keyboard_device", "Device", &["phone", "tablet"], "phone"),
            choice(
                "orientation",
                "Orientation",
                &["portrait", "landscape"],
                "portrait",
            ),
        ]);
    }
    if matches!(c.id, "tablet" | "smartphone") {
        c.properties.extend([
            choice(
                "orientation",
                "Orientation",
                &["portrait", "landscape"],
                "portrait",
            ),
            choice("device_resizing", "Resizing", &["stretch", "fit"], "fit"),
            boolean("transparent_background", "Transparent screen", false),
            boolean("status_bar", "Show status bar", true),
        ]);
    }
    if c.id == "phone" {
        c.properties.extend([
            choice(
                "device_model",
                "Model",
                &["classic", "tall", "wide", "notched"],
                "classic",
            ),
            choice(
                "orientation",
                "Orientation",
                &["portrait", "landscape"],
                "portrait",
            ),
            choice("device_resizing", "Resizing", &["stretch", "fit"], "fit"),
            choice(
                "background_pattern",
                "Background",
                &[
                    "white",
                    "header",
                    "header_footer",
                    "header_dark_footer",
                    "dark",
                ],
                "white",
            ),
            boolean("transparent_background", "Transparent screen", false),
            boolean("status_bar", "Show status bar", true),
        ]);
    }
    if c.id == "field_set" {
        c.properties.push(number(
            "background_opacity",
            "Fill opacity",
            1.,
            0.,
            1.,
            false,
        ));
    }
    if c.id == "window" {
        c.properties.extend([
            number("header_height", "Header height", 26., 0., 10000., true),
            number("footer_height", "Footer height", 25., 0., 10000., true),
            boolean("minimize", "Minimize button", false),
            boolean("maximize", "Maximize button", false),
            boolean("close", "Close button", false),
            boolean("resize_handle", "Resize handle", false),
        ]);
    }
    if c.id == "app_bar" {
        c.properties.extend([
            boolean("nav_icon", "Navigation icon", true),
            boolean("filter_icon", "Filter icon", true),
            boolean("menu_icon", "Menu icon", true),
            prop(
                "icons",
                "Custom icons",
                PropertyType::Icons,
                Value::Icons(vec![]),
            ),
        ]);
    }
    if c.id == "alert" {
        c.properties.push(choice(
            "button_style",
            "Button style",
            &["divided", "inline"],
            "divided",
        ));
        if let Some(border) = c.properties.iter_mut().find(|p| p.key == "border") {
            border.kind = PropertyType::Choice(&["none", "solid"]);
        }
        if let Some(ink) = c.properties.iter_mut().find(|p| p.key == "text_color") {
            ink.default = DefaultValue::FormInk;
            ink.label = "Button color";
        }
    }
    if matches!(c.id, "image" | "icon") {
        let mut asset = prop("image", "Image", PropertyType::Asset, Value::Null);
        asset.nullable = true;
        c.properties.push(asset);
        c.properties
            .push(number("rotation", "Rotation", 0., 0., 359., true));
    }
    if c.id == "image" {
        c.properties.push(boolean("grayscale", "Grayscale", false));
        c.properties
            .push(boolean("sketch_filter", "Sketch filter", false));
        c.properties
            .push(boolean("image_border", "Show border", false));
    }
    if c.id == "search" {
        c.properties.extend([
            choice("shape", "Shape", &["rounded", "rectangle"], "rounded"),
            boolean("search_icon", "Search icon", true),
            boolean("microphone_icon", "Microphone icon", false),
        ]);
    }
    if c.id == "shape" {
        c.properties.push(choice(
            "shape",
            "Shape",
            &[
                "ellipse",
                "rectangle",
                "rounded_rectangle",
                "dotted_rectangle",
                "diamond",
                "star",
                "parallelogram",
                "triangle",
                "torn_bottom",
                "torn_top",
            ],
            "ellipse",
        ));
        c.properties.push(number(
            "shape_rotation",
            "Shape rotation",
            0.,
            0.,
            359.,
            true,
        ));
    }
    if matches!(
        c.id,
        "button"
            | "dropdown"
            | "icon"
            | "icon_label"
            | "circle_button"
            | "pointed_button"
            | "multiline_button"
            | "menu_bar"
    ) {
        let mut icon = prop("icon", "Icon", PropertyType::Icon, Value::Null);
        icon.nullable = true;
        c.properties.extend([
            icon,
            choice(
                "icon_size",
                "Icon size",
                &["16", "24", "32", "48", "64", "128"],
                if c.id == "icon_label" { "48" } else { "24" },
            ),
            choice(
                "icon_rotation",
                "Icon rotation",
                &["0", "90", "180", "270"],
                "0",
            ),
        ]);
        if matches!(c.id, "button" | "pointed_button") {
            c.properties.push(boolean("menu_icon", "Menu arrow", false));
        }
    }
    if matches!(c.id, "multiline_button" | "tree") {
        c.properties.push(number(
            "background_opacity",
            "Fill opacity",
            1.,
            0.,
            1.,
            false,
        ));
    }
    if matches!(c.id, "menu_bar" | "tree" | "mobile_menu") {
        c.properties.retain(|p| p.key != "border");
        c.properties
            .push(choice("border", "Border", &["solid", "none"], "solid"));
    }
    if c.id == "pointed_button" {
        c.properties.push(choice(
            "direction",
            "Direction",
            &["left", "center", "right"],
            "right",
        ));
        c.properties
            .push(choice("border", "Border", &["none", "solid"], "solid"));
    }
    if matches!(c.id, "icon" | "icon_label" | "circle_button") {
        c.properties.retain(|p| p.key != "border");
        c.properties.push(choice(
            "border",
            "Border",
            &["none", "solid"],
            if c.id == "circle_button" {
                "solid"
            } else {
                "none"
            },
        ));
        if matches!(c.id, "icon_label" | "circle_button") {
            c.properties.push(choice(
                "label_position",
                "Label position",
                &["bottom", "left", "right"],
                "bottom",
            ));
        }
    }
    if matches!(
        c.id,
        "button_bar"
            | "breadcrumbs"
            | "link_bar"
            | "tabs"
            | "vertical_tabs"
            | "checkbox_group"
            | "radio_group"
            | "menu"
            | "menu_bar"
            | "mobile_menu"
            | "tree"
            | "accordion"
            | "alert"
            | "date_picker"
            | "time_picker"
            | "app_bar"
    ) {
        c.properties.push(prop(
            "row_links",
            if c.id == "app_bar" {
                "Icon links"
            } else if matches!(c.id, "alert" | "date_picker" | "time_picker") {
                "Button links"
            } else if c.id == "button_bar" {
                "Segment links"
            } else if matches!(
                c.id,
                "checkbox_group" | "radio_group" | "menu" | "mobile_menu" | "tree" | "accordion"
            ) {
                "Row links"
            } else {
                "Item links"
            },
            PropertyType::Links,
            Value::Links(vec![]),
        ));
    }
    if matches!(c.id, "tabs" | "vertical_tabs") {
        c.properties.push(choice(
            "tab_position",
            "Tabs position",
            if c.id == "tabs" {
                &["top", "bottom"]
            } else {
                &["left", "right"]
            },
            if c.id == "tabs" { "top" } else { "left" },
        ));
        c.properties
            .push(choice("border", "Border", &["solid", "none"], "solid"));
        c.properties.push(number(
            "background_opacity",
            "Fill opacity",
            1.,
            0.,
            1.,
            false,
        ));
        if let Some(p) = c.properties.iter_mut().find(|p| p.key == "text_color") {
            p.default = DefaultValue::FormInk;
        }
    }
    if c.id == "link_bar" {
        c.properties.push(prop(
            "border_color",
            "Separator and selected text",
            PropertyType::Color,
            Value::Color([0, 0, 0, 255]),
        ));
        if let Some(p) = c.properties.iter_mut().find(|p| p.key == "text_color") {
            p.default = DefaultValue::Literal(Value::Color([8, 83, 148, 255]));
        }
    }
    if c.id == "list" {
        if let Some(text) = c.properties.iter_mut().find(|p| p.key == "text_color") {
            text.default = DefaultValue::FormInk;
        }
        c.properties.push(prop(
            "row_icons",
            "Row icons",
            PropertyType::Icons,
            Value::Icons(vec![]),
        ));
        c.properties.push(prop(
            "row_links",
            "Row links",
            PropertyType::Links,
            Value::Links(vec![]),
        ));
        c.properties.extend([
            boolean("header", "Header row", false),
            number("row_height", "Row height", 21., 1., 1000., true),
            prop(
                "alternate_fill",
                "Alternate row",
                PropertyType::Color,
                Value::Color([255; 4]),
            ),
            choice("border", "Border", &["solid", "none"], "solid"),
        ]);
    }
    if c.id == "table" {
        c.properties.extend([
            boolean("header", "Header row", true),
            boolean("vertical_lines", "Vertical lines", true),
            boolean("horizontal_lines", "Horizontal lines", false),
            number("row_height", "Row height", 30., 16., 80., true),
            prop(
                "alternate_fill",
                "Alternate row",
                PropertyType::Color,
                Value::Color([238, 238, 238, 255]),
            ),
        ]);
    }
    if matches!(
        c.id,
        "progress"
            | "horizontal_slider"
            | "vertical_slider"
            | "horizontal_scrollbar"
            | "vertical_scrollbar"
            | "volume_slider"
    ) {
        c.properties.push(number(
            "value",
            "Value",
            if matches!(c.id, "horizontal_slider" | "vertical_slider") {
                0.05
            } else {
                0.5
            },
            0.,
            1.,
            false,
        ));
    }
    if c.id == "heading" {
        let p = c
            .properties
            .iter_mut()
            .find(|p| p.key == "font_size")
            .unwrap();
        p.default = DefaultValue::Literal(Value::Number(28.));
    }
    if c.id == "subtitle" {
        let p = c
            .properties
            .iter_mut()
            .find(|p| p.key == "font_size")
            .unwrap();
        p.default = DefaultValue::Literal(Value::Number(20.));
    }
    for property in &mut c.properties {
        property.visible = super::fields::visible(c.id, property.key);
    }
    c
}
