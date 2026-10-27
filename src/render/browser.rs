// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use crate::browser::{Browser, Chrome, Style, torn_edge};
impl Scene {
    pub(super) fn browser(
        &mut self,
        layer: &Layer,
        sketch: bool,
        fonts: &EguiMeasurer<'_>,
    ) -> Result<(), String> {
        let c = &layer.control;
        let b = c.bounds;
        let p = &layer.properties;
        let clip = clip_with(layer.clip, b);
        let browser = Browser::parse(&c.text, &self.title)?;
        let s = Style::new(sketch);
        let layout = browser.layout(
            fonts,
            c.font_size,
            b.w,
            b.h,
            sketch,
            matches!(p.get("border"),Some(Value::Choice(v)) if v=="torn_bottom"),
            boolean(p, "vertical_scrollbar"),
        );
        let offset = |r: Bounds| Bounds::new(b.x + r.x, b.y + r.y, r.w, r.h);
        if layout.mask {
            self.items.push(Item {
                opacity: 1.,
                clip,
                drawing: Drawing::BeginMask(mask::BottomMask {
                    keep_above: true,
                    bounds: b,
                    edge: torn_edge(b.w, b.h - 19., 0.)
                        .into_iter()
                        .map(|v| [b.x + v[0], b.y + v[1]])
                        .collect(),
                }),
            });
        }
        for (area, fill) in [(Some(layout.white), [255; 3]), (layout.content, c.fill)] {
            if let Some(area) = area.filter(|area| area.w > 0. && area.h > 0.) {
                self.primitive(
                    Primitive::Rect {
                        b: offset(area),
                        fill,
                        stroke: false,
                        radius: 0.,
                    },
                    clip,
                );
            }
        }
        self.browser_chrome(offset(layout.skin), s, sketch, clip);
        for (text, area, align) in [
            (&browser.address, layout.address, Alignment::Left),
            (&browser.title, Some(layout.title), Alignment::Center),
        ] {
            if let Some(area) = area {
                let mut options = paint_options(p);
                options.ink = if sketch {
                    Color32::BLACK
                } else {
                    Color32::from_gray(51)
                };
                options.underline = false;
                let links = self.links.len();
                self.text(
                    text,
                    TextStyle {
                        size: c.font_size,
                        bold: false,
                        italic: false,
                    },
                    TextBox {
                        bounds: offset(area),
                        align,
                        middle: false,
                        wrap: false,
                        soft_breaks: true,
                        truncate: true,
                        clip,
                    },
                    fonts,
                    options,
                )?;
                self.links.truncate(links);
            }
        }
        if let Some(scroll) = layout.scrollbar {
            self.browser_scrollbar(offset(scroll), number(p, "value", 0.), sketch, clip);
        }
        if layout.mask {
            self.items.push(Item {
                opacity: 1.,
                clip,
                drawing: Drawing::EndMask,
            });
            self.primitive(
                Primitive::Line {
                    points: torn_edge(b.w - 2., b.h - 19., s.tear_stroke)
                        .into_iter()
                        .map(|v| [b.x + 1. + v[0], b.y + v[1]])
                        .collect(),
                    color: [0; 3],
                    width: s.tear_stroke,
                },
                clip,
            );
        }
        Ok(())
    }
    fn browser_chrome(&mut self, b: Bounds, s: Style, sketch: bool, clip: Option<Bounds>) {
        let chrome = Chrome {
            width: b.w,
            height: b.h,
            style: s,
        };
        let sx = (b.w / (s.slices[1] + s.slices[2])).min(1.);
        let sy = (b.h / (s.slices[0] + s.slices[3])).min(1.);
        let point = |x: f32, y: f32| [b.x + chrome.x(x.abs(), x < 0.), b.y + chrome.y(y, false)];
        let ink = if sketch { [0; 3] } else { [51; 3] };
        let chrome_fill = if sketch { [193; 3] } else { [222, 222, 221] };
        if sketch {
            for primitive in super::browser_frame::sketch(b) {
                self.primitive(primitive, clip);
            }
            for offset in [0., 7.] {
                self.primitive(
                    Primitive::Line {
                        points: vec![
                            [b.x + chrome.x(18. + offset, true), b.y + chrome.y(7., true)],
                            [b.x + chrome.x(8. + offset, true), b.y + chrome.y(16., true)],
                        ],
                        color: ink,
                        width: 2.,
                    },
                    clip,
                );
            }
        } else {
            self.primitive(
                Primitive::Rect {
                    b: Bounds::new(b.x, b.y, b.w, s.content_top * sy),
                    fill: chrome_fill,
                    stroke: false,
                    radius: 0.,
                },
                clip,
            );
            self.rectangle_frame(b, ink, 2., clip);
            for y in [s.content_top, 32.] {
                self.primitive(
                    Primitive::Line {
                        points: vec![[b.x, b.y + y * sy], [b.x + b.w, b.y + y * sy]],
                        color: ink,
                        width: 1.,
                    },
                    clip,
                );
            }
        }
        let (left, right, top, height) = if sketch {
            (155., 94., 31., 19.)
        } else {
            (99., 34., 41., 24.)
        };
        let p0 = point(left, top);
        let p1 = point(-right, top + height);
        let address = Bounds::new(
            p0[0],
            p0[1],
            (p1[0] - p0[0]).max(0.),
            (p1[1] - p0[1]).max(0.),
        );
        if address.w > 0. {
            self.primitive(
                Primitive::Rect {
                    b: address,
                    fill: [255; 3],
                    stroke: false,
                    radius: if sketch { 2. } else { 0. },
                },
                clip,
            );
            if sketch {
                self.primitive(
                    outlines::sketch(address, outlines::Profile::Square, ink),
                    clip,
                );
            } else {
                self.rectangle_frame(address, [107; 3], 1., clip);
            }
        }
        // Original symbols drawn from simple paths; no embedded chrome artwork.
        for (cx, dir) in if sketch {
            [(30., -1.), (65., 1.)]
        } else {
            [(19., -1.), (44., 1.)]
        } {
            let cy = if sketch { 36. } else { 53. };
            if sketch {
                let pts = [
                    [-12., 0.],
                    [-1., -10.],
                    [-1., -5.],
                    [12., -5.],
                    [12., 5.],
                    [-1., 5.],
                    [-1., 10.],
                    [-12., 0.],
                ]
                .map(|v| point(cx + v[0] * -dir, cy + v[1]))
                .to_vec();
                // Outline arrow is concave: use two convex pieces for the fill.
                let tri = vec![
                    point(cx + 12. * dir, cy),
                    point(cx + dir, cy - 10.),
                    point(cx + dir, cy + 10.),
                ];
                self.primitive(
                    Primitive::Polygon {
                        points: tri,
                        fill: [255; 3],
                    },
                    clip,
                );
                let a = point(cx + if dir < 0. { -1. } else { -12. }, cy - 5.);
                self.primitive(
                    Primitive::Rect {
                        b: Bounds::new(a[0], a[1], 13. * sx, 10. * sy),
                        fill: [255; 3],
                        stroke: false,
                        radius: 0.,
                    },
                    clip,
                );
                self.primitive(
                    Primitive::Line {
                        points: pts,
                        color: ink,
                        width: 2.5,
                    },
                    clip,
                );
            } else {
                self.items.push(Item {
                    opacity: 1.,
                    clip,
                    drawing: Drawing::Icon {
                        bounds: Bounds::new(
                            point(cx - 8., cy - 8.)[0],
                            point(cx - 8., cy - 8.)[1],
                            16. * sx,
                            16. * sy,
                        ),
                        id: if dir < 0. {
                            "arrow-left-solid"
                        } else {
                            "arrow-right-solid"
                        }
                        .into(),
                        turns: 0,
                        color: [107, 107, 107, 255],
                    },
                });
            }
        }
        if sketch {
            for pts in [
                vec![point(96., 29.), point(106., 43.)],
                vec![point(106., 29.), point(96., 43.)],
            ] {
                self.primitive(
                    Primitive::Line {
                        points: pts,
                        color: ink,
                        width: 3.,
                    },
                    clip,
                );
            }
            let pts = vec![
                point(120., 36.),
                point(130., 24.),
                point(140., 36.),
                point(136., 36.),
                point(136., 48.),
                point(132., 48.),
                point(132., 41.),
                point(128., 41.),
                point(128., 48.),
                point(124., 48.),
                point(124., 36.),
                point(120., 36.),
            ];
            self.primitive(
                Primitive::Line {
                    points: pts,
                    color: ink,
                    width: 2.5,
                },
                clip,
            );
            let a = point(-77., 30.);
            self.primitive(
                Primitive::StyledEllipse {
                    b: Bounds::new(a[0], a[1], 57. * sx, 22. * sy),
                    fill: [255; 3],
                    stroke: ink,
                    width: 2.5,
                },
                clip,
            );
            self.browser_magnifier(point(-68., 40.), 4. * sx, 4. * sy, ink, 1.8, clip);
        } else {
            for (id, x, y, size) in [
                ("rotate-right-solid", 65., 45., 16.),
                ("magnifying-glass-solid", 103., 44., 16.),
                ("bars-solid", -27., 45., 16.),
            ] {
                let origin = point(x, y);
                self.items.push(Item {
                    opacity: 1.,
                    clip,
                    drawing: Drawing::Icon {
                        bounds: Bounds::new(origin[0], origin[1], size * sx, size * sy),
                        id: id.into(),
                        turns: 0,
                        color: [107, 107, 107, 255],
                    },
                });
            }
        }
    }
    fn browser_magnifier(
        &mut self,
        center: [f32; 2],
        rx: f32,
        ry: f32,
        color: [u8; 3],
        width: f32,
        clip: Option<Bounds>,
    ) {
        self.primitive(
            Primitive::StyledEllipse {
                b: Bounds::new(center[0] - rx, center[1] - ry, rx * 2., ry * 2.),
                fill: [255; 3],
                stroke: color,
                width,
            },
            clip,
        );
        self.primitive(
            Primitive::Line {
                points: vec![
                    [center[0] + rx * 0.7, center[1] + ry * 0.7],
                    [center[0] + rx * 1.8, center[1] + ry * 1.8],
                ],
                color,
                width,
            },
            clip,
        );
    }
}
