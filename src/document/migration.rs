// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use crate::model as legacy;
use std::collections::HashMap;

pub fn decode(bytes: &[u8]) -> Result<Project, String> {
    if bytes.len() > MAX_FILE_BYTES {
        return Err("Project exceeds the 64 MB file limit".into());
    }
    #[derive(Deserialize)]
    struct Version {
        format_version: u32,
    }
    let version: Version =
        serde_json::from_slice(bytes).map_err(|e| format!("Cannot read project: {e}"))?;
    let project = match version.format_version {
        1 => Project::try_from(
            serde_json::from_slice::<legacy::Document>(bytes).map_err(|e| e.to_string())?,
        )?,
        FORMAT_VERSION => {
            serde_json::from_slice(bytes).map_err(|e| format!("Cannot read project: {e}"))?
        }
        version => return Err(format!("Unsupported document version {version}")),
    };
    project.validate()?;
    Ok(project)
}
impl TryFrom<legacy::Document> for Project {
    type Error = String;
    fn try_from(old: legacy::Document) -> Result<Self, String> {
        old.validate()?;
        let mut pages = vec![];
        for page in old.pages {
            let mut bounds: HashMap<Id, legacy::Bounds> = HashMap::new();
            for c in &page.controls {
                if let Some(g) = c.group {
                    bounds
                        .entry(g)
                        .and_modify(|b| *b = b.union(c.bounds))
                        .or_insert(c.bounds);
                }
            }
            let mut nodes = vec![];
            let mut group_indices = HashMap::new();
            let mut paint_order = vec![];
            for c in page.controls {
                let group = c.group;
                let mut node = migrate_control(c);
                if let Some(g) = group {
                    let b = bounds[&g];
                    let i = *group_indices.entry(g).or_insert_with(|| {
                        let i = nodes.len();
                        nodes.push(Node::group(g, Point::new(b.x, b.y), vec![]));
                        i
                    });
                    node.origin.x -= b.x;
                    node.origin.y -= b.y;
                    paint_order.push(vec![g, node.id]);
                    if let NodeContent::Group { children } = &mut nodes[i].content {
                        children.push(node);
                    }
                } else {
                    paint_order.push(vec![node.id]);
                    nodes.push(node);
                }
            }
            // Store an override only when recursive ordering cannot preserve old layering.
            let mut natural = vec![];
            for node in &nodes {
                match &node.content {
                    NodeContent::Group { children } => {
                        natural.extend(children.iter().map(|c| vec![node.id, c.id]))
                    }
                    _ => natural.push(vec![node.id]),
                }
            }
            if natural == paint_order {
                paint_order.clear();
            }
            pages.push(Page {
                id: page.id,
                name: page.name,
                notes: page.notes,
                parent: None,
                content: Content { nodes, paint_order },
                alternates: vec![],
            });
        }
        let project = Self {
            format_version: FORMAT_VERSION,
            title: old.title,
            next_id: old.next_id,
            settings: Settings {
                sketch: old.sketch,
                font_size: 15.,
            },
            pages,
            definitions: vec![],
            assets: vec![],
            font: None,
        };
        project.validate()?;
        Ok(project)
    }
}
fn migrate_control(c: legacy::Control) -> Node {
    // Stable product identifiers; display names remain a catalog concern.
    let kind = match c.kind {
        legacy::Kind::Button => "button",
        legacy::Kind::Label => "label",
        legacy::Kind::Heading => "heading",
        legacy::Kind::Subtitle => "subtitle",
        legacy::Kind::TextBlock => "text_block",
        legacy::Kind::TextLine => "text_line",
        legacy::Kind::TagCloud => "tag_cloud",

        legacy::Kind::Paragraph => "paragraph",
        legacy::Kind::TextInput => "text_input",
        legacy::Kind::TextArea => "text_area",
        legacy::Kind::Checkbox => "checkbox",
        legacy::Kind::Radio => "radio",
        legacy::Kind::CheckboxGroup => "checkbox_group",
        legacy::Kind::RadioGroup => "radio_group",
        legacy::Kind::Menu => "menu",
        legacy::Kind::MenuBar => "menu_bar",
        legacy::Kind::Tree => "tree",
        legacy::Kind::Accordion => "accordion",
        legacy::Kind::Alert => "alert",
        legacy::Kind::AppBar => "app_bar",
        legacy::Kind::Dropdown => "dropdown",
        legacy::Kind::Search => "search",
        legacy::Kind::Rectangle => "rectangle",
        legacy::Kind::Ellipse => "ellipse",
        legacy::Kind::Line => "line",
        legacy::Kind::Arrow => "arrow",
        legacy::Kind::Image => "image",
        legacy::Kind::Icon => "icon",
        legacy::Kind::IconLabel => "icon_label",
        legacy::Kind::CircleButton => "circle_button",
        legacy::Kind::PointedButton => "pointed_button",
        legacy::Kind::MultilineButton => "multiline_button",
        legacy::Kind::ButtonBar => "button_bar",
        legacy::Kind::Breadcrumbs => "breadcrumbs",
        legacy::Kind::LinkBar => "link_bar",
        legacy::Kind::Browser => "browser",
        legacy::Kind::Phone => "phone",
        legacy::Kind::Tablet => "tablet",
        legacy::Kind::Smartphone => "smartphone",
        legacy::Kind::MobileKeyboard => "mobile_keyboard",
        legacy::Kind::MobileMenu => "mobile_menu",
        legacy::Kind::MobilePicker => "mobile_picker",
        legacy::Kind::Calendar => "calendar",
        legacy::Kind::DateChooser => "date_chooser",
        legacy::Kind::DatePicker => "date_picker",
        legacy::Kind::TimePicker => "time_picker",
        legacy::Kind::HorizontalSlider => "horizontal_slider",
        legacy::Kind::VerticalSlider => "vertical_slider",
        legacy::Kind::HorizontalScrollbar => "horizontal_scrollbar",
        legacy::Kind::VerticalScrollbar => "vertical_scrollbar",
        legacy::Kind::HorizontalSplitter => "horizontal_splitter",
        legacy::Kind::VerticalSplitter => "vertical_splitter",
        legacy::Kind::VolumeSlider => "volume_slider",
        legacy::Kind::Playback => "playback",
        legacy::Kind::Webcam => "webcam",
        legacy::Kind::Video => "video",
        legacy::Kind::CoverFlow => "cover_flow",
        legacy::Kind::ColumnChart => "column_chart",
        legacy::Kind::BarChart => "bar_chart",
        legacy::Kind::LineChart => "line_chart",
        legacy::Kind::PieChart => "pie_chart",
        legacy::Kind::HorizontalBrace => "horizontal_brace",
        legacy::Kind::VerticalBrace => "vertical_brace",
        legacy::Kind::RedCross => "red_cross",
        legacy::Kind::ScratchOut => "scratch_out",

        legacy::Kind::Callout => "callout",
        legacy::Kind::StickyNote => "sticky_note",
        legacy::Kind::Tooltip => "tooltip",

        legacy::Kind::Panel => "panel",
        legacy::Kind::FieldSet => "field_set",
        legacy::Kind::List => "list",
        legacy::Kind::Table => "table",
        legacy::Kind::Tabs => "tabs",
        legacy::Kind::VerticalTabs => "vertical_tabs",
        legacy::Kind::Progress => "progress",
        legacy::Kind::Toggle => "toggle",
        legacy::Kind::Link => "link",
    };
    let mut node = Node::control(
        c.id,
        kind,
        Point::new(c.bounds.x, c.bounds.y),
        Size::fixed(c.bounds.w, c.bounds.h),
    );
    node.locked = c.locked;
    node.properties = BTreeMap::from([
        ("text".into(), Value::Text(c.text)),
        ("font_size".into(), Value::Number(c.font_size as f64)),
        ("bold".into(), Value::Bool(c.bold)),
        (
            "fill".into(),
            Value::Color([c.fill[0], c.fill[1], c.fill[2], 255]),
        ),
        ("checked".into(), Value::Bool(c.checked)),
    ]);
    if let Some(page) = c.link {
        node.properties
            .insert("link".into(), Value::Link(Link::Page(page)));
    }
    node
}
