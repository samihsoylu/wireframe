// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;

pub(super) fn raster_bounds(layer: &Layer) -> Bounds {
    let full = layer.control.bounds;
    let frame = layer.frame;
    if !boolean(&layer.properties, "image_border") {
        return full;
    }
    let inset = inset_frame(frame);
    let px = inset.x - frame.x;
    let py = inset.y - frame.y;
    let sx = (frame.w - 2. * px) / frame.w.max(0.1);
    let sy = (frame.h - 2. * py) / frame.h.max(0.1);
    Bounds::new(
        frame.x + px + (full.x - frame.x) * sx,
        frame.y + py + (full.y - frame.y) * sy,
        full.w * sx,
        full.h * sy,
    )
}
fn inset_frame(frame: Bounds) -> Bounds {
    let px = 5f32.min((frame.w - 0.1).max(0.) / 2.);
    let py = 5f32.min((frame.h - 0.1).max(0.) / 2.);
    Bounds::new(
        frame.x + px,
        frame.y + py,
        frame.w - 2. * px,
        frame.h - 2. * py,
    )
}
pub(super) fn raster_clip(layer: &Layer) -> Option<Bounds> {
    if boolean(&layer.properties, "image_border") {
        clip_with(layer.clip, inset_frame(layer.frame))
    } else {
        layer.clip
    }
}
impl Scene {
    pub(super) fn image_placeholder(
        &mut self,
        layer: &Layer,
        sketch: bool,
        fonts: &EguiMeasurer<'_>,
    ) -> Result<(), String> {
        // Retain the original frame under an ancestor/crop clip, as for artwork.
        let b = layer.control.bounds;
        let clip = clip_with(layer.clip, b);
        self.primitive(
            Primitive::Rect {
                b,
                fill: [255; 3],
                stroke: false,
                radius: 0.,
            },
            clip,
        );
        self.image_frame(b, clip, sketch);
        for points in [
            vec![[b.x + 5., b.y + 5.], [b.x + b.w - 5., b.y + b.h - 5.]],
            vec![[b.x + b.w - 5., b.y + 5.], [b.x + 5., b.y + b.h - 5.]],
        ] {
            self.primitive(
                Primitive::Line {
                    points,
                    color: if sketch { [92, 92, 94] } else { [51; 3] },
                    width: if sketch { 3. } else { 1. },
                },
                clip,
            );
        }
        if matches!(layer.properties.get("image"), Some(Value::Asset(_))) {
            self.image_label(
                layer,
                fonts,
                "Image unavailable",
                TextStyle {
                    size: number(&layer.properties, "_project_font_size", 13.),
                    bold: false,
                    italic: false,
                },
                PaintOptions::default(),
                layer.clip,
            )?;
        }
        self.image_decoration(layer, sketch, fonts)
    }

    fn image_frame(&mut self, b: Bounds, clip: Option<Bounds>, sketch: bool) {
        let width = if sketch { 3f32 } else { 1f32 };
        let inset = (width / 2.).min(b.w / 2.).min(b.h / 2.);
        let (l, t, r, d) = (
            b.x + inset,
            b.y + inset,
            b.x + b.w - inset,
            b.y + b.h - inset,
        );
        let points = if sketch {
            vec![
                [l, t],
                [l + (r - l) * 0.45, t + 0.35],
                [r, t],
                [r - 0.25, t + (d - t) * 0.5],
                [r, d],
                [l + (r - l) * 0.6, d - 0.35],
                [l, d],
                [l + 0.25, t + (d - t) * 0.5],
                [l, t],
            ]
        } else {
            vec![[l, t], [r, t], [r, d], [l, d], [l, t]]
        };
        self.primitive(
            Primitive::Line {
                points,
                color: if sketch { [0; 3] } else { [51; 3] },
                width,
            },
            clip,
        );
    }

    pub(super) fn image_decoration(
        &mut self,
        layer: &Layer,
        sketch: bool,
        fonts: &EguiMeasurer<'_>,
    ) -> Result<(), String> {
        if boolean(&layer.properties, "image_border") {
            self.image_frame(layer.frame, clip_with(layer.clip, layer.frame), sketch);
        }
        if layer.control.text.is_empty() {
            return Ok(());
        }
        self.image_label(
            layer,
            fonts,
            &layer.control.text,
            TextStyle {
                size: layer.control.font_size,
                bold: layer.control.bold,
                italic: boolean(&layer.properties, "italic"),
            },
            paint_options(&layer.properties),
            clip_with(layer.clip, layer.frame),
        )
    }

    fn image_label(
        &mut self,
        layer: &Layer,
        fonts: &EguiMeasurer<'_>,
        text: &str,
        style: TextStyle,
        options: PaintOptions,
        clip: Option<Bounds>,
    ) -> Result<(), String> {
        let b = layer.frame;
        let parsed = RichText::parse(text, false);
        let measured = fonts
            .rich_galley_with(&parsed, style, None, false, options)
            .size;
        let text_bounds = Bounds::new(
            b.x + (b.w - measured.x) / 2.,
            b.y + (b.h - measured.y) / 2.,
            measured.x,
            measured.y,
        );
        let background = Bounds::new(
            text_bounds.x - 4.,
            text_bounds.y - 5.,
            text_bounds.w + 8.,
            text_bounds.h + 10.,
        );
        self.primitive(
            Primitive::Rect {
                b: background,
                fill: [255; 3],
                stroke: false,
                radius: 4.,
            },
            clip,
        );
        self.items.last_mut().unwrap().opacity = 0.85;
        let (x, y, w, h) = (background.x, background.y, background.w, background.h);
        self.primitive(
            Primitive::Line {
                points: vec![
                    [x + 4., y],
                    [x + w - 4., y],
                    [x + w, y + 4.],
                    [x + w, y + h - 4.],
                    [x + w - 4., y + h],
                    [x + 4., y + h],
                    [x, y + h - 4.],
                    [x, y + 4.],
                    [x + 4., y],
                ],
                color: [204; 3],
                width: 1.,
            },
            clip,
        );
        self.items.last_mut().unwrap().opacity = 0.85;
        self.text(
            &parsed,
            style,
            TextBox {
                bounds: text_bounds,
                align: Alignment::Center,
                middle: true,
                wrap: false,
                soft_breaks: false,
                truncate: false,
                clip,
            },
            fonts,
            options,
        )
    }
}
