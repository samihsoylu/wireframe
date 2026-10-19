// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors

use crate::model::{Bounds, Control, Kind, Page};

pub const INK: [u8; 3] = [44, 48, 55];
#[derive(Clone, Debug)]
pub enum Primitive {
    /// One filled surface. Shared triangle edges must not be antialiased separately.
    Mesh {
        points: Vec<[f32; 2]>,
        indices: Vec<u32>,
        fill: [u8; 3],
    },
    Polygon {
        points: Vec<[f32; 2]>,
        fill: [u8; 3],
    },
    Rect {
        b: Bounds,
        fill: [u8; 3],
        stroke: bool,
        radius: f32,
    },
    Ellipse {
        b: Bounds,
        fill: [u8; 3],
    },
    StyledEllipse {
        b: Bounds,
        fill: [u8; 3],
        stroke: [u8; 3],
        width: f32,
    },
    Line {
        points: Vec<[f32; 2]>,
        color: [u8; 3],
        width: f32,
    },
    Text {
        x: f32,
        y: f32,
        text: String,
        size: f32,
        bold: bool,
        color: [u8; 3],
        center: bool,
    },
}
fn rect(out: &mut Vec<Primitive>, b: Bounds, fill: [u8; 3], radius: f32) {
    out.push(Primitive::Rect {
        b,
        fill,
        stroke: true,
        radius,
    });
}
fn line(out: &mut Vec<Primitive>, points: Vec<[f32; 2]>) {
    out.push(Primitive::Line {
        points,
        color: INK,
        width: 1.5,
    });
}
fn text(
    out: &mut Vec<Primitive>,
    x: f32,
    y: f32,
    value: &str,
    size: f32,
    bold: bool,
    center: bool,
) {
    out.push(Primitive::Text {
        x,
        y,
        text: value.into(),
        size,
        bold,
        color: INK,
        center,
    });
}
fn wrap(value: &str, width: f32, size: f32, max_lines: usize) -> Vec<String> {
    let cols = ((width / (size * 0.57)).floor() as usize).max(1);
    let mut lines = Vec::new();
    for para in value.lines() {
        let mut current = String::new();
        for ch in para.chars() {
            if current.chars().count() >= cols {
                lines.push(current);
                current = String::new();
                if lines.len() >= max_lines {
                    return lines;
                }
            }
            current.push(ch);
        }
        lines.push(current);
        if lines.len() >= max_lines {
            break;
        }
    }
    lines
}
pub fn control_scene(c: &Control, sketch: bool) -> Vec<Primitive> {
    let mut out = Vec::new();
    let b = c.bounds;
    let (x, y, w, h) = (b.x, b.y, b.w, b.h);
    let ink = INK;
    let gray = [240, 241, 243];
    let white = c.fill;
    let f = c.font_size.min(h.max(6.));
    let short = wrap(&c.text, (w - 16.).max(1.), f, 1)
        .first()
        .cloned()
        .unwrap_or_default();
    match c.kind {
        Kind::Popover
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
        | Kind::Icon
        | Kind::IconLabel
        | Kind::CircleButton
        | Kind::PointedButton
        | Kind::MultilineButton
        | Kind::ButtonBar
        | Kind::CheckboxGroup
        | Kind::RadioGroup
        | Kind::Menu
        | Kind::MenuBar
        | Kind::Tree
        | Kind::Accordion
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
        | Kind::Alert
        | Kind::AppBar
        | Kind::FieldSet
        | Kind::Breadcrumbs
        | Kind::LinkBar => {
            // Legacy scene fallback; the live editor uses the shared icon renderer.
            rect(&mut out, Bounds::new(x, y, w.min(h), w.min(h)), white, 2.);
        }
        Kind::Button => {
            rect(
                &mut out,
                Bounds::new(x + 2., y + 3., w, h),
                [222, 224, 227],
                5.,
            );
            rect(&mut out, b, white, 5.);
            text(
                &mut out,
                x + w / 2.,
                y + h / 2. + f * 0.34,
                &short,
                f,
                c.bold,
                true,
            );
        }
        Kind::Label | Kind::Heading | Kind::Link => {
            text(&mut out, x, y + f, &short, f, c.bold, false);
            if c.kind == Kind::Link {
                line(
                    &mut out,
                    vec![
                        [x, y + f + 3.],
                        [
                            x + (short.chars().count() as f32 * f * 0.52).min(w),
                            y + f + 3.,
                        ],
                    ],
                );
            }
        }
        Kind::Paragraph | Kind::TextArea => {
            if c.kind == Kind::TextArea {
                rect(&mut out, b, white, 2.);
            }
            let pad = if c.kind == Kind::Paragraph { 0. } else { 8. };
            let lines = wrap(&c.text, w - 2. * pad, f, ((h - pad) / (f * 1.35)) as usize);
            for (i, t) in lines.iter().enumerate() {
                text(
                    &mut out,
                    x + pad,
                    y + pad + f + i as f32 * f * 1.35,
                    t,
                    f,
                    c.bold,
                    false,
                );
            }
        }
        Kind::TextInput | Kind::Dropdown | Kind::Search => {
            rect(&mut out, b, white, 3.);
            let pad = if c.kind == Kind::Search { 30. } else { 10. };
            text(
                &mut out,
                x + pad,
                y + h / 2. + f * 0.34,
                &wrap(&c.text, w - pad - 24., f, 1)
                    .first()
                    .cloned()
                    .unwrap_or_default(),
                f,
                c.bold,
                false,
            );
            if c.kind == Kind::Dropdown {
                line(
                    &mut out,
                    vec![
                        [x + w - 22., y + h / 2. - 3.],
                        [x + w - 16., y + h / 2. + 3.],
                        [x + w - 10., y + h / 2. - 3.],
                    ],
                );
            }
            if c.kind == Kind::Search {
                out.push(Primitive::Ellipse {
                    b: Bounds::new(x + 10., y + h / 2. - 7., 11., 11.),
                    fill: white,
                });
                line(
                    &mut out,
                    vec![[x + 19., y + h / 2. + 3.], [x + 25., y + h / 2. + 9.]],
                );
            }
        }
        Kind::Checkbox | Kind::Radio => {
            let size = h.min(19.);
            let bb = Bounds::new(x, y + (h - size) / 2., size, size);
            if c.kind == Kind::Checkbox {
                rect(&mut out, bb, white, 2.);
                if c.checked {
                    line(
                        &mut out,
                        vec![
                            [x + 3., bb.y + size / 2.],
                            [x + size * 0.43, bb.y + size - 4.],
                            [x + size - 3., bb.y + 4.],
                        ],
                    );
                }
            } else {
                out.push(Primitive::Ellipse { b: bb, fill: white });
                if c.checked {
                    out.push(Primitive::Ellipse {
                        b: Bounds::new(x + 5., bb.y + 5., size - 10., size - 10.),
                        fill: ink,
                    });
                }
            }
            text(
                &mut out,
                x + size + 8.,
                y + h / 2. + f * 0.34,
                &wrap(&c.text, w - size - 8., f, 1)
                    .first()
                    .cloned()
                    .unwrap_or_default(),
                f,
                c.bold,
                false,
            );
        }
        Kind::Rectangle | Kind::Panel => {
            rect(
                &mut out,
                b,
                white,
                if c.kind == Kind::Panel { 4. } else { 0. },
            );
            if c.kind == Kind::Panel {
                out.push(Primitive::Rect {
                    b: Bounds::new(x + 1., y + 1., w - 2., 32.),
                    fill: gray,
                    stroke: false,
                    radius: 3.,
                });
                line(&mut out, vec![[x, y + 34.], [x + w, y + 34.]]);
                text(&mut out, x + 12., y + 23., &short, 14., true, false);
            } else {
                text(
                    &mut out,
                    x + w / 2.,
                    y + h / 2. + f * 0.34,
                    &short,
                    f,
                    c.bold,
                    true,
                );
            }
        }
        Kind::Ellipse | Kind::Shape => {
            out.push(Primitive::Ellipse { b, fill: white });
            text(
                &mut out,
                x + w / 2.,
                y + h / 2. + f * 0.34,
                &short,
                f,
                c.bold,
                true,
            );
        }
        Kind::Line | Kind::Arrow => {
            line(&mut out, vec![[x, y + h / 2.], [x + w, y + h / 2.]]);
            if c.kind == Kind::Arrow {
                line(
                    &mut out,
                    vec![
                        [x + w - 10., y + h / 2. - 6.],
                        [x + w, y + h / 2.],
                        [x + w - 10., y + h / 2. + 6.],
                    ],
                );
            }
        }
        Kind::Image => {
            rect(&mut out, b, [245, 246, 248], 2.);
            line(&mut out, vec![[x, y], [x + w, y + h]]);
            line(&mut out, vec![[x + w, y], [x, y + h]]);
            rect(
                &mut out,
                Bounds::new(x + w / 2. - 31., y + h / 2. - 12., 62., 24.),
                [255, 255, 255],
                3.,
            );
            text(
                &mut out,
                x + w / 2.,
                y + h / 2. + 5.,
                "Image",
                13.,
                false,
                true,
            );
        }
        Kind::Browser => {
            rect(&mut out, b, white, 6.);
            out.push(Primitive::Rect {
                b: Bounds::new(x + 1., y + 1., w - 2., 38.),
                fill: gray,
                stroke: false,
                radius: 5.,
            });
            line(&mut out, vec![[x, y + 40.], [x + w, y + 40.]]);
            for i in 0..3 {
                out.push(Primitive::Ellipse {
                    b: Bounds::new(x + 12. + i as f32 * 13., y + 15., 7., 7.),
                    fill: [210, 213, 218],
                });
            }
            rect(
                &mut out,
                Bounds::new(x + 64., y + 9., (w - 78.).max(10.), 23.),
                [255, 255, 255],
                3.,
            );
            text(
                &mut out,
                x + 73.,
                y + 25.,
                &wrap(&c.text, w - 95., 12., 1)
                    .first()
                    .cloned()
                    .unwrap_or_default(),
                12.,
                false,
                false,
            );
        }
        // Detailed keyboard rendering uses the shared scene pipeline.
        Kind::MobileMenu | Kind::MobilePicker | Kind::Calendar | Kind::DateChooser => {
            rect(&mut out, b, white, 0.);
        }
        Kind::MobileKeyboard => {
            rect(&mut out, b, [238; 3], 0.);
        }
        Kind::Phone | Kind::Tablet | Kind::Smartphone => {
            rect(&mut out, b, white, 18.);
            rect(
                &mut out,
                Bounds::new(x + 10., y + 34., (w - 20.).max(1.), (h - 66.).max(1.)),
                [253, 253, 253],
                1.,
            );
            line(
                &mut out,
                vec![[x + w / 2. - 18., y + 18.], [x + w / 2. + 18., y + 18.]],
            );
            line(
                &mut out,
                vec![
                    [x + w / 2. - 22., y + h - 16.],
                    [x + w / 2. + 22., y + h - 16.],
                ],
            );
        }
        Kind::List => {
            rect(&mut out, b, white, 2.);
            let row = 32.;
            for (i, t) in c.text.lines().take((h / row) as usize).enumerate() {
                if i % 2 == 1 {
                    out.push(Primitive::Rect {
                        b: Bounds::new(x + 1., y + i as f32 * row, w - 2., row),
                        fill: gray,
                        stroke: false,
                        radius: 0.,
                    });
                }
                text(
                    &mut out,
                    x + 10.,
                    y + i as f32 * row + 22.,
                    &wrap(t, w - 20., f, 1).first().cloned().unwrap_or_default(),
                    f,
                    c.bold,
                    false,
                );
            }
        }
        Kind::Table => {
            rect(&mut out, b, white, 0.);
            let rows: Vec<_> = c.text.lines().collect();
            let cols = rows
                .first()
                .map(|t| t.split(',').count())
                .unwrap_or(1)
                .max(1);
            let rh = 32.;
            out.push(Primitive::Rect {
                b: Bounds::new(x + 1., y + 1., w - 2., rh - 1.),
                fill: gray,
                stroke: false,
                radius: 0.,
            });
            for (i, row) in rows.iter().take((h / rh) as usize).enumerate() {
                if i > 0 {
                    line(
                        &mut out,
                        vec![[x, y + i as f32 * rh], [x + w, y + i as f32 * rh]],
                    );
                }
                for (j, t) in row.split(',').take(cols).enumerate() {
                    text(
                        &mut out,
                        x + j as f32 * w / cols as f32 + 7.,
                        y + i as f32 * rh + 22.,
                        &wrap(t, w / cols as f32 - 14., f, 1)
                            .first()
                            .cloned()
                            .unwrap_or_default(),
                        f,
                        i == 0,
                        false,
                    );
                }
            }
            for i in 1..cols {
                line(
                    &mut out,
                    vec![
                        [x + i as f32 * w / cols as f32, y],
                        [x + i as f32 * w / cols as f32, y + h],
                    ],
                );
            }
        }
        Kind::Tabs | Kind::VerticalTabs => {
            let values: Vec<_> = c.text.split(',').collect();
            let n = values.len().max(1);
            for (i, t) in values.iter().enumerate() {
                let tw = w / n as f32;
                rect(
                    &mut out,
                    Bounds::new(x + i as f32 * tw, y, tw, h),
                    if i == 0 { white } else { gray },
                    3.,
                );
                text(
                    &mut out,
                    x + (i as f32 + 0.5) * tw,
                    y + h / 2. + f * 0.34,
                    &wrap(t, tw - 12., f, 1).first().cloned().unwrap_or_default(),
                    f,
                    i == 0,
                    true,
                );
            }
        }
        Kind::Progress => {
            rect(&mut out, b, white, 3.);
            out.push(Primitive::Rect {
                b: Bounds::new(x + 3., y + 3., (w - 6.) * 0.6, (h - 6.).max(1.)),
                fill: [140, 151, 165],
                stroke: false,
                radius: 2.,
            });
        }
        Kind::Toggle => {
            rect(
                &mut out,
                b,
                if c.checked { [147, 171, 159] } else { gray },
                h / 2.,
            );
            out.push(Primitive::Ellipse {
                b: Bounds::new(
                    if c.checked { x + w - h + 3. } else { x + 3. },
                    y + 3.,
                    (h - 6.).max(1.),
                    (h - 6.).max(1.),
                ),
                fill: [255, 255, 255],
            });
        }
    }
    if sketch {
        // Deterministic secondary strokes add a light hand-drawn edge.
        let seed = (c.id % 7) as f32 * 0.08;
        let extra: Vec<_> = out
            .iter()
            .filter_map(|p| {
                if let Primitive::Rect {
                    b, stroke: true, ..
                } = p
                {
                    Some(Primitive::Line {
                        points: vec![
                            [b.x + 0.5, b.y + 0.7],
                            [b.x + b.w - 0.6, b.y + seed],
                            [b.x + b.w + 0.5, b.y + b.h - 0.5],
                        ],
                        color: [116, 119, 124],
                        width: 0.55,
                    })
                } else {
                    None
                }
            })
            .collect();
        out.extend(extra);
    }
    out
}
pub fn page_bounds(page: &Page) -> Bounds {
    page.controls
        .iter()
        .map(|c| c.bounds)
        .reduce(Bounds::union)
        .unwrap_or(Bounds::new(0., 0., 800., 600.))
}
pub(crate) fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
pub(crate) fn color(c: [u8; 3]) -> String {
    format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2])
}
pub fn svg(page: &Page, sketch: bool) -> String {
    let layers: Vec<_> = page.controls.iter().map(|c| (c.clone(), None)).collect();
    svg_layers(&page.name, &layers, sketch)
}
pub fn svg_layers(name: &str, layers: &[(Control, Option<Bounds>)], sketch: bool) -> String {
    let b = layers
        .iter()
        .map(|(c, clip)| clip.map_or(c.bounds, |clip| intersect(c.bounds, clip)))
        .reduce(Bounds::union)
        .unwrap_or(Bounds::new(0., 0., 800., 600.));
    let w = b.w + 48.;
    let h = b.h + 48.;
    let mut s = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"{} {} {w} {h}\"><title>{}</title><rect x=\"{}\" y=\"{}\" width=\"{w}\" height=\"{h}\" fill=\"white\"/>",
        b.x - 24.,
        b.y - 24.,
        escape(name),
        b.x - 24.,
        b.y - 24.
    );
    for (i, (c, clip)) in layers.iter().enumerate() {
        if let Some(b) = clip {
            s.push_str(&format!("<defs><clipPath id=\"clip{i}\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"/></clipPath></defs><g clip-path=\"url(#clip{i})\">",b.x,b.y,b.w,b.h));
        }
        for p in control_scene(c, sketch) {
            s.push_str(&primitive_svg(p));
        }
        if clip.is_some() {
            s.push_str("</g>");
        }
    }
    s.push_str("</svg>");
    s
}
pub fn png(page: &Page, sketch: bool) -> Result<Vec<u8>, String> {
    png_svg(&svg(page, sketch))
}
pub fn png_svg(svg: &str) -> Result<Vec<u8>, String> {
    png_svg_with_attribution(svg, None)
}
pub fn png_svg_with_attribution(svg: &str, attribution: Option<&str>) -> Result<Vec<u8>, String> {
    png_svg_with_fonts(svg, attribution, None)
}
pub(crate) fn export_tree(
    svg: &str,
    font: Option<&crate::fonts::ProjectFont>,
) -> Result<resvg::usvg::Tree, String> {
    let mut options = resvg::usvg::Options::default();
    // Use the same bundled fonts as the canvas, including on minimal Linux systems.
    for font in eframe::egui::FontDefinitions::default().font_data.values() {
        options.fontdb_mut().load_font_data(font.font.to_vec());
    }
    crate::fonts::install_export(options.fontdb_mut(), font);
    options.fontdb_mut().set_sans_serif_family("Ubuntu");
    resvg::usvg::Tree::from_str(svg, &options).map_err(|e| e.to_string())
}
pub fn png_svg_with_fonts(
    svg: &str,
    attribution: Option<&str>,
    font: Option<&crate::fonts::ProjectFont>,
) -> Result<Vec<u8>, String> {
    let tree = export_tree(svg, font)?;
    let size = tree.size().to_int_size();
    if size.width() > 8192
        || size.height() > 8192
        || u64::from(size.width()) * u64::from(size.height()) > 32_000_000
    {
        return Err("PNG export is limited to 8192 pixels per side and 32 million pixels. Use SVG for a larger canvas.".into());
    }
    let mut pixmap = resvg::tiny_skia::Pixmap::new(size.width(), size.height())
        .ok_or("Cannot allocate export image")?;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::identity(),
        &mut pixmap.as_mut(),
    );
    if let Some(attribution) = attribution {
        let rgba: Vec<u8> = pixmap
            .pixels()
            .iter()
            .flat_map(|p| {
                let p = p.demultiply();
                [p.red(), p.green(), p.blue(), p.alpha()]
            })
            .collect();
        let mut out = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut out, size.width(), size.height());
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder
                .add_itxt_chunk("Attribution".into(), attribution.into())
                .map_err(|e| e.to_string())?;
            let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
            writer.write_image_data(&rgba).map_err(|e| e.to_string())?;
            writer.finish().map_err(|e| e.to_string())?;
        }
        Ok(out)
    } else {
        pixmap.encode_png().map_err(|e| e.to_string())
    }
}

fn intersect(a: Bounds, b: Bounds) -> Bounds {
    let x = a.x.max(b.x);
    let y = a.y.max(b.y);
    Bounds::new(
        x,
        y,
        ((a.x + a.w).min(b.x + b.w) - x).max(0.),
        ((a.y + a.h).min(b.y + b.h) - y).max(0.),
    )
}

pub fn primitive_svg(p: Primitive) -> String {
    match p {
        Primitive::Rect {
            b,
            fill,
            stroke,
            radius,
        } => format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
            b.x,
            b.y,
            b.w,
            b.h,
            radius,
            color(fill),
            if stroke { color(INK) } else { "none".into() }
        ),
        Primitive::StyledEllipse {
            b,
            fill,
            stroke,
            width,
        } => format!(
            "<ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"/>",
            b.x + b.w / 2.,
            b.y + b.h / 2.,
            b.w / 2.,
            b.h / 2.,
            color(fill),
            if width > 0. {
                color(stroke)
            } else {
                "none".into()
            },
            width
        ),
        Primitive::Ellipse { b, fill } => format!(
            "<ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
            b.x + b.w / 2.,
            b.y + b.h / 2.,
            b.w / 2.,
            b.h / 2.,
            color(fill),
            color(INK)
        ),
        Primitive::Mesh {
            points,
            indices,
            fill,
        } => {
            let mut path = String::new();
            for t in indices.as_chunks::<3>().0 {
                let [a, b, c] = [
                    points[t[0] as usize],
                    points[t[1] as usize],
                    points[t[2] as usize],
                ];
                path.push_str(&format!(
                    "M{},{}L{},{}L{},{}Z",
                    a[0], a[1], b[0], b[1], c[0], c[1]
                ));
            }
            format!("<path d=\"{path}\" fill=\"{}\"/>", color(fill))
        }
        Primitive::Polygon { points, fill } => format!(
            "<polygon points=\"{}\" fill=\"{}\"/>",
            points
                .iter()
                .map(|p| format!("{},{}", p[0], p[1]))
                .collect::<Vec<_>>()
                .join(" "),
            color(fill)
        ),
        Primitive::Line {
            points,
            color: c,
            width,
        } => format!(
            "<polyline points=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{width}\" stroke-linecap=\"round\" stroke-linejoin=\"round\"/>",
            points
                .iter()
                .map(|p| format!("{},{}", p[0], p[1]))
                .collect::<Vec<_>>()
                .join(" "),
            color(c)
        ),
        Primitive::Text {
            x,
            y,
            text,
            size,
            bold,
            color: c,
            center,
        } => format!(
            "<text x=\"{x}\" y=\"{y}\" font-family=\"Ubuntu,DejaVu Sans,Arial,sans-serif\" font-size=\"{size}\" font-weight=\"{}\" text-anchor=\"{}\" fill=\"{}\">{}</text>",
            if bold { "bold" } else { "normal" },
            if center { "middle" } else { "start" },
            color(c),
            escape(&text)
        ),
    }
}
