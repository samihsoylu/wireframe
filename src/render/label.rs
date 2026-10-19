// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use crate::text_controls::LabelOrientation;

impl Scene {
    pub(super) fn oriented_label(
        &mut self,
        layer: &Layer,
        fonts: &EguiMeasurer<'_>,
    ) -> Result<(), String> {
        let control = &layer.control;
        let bounds = control.bounds;
        let orientation = LabelOrientation::from_properties(&layer.properties);
        let rotation = orientation.radians();
        let rotate = egui::emath::Rot2::from_angle(rotation);
        let parsed = RichText::parse(&control.text, false);
        let style = TextStyle {
            size: control.font_size,
            bold: control.bold,
            italic: boolean(&layer.properties, "italic"),
        };
        let options = paint_options(&layer.properties);
        let painted = fonts.rich_galley_with(&parsed, style, None, false, options);
        if let Some(error) = &painted.error {
            return Err(error.clone());
        }
        let baseline = painted.rows.first().map_or(0., |row| row.baseline);
        let (pivot, origin) = match orientation {
            LabelOrientation::Left => (egui::Vec2::ZERO, egui::vec2(bounds.x, bounds.y + bounds.h)),
            LabelOrientation::Right => {
                (egui::Vec2::ZERO, egui::vec2(bounds.x + bounds.w, bounds.y))
            }
            LabelOrientation::Diagonal => (
                egui::vec2(0., baseline),
                egui::vec2(bounds.x, bounds.y + bounds.h - painted.size.y + baseline),
            ),
            LabelOrientation::Upright => unreachable!(),
        };
        let transform = |x: f32, y: f32| origin + rotate * (egui::vec2(x, y) - pivot);
        let text_corners = [
            [0., 0.],
            [painted.size.x, 0.],
            [painted.size.x, painted.size.y],
            [0., painted.size.y],
        ]
        .map(|p| {
            let q = transform(p[0], p[1]);
            [q.x, q.y]
        });
        let text_bounds = quad_bounds(text_corners);
        let visible = layer
            .clip
            .map_or(text_bounds, |clip| intersect(text_bounds, clip));
        if visible.w > 0. && visible.h > 0. {
            self.bounds = self.bounds.union(visible);
        }
        let first_item = self.items.len();
        let first_link = self.links.len();
        // Rotated captions always use their natural single-line width, even
        // when the selection frame has an explicit width. Clip after rotating.
        self.text(
            &parsed,
            style,
            TextBox {
                bounds: Bounds::new(0., 0., painted.size.x, painted.size.y),
                align: Alignment::Left,
                middle: false,
                wrap: false,
                soft_breaks: false,
                truncate: false,
                clip: None,
            },
            fonts,
            options,
        )?;
        for item in &mut self.items[first_item..] {
            item.clip = layer.clip;
            match &mut item.drawing {
                Drawing::Glyph(glyph) => {
                    let point = transform(glyph.x, glyph.baseline);
                    glyph.x = point.x;
                    glyph.baseline = point.y;
                    glyph.rotation = rotation;
                }
                Drawing::Primitive(Primitive::Line { points, .. }) => {
                    for point in points {
                        let rotated = transform(point[0], point[1]);
                        *point = [rotated.x, rotated.y];
                    }
                }
                _ => unreachable!("Label text emits glyphs and text-decoration lines"),
            }
        }
        for link in &mut self.links[first_link..] {
            let b = link.bounds;
            let quad = [
                [b.x, b.y],
                [b.x + b.w, b.y],
                [b.x + b.w, b.y + b.h],
                [b.x, b.y + b.h],
            ]
            .map(|p| {
                let q = transform(p[0], p[1]);
                [q.x, q.y]
            });
            link.hint = quad_bounds(quad);
            link.bounds = layer
                .clip
                .map_or(link.hint, |clip| intersect(link.hint, clip));
            link.quad = Some(quad);
        }
        self.links
            .retain(|link| link.bounds.w > 0. && link.bounds.h > 0.);
        Ok(())
    }
}

impl LinkRegion {
    pub fn contains(&self, x: f32, y: f32) -> bool {
        if !self.bounds.contains(x, y) {
            return false;
        }
        let Some(quad) = self.quad else {
            return true;
        };
        let mut positive = false;
        let mut negative = false;
        for i in 0..4 {
            let a = quad[i];
            let b = quad[(i + 1) % 4];
            let cross = (b[0] - a[0]) * (y - a[1]) - (b[1] - a[1]) * (x - a[0]);
            positive |= cross > 0.001;
            negative |= cross < -0.001;
        }
        !(positive && negative)
    }
}

fn quad_bounds(quad: [[f32; 2]; 4]) -> Bounds {
    let min_x = quad.iter().map(|p| p[0]).fold(f32::INFINITY, f32::min);
    let min_y = quad.iter().map(|p| p[1]).fold(f32::INFINITY, f32::min);
    let max_x = quad.iter().map(|p| p[0]).fold(f32::NEG_INFINITY, f32::max);
    let max_y = quad.iter().map(|p| p[1]).fold(f32::NEG_INFINITY, f32::max);
    Bounds::new(min_x, min_y, max_x - min_x, max_y - min_y)
}
