// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Layout independent of editor widgets. No measured geometry is written to the file.
use crate::{
    catalog::{self, Context},
    document::{
        Content, Dimension, Node, NodeContent, NodePath, Point, Project, Properties, Value,
    },
    rich_text::RichText,
};
use eframe::egui;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}
impl Rect {
    pub fn union(self, b: Self) -> Self {
        let x = self.x.min(b.x);
        let y = self.y.min(b.y);
        Self {
            x,
            y,
            width: (self.x + self.width).max(b.x + b.width) - x,
            height: (self.y + self.height).max(b.y + b.height) - y,
        }
    }
    pub fn intersect(self, b: Self) -> Self {
        let x = self.x.max(b.x);
        let y = self.y.max(b.y);
        Self {
            x,
            y,
            width: ((self.x + self.width).min(b.x + b.width) - x).max(0.),
            height: ((self.y + self.height).min(b.y + b.height) - y).max(0.),
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub struct TextStyle {
    pub size: f32,
    pub bold: bool,
    pub italic: bool,
}
#[derive(Clone, Copy)]
pub struct TextPlacement {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
    pub middle: bool,
    pub wrap: bool,
}
pub fn text_placement(kind: &str) -> TextPlacement {
    let (left, right, top, bottom, middle, wrap) = match kind {
        "paragraph" => (0., 0., 0., 0., false, true),
        "text_area" | "sticky_note" => (8., 8., 8., 8., false, true),
        "button" | "pointed_button" => (8., 8., 5., 5., true, false),
        "multiline_button" => (8., 8., 5., 5., true, true),
        "checkbox" | "radio" => (20., 0., 6., 4., true, false),
        "search" => (30., 24., 4., 4., true, false),
        "text_input" | "dropdown" | "numeric_stepper" | "date_chooser" => {
            (10., 24., 4., 4., true, false)
        }
        _ => (0., 0., 0., 0., false, false),
    };
    TextPlacement {
        left,
        right,
        top,
        bottom,
        middle,
        wrap,
    }
}
#[derive(Clone, Copy, Debug, Default)]
pub struct TextMetrics {
    pub width: f32,
    pub height: f32,
    pub rows: usize,
}
pub trait TextMeasurer {
    fn measure(&self, text: &str, style: TextStyle, wrap_width: Option<f32>) -> TextMetrics;
    /// Fallback for simple metric providers. Font-backed providers should preserve span styles.
    fn measure_rich(
        &self,
        text: &RichText,
        style: TextStyle,
        wrap_width: Option<f32>,
        soft_breaks: bool,
    ) -> TextMetrics {
        self.measure(&text.display_text(soft_breaks), style, wrap_width)
    }
}
/// Uses the same font system as the canvas, with its glyph advances and wrapping.
/// Font family selection is supplied by the app so loaded bold faces can be measured.
pub struct EguiMeasurer<'a> {
    pub context: &'a egui::Context,
    pub regular: egui::FontFamily,
    pub bold: egui::FontFamily,
}
impl TextMeasurer for EguiMeasurer<'_> {
    fn measure(&self, text: &str, style: TextStyle, wrap_width: Option<f32>) -> TextMetrics {
        self.measure_rich(&RichText::plain(text), style, wrap_width, true)
    }
    fn measure_rich(
        &self,
        text: &RichText,
        style: TextStyle,
        wrap_width: Option<f32>,
        soft_breaks: bool,
    ) -> TextMetrics {
        let galley = self
            .rich_galley(text, style, wrap_width, soft_breaks)
            .galley;
        TextMetrics {
            width: galley.size().x,
            height: galley.size().y,
            rows: galley.rows.len(),
        }
    }
}
#[derive(Clone, Debug)]
pub struct LayoutNode {
    pub path: NodePath,
    pub kind: Option<String>,
    pub bounds: Rect,
    pub uncropped: Rect,
    pub clip: Option<Rect>,
    pub properties: Properties,
    pub locked: bool,
    pub markup: bool,
    pub children: Vec<LayoutNode>,
}
#[derive(Clone, Debug, Default)]
pub struct Layout {
    pub nodes: Vec<LayoutNode>,
    pub warnings: Vec<String>,
    pub paint_order: Vec<NodePath>,
}
pub struct Engine<'a, M: TextMeasurer> {
    pub project: &'a Project,
    pub measurer: &'a M,
    pub context: Context,
}
impl<M: TextMeasurer> Engine<'_, M> {
    pub fn layout(&self, content: &Content) -> Result<Layout, String> {
        let mut result = Layout {
            paint_order: content.paint_order.clone(),
            ..Layout::default()
        };
        for expanded in self.project.resolve_all(&content.nodes)? {
            for (instance, path) in expanded.orphan_overrides {
                result.warnings.push(format!(
                    "Unused component override at {instance:?}/{path:?}"
                ));
            }
            result.nodes.push(self.node(
                &expanded.root,
                Point::default(),
                false,
                false,
                &mut vec![],
                &mut result.warnings,
            )?);
        }
        Ok(result)
    }
    fn node(
        &self,
        node: &Node,
        parent: Point,
        locked: bool,
        markup: bool,
        path: &mut NodePath,
        warnings: &mut Vec<String>,
    ) -> Result<LayoutNode, String> {
        let origin = node.origin.translated(parent);
        path.push(node.id);
        let locked = locked || node.locked;
        let markup = markup || node.markup;
        let (kind, properties, children, uncropped) = match &node.content {
            NodeContent::Control { kind, size } => {
                let spec =
                    catalog::get(kind).ok_or_else(|| format!("Unknown control type: {kind}"))?;
                let mut context = self.context;
                context.font_size = self.project.settings.font_size;
                let props = spec.effective(node, &context).map_err(|e| e.to_string())?;
                let (w, h) = self.measure(kind, &props, size.width, size.height, warnings)?;
                (
                    Some(kind.clone()),
                    props,
                    vec![],
                    Rect {
                        x: origin.x,
                        y: origin.y,
                        width: w,
                        height: h,
                    },
                )
            }
            NodeContent::Group { children } => {
                let mut layouts = vec![];
                for child in children {
                    layouts.push(self.node(child, origin, locked, markup, path, warnings)?);
                }
                let bounds = layouts
                    .iter()
                    .map(|c| c.bounds)
                    .reduce(Rect::union)
                    .unwrap_or(Rect {
                        x: origin.x,
                        y: origin.y,
                        ..Rect::default()
                    });
                (None, node.properties.clone(), layouts, bounds)
            }
            NodeContent::Instance { .. } => return Err("Unresolved component instance".into()),
        };
        let clip = node.crop.map(|c| Rect {
            x: uncropped.x + c.x * uncropped.width,
            y: uncropped.y + c.y * uncropped.height,
            width: c.width * uncropped.width,
            height: c.height * uncropped.height,
        });
        let result = LayoutNode {
            path: path.clone(),
            kind,
            bounds: clip.unwrap_or(uncropped),
            uncropped,
            clip,
            properties,
            locked,
            markup,
            children,
        };
        path.pop();
        Ok(result)
    }
    fn measure(
        &self,
        kind: &str,
        props: &Properties,
        width: Dimension,
        height: Dimension,
        warnings: &mut Vec<String>,
    ) -> Result<(f32, f32), String> {
        let text = match props.get("text") {
            Some(Value::Text(s)) => s.as_str(),
            _ => "",
        };
        let style = TextStyle {
            size: number(props, "font_size", self.context.font_size),
            bold: boolean(props, "bold"),
            italic: boolean(props, "italic"),
        };
        if kind == "table" {
            let grid = crate::rich_text::structured::Grid::parse(text);
            let available = match width {
                Dimension::Fixed(w) => Some(w),
                Dimension::Auto => None,
            };
            let measured = grid.table_layout(
                self.measurer,
                style,
                available,
                number(props, "row_height", 30.),
                boolean(props, "header"),
                self.project.settings.sketch,
            )?;
            let w = match width {
                Dimension::Fixed(w) => w,
                Dimension::Auto => measured.natural_width.max(1.),
            };
            let h = match height {
                Dimension::Fixed(h) => h,
                Dimension::Auto => measured.natural_height.max(1.),
            };
            validate_size(w, h)?;
            return Ok((w, h));
        }
        let placement = text_placement(kind);
        let multiline = placement.wrap;
        let (_, _, min_w, min_h) = match kind {
            "label" | "heading" | "subtitle" | "link" | "paragraph" => (0., 0., 1., 1.),
            "button" | "pointed_button" | "multiline_button" => (16., 10., 20., 20.),
            "checkbox" | "radio" => (20., 10., 20., 1.),
            "text_input" | "dropdown" | "search" | "numeric_stepper" | "date_chooser" => {
                (12., 8., 24., 24.)
            }
            "text_area" | "sticky_note" => (12., 12., 40., 30.),
            "shape" | "rectangle" | "ellipse" | "line" | "horizontal_rule" | "vertical_rule" => {
                (0., 0., 1., 1.)
            }
            _ => {
                if let (Dimension::Fixed(w), Dimension::Fixed(h)) = (width, height) {
                    warnings.push(format!(
                        "{kind}: control-specific measurement remains to be implemented"
                    ));
                    return Ok((w, h));
                }
                return Err(format!("Automatic layout is not implemented for {kind}"));
            }
        };
        let pad_x = placement.left + placement.right;
        let pad_y = placement.top + placement.bottom;
        let wrap = if multiline {
            match width {
                Dimension::Fixed(w) => Some((w - pad_x).max(1.)),
                Dimension::Auto => None,
            }
        } else {
            None
        };
        let parsed = RichText::parse(text, kind == "paragraph");
        if parsed.complexity_limited {
            warnings.push(format!(
                "{kind}: complex text preserved literally after parser limit"
            ));
        }
        let measured = self.measurer.measure_rich(&parsed, style, wrap, multiline);
        if ![measured.width, measured.height]
            .iter()
            .all(|v| v.is_finite() && *v >= 0.)
        {
            return Err("Text measurement returned invalid geometry".into());
        }
        let w = match width {
            Dimension::Auto => (measured.width + pad_x).max(min_w),
            Dimension::Fixed(w) => w,
        };
        let h = match height {
            Dimension::Auto => (measured.height + pad_y).max(min_h),
            Dimension::Fixed(h) => h,
        };
        validate_size(w, h)?;
        Ok((w, h))
    }
}
fn validate_size(w: f32, h: f32) -> Result<(), String> {
    if ![w, h]
        .iter()
        .all(|v| v.is_finite() && *v > 0. && *v <= 100_000.)
    {
        return Err("Layout dimensions exceed the supported range".into());
    }
    Ok(())
}
fn number(props: &Properties, key: &str, fallback: f32) -> f32 {
    match props.get(key) {
        Some(Value::Number(v)) => *v as f32,
        _ => fallback,
    }
}
fn boolean(props: &Properties, key: &str) -> bool {
    matches!(props.get(key), Some(Value::Bool(true)))
}
impl Layout {
    /// Ordered leaf geometry with the intersection of all ancestor clipping masks.
    pub fn leaves(&self) -> Vec<(&LayoutNode, Rect)> {
        fn collect<'a>(
            nodes: &'a [LayoutNode],
            clip: Option<Rect>,
            out: &mut Vec<(&'a LayoutNode, Rect)>,
        ) {
            for n in nodes {
                let clip = match (clip, n.clip) {
                    (Some(a), Some(b)) => Some(a.intersect(b)),
                    (a, None) => a,
                    (None, b) => b,
                };
                if n.kind.is_some() {
                    out.push((n, clip.map_or(n.bounds, |c| n.bounds.intersect(c))));
                } else {
                    collect(&n.children, clip, out);
                }
            }
        }
        let mut out = vec![];
        collect(&self.nodes, None, &mut out);
        if !self.paint_order.is_empty() {
            let order: std::collections::BTreeMap<_, _> = self
                .paint_order
                .iter()
                .enumerate()
                .map(|(i, path)| (path, i))
                .collect();
            out.sort_by_key(|(n, _)| {
                (1..=n.path.len())
                    .rev()
                    .find_map(|len| order.get(&n.path[..len].to_vec()).copied())
                    .unwrap_or(usize::MAX)
            });
        }
        out
    }
}
