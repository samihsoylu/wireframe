// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use crate::scene::{color, escape};
fn embedded_fonts(
    font: Option<&crate::fonts::ProjectFont>,
    fallbacks: &std::collections::BTreeSet<&str>,
) -> String {
    let mut css = String::from("<style>");
    let family = crate::fonts::export_family(font);
    for (index, data) in crate::fonts::faces(font).into_iter().enumerate() {
        let weight = if index % 2 == 1 { 700 } else { 400 };
        let style = if index >= 2 { "italic" } else { "normal" };
        let (media, format) = if data.starts_with(b"OTTO") {
            ("font/otf", "opentype")
        } else {
            ("font/ttf", "truetype")
        };
        css.push_str(&format!("@font-face{{font-family:'{family}';font-weight:{weight};font-style:{style};src:url(data:{media};base64,{}) format('{format}');}}",base64(data)));
    }
    let definitions = egui::FontDefinitions::default();
    for &face in fallbacks {
        if let Some(data) = definitions.font_data.get(face) {
            css.push_str(&format!("@font-face{{font-family:'{}';font-weight:400;font-style:normal;src:url(data:font/ttf;base64,{}) format('truetype');}}", crate::fonts::export_fallback_family(face), base64(&data.font)));
        }
    }
    css.push_str("</style><metadata>");
    css.push_str(&escape(crate::fonts::notices(font)));
    if !fallbacks.is_empty() {
        css.push_str(&escape(crate::fonts::FALLBACK_NOTICES));
    }
    css.push_str("</metadata>");
    css
}
impl Scene {
    pub fn svg(&self) -> String {
        let fallbacks = self
            .items
            .iter()
            .filter_map(|item| match &item.drawing {
                Drawing::Glyph(g) if !g.face.starts_with("canvas-") => Some(g.face.as_str()),
                _ => None,
            })
            .collect();
        let b = self.bounds;
        let w = b.w + 48.;
        let h = b.h + 48.;
        let mut out = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"{} {} {w} {h}\"><title>{}</title>{}<rect x=\"{}\" y=\"{}\" width=\"{w}\" height=\"{h}\" fill=\"white\"/>",
            b.x - 24.,
            b.y - 24.,
            escape(&self.title),
            embedded_fonts(self.font.as_deref(), &fallbacks),
            b.x - 24.,
            b.y - 24.
        );
        if self
            .items
            .iter()
            .any(|item| matches!(item.drawing, Drawing::Icon { .. }))
        {
            out.push_str(&format!(
                "<metadata>{}</metadata>",
                escape(crate::icons::ATTRIBUTION)
            ));
        }
        for (i, item) in self.items.iter().enumerate() {
            match &item.drawing {
                Drawing::BeginMask(mask) => {
                    let points = mask
                        .polygon()
                        .iter()
                        .map(|p| format!("{},{}", p[0], p[1]))
                        .collect::<Vec<_>>()
                        .join(" ");
                    out.push_str(&format!("<defs><clipPath id=\"m{i}\"><polygon points=\"{points}\"/></clipPath></defs><g clip-path=\"url(#m{i})\">"));
                    continue;
                }
                Drawing::EndMask => {
                    out.push_str("</g>");
                    continue;
                }
                _ => {}
            }
            if let Some(b) = item.clip {
                out.push_str(&format!("<defs><clipPath id=\"c{i}\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"/></clipPath></defs><g clip-path=\"url(#c{i})\">",b.x,b.y,b.w,b.h));
            }
            if item.opacity != 1. {
                out.push_str(&format!("<g opacity=\"{}\">", item.opacity));
            }
            match &item.drawing {
    Drawing::BeginMask(_) | Drawing::EndMask => unreachable!(),
    Drawing::Icon { bounds, id, color, turns } => {
        out.push_str(&format!("<g transform=\"rotate({} {} {})\">",-90*i32::from(*turns),bounds.x+bounds.w/2.,bounds.y+bounds.h/2.));
        if let Some(icon) = crate::icons::get(id) {out.push_str(&icon.svg(*bounds,*color));}
        else {out.push_str(&format!("<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"none\" stroke=\"{}\"/>",bounds.x,bounds.y,bounds.w,bounds.h,crate::scene::color([color[0],color[1],color[2]])));}
        out.push_str("</g>");
    },
    Drawing::Image { bounds: b, data } => out.push_str(&format!("<image x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" preserveAspectRatio=\"none\" href=\"data:image/png;base64,{}\"/>", b.x,b.y,b.w,b.h,base64(&data.png))),
    Drawing::Primitive(p)=>out.push_str(&crate::scene::primitive_svg(p.clone())),
    Drawing::Glyph(g) => {
        if let Some(text) = &g.positioned {
            out.push_str(&format!("<g aria-label=\"{}\" transform=\"translate({} {})\">{}</g>",escape(&g.text),g.x,g.baseline,text.svg(g.color)));
        } else {
        let primary = g.face.starts_with("canvas-");
        let family = if primary { crate::fonts::export_family(self.font.as_deref()).to_owned() } else { crate::fonts::export_fallback_family(&g.face) };
        out.push_str(&format!("<text x=\"{}\" y=\"{}\" font-family=\"{}\" font-size=\"{}\" font-weight=\"{}\" font-style=\"{}\" fill=\"{}\" fill-opacity=\"{}\" xml:space=\"preserve\">{}</text>",g.x,g.baseline,escape(&family),g.size*g.face_scale,if primary && g.bold{700}else{400},if primary && g.italic{"italic"}else{"normal"},color([g.color[0],g.color[1],g.color[2]]),g.color[3]as f32/255.,escape(&g.text)));
        }
    },
   }
            if item.opacity != 1. {
                out.push_str("</g>");
            }
            if item.clip.is_some() {
                out.push_str("</g>");
            }
        }
        for link in &self.links {
            if link.target.starts_with("https://") || link.target.starts_with("http://") {
                let b = link.bounds;
                out.push_str(&format!("<a href=\"{}\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"transparent\"/></a>",escape(&link.target),b.x,b.y,b.w,b.h));
            }
        }
        out.push_str("</svg>");
        out
    }
    pub fn png(&self) -> Result<Vec<u8>, String> {
        let attribution = self
            .items
            .iter()
            .any(|item| matches!(item.drawing, Drawing::Icon { .. }))
            .then_some(crate::icons::ATTRIBUTION);
        crate::scene::png_svg_with_fonts(&self.svg(), attribution, self.font.as_deref())
    }
}
fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = ((chunk[0] as u32) << 16)
            | ((chunk.get(1).copied().unwrap_or(0) as u32) << 8)
            | chunk.get(2).copied().unwrap_or(0) as u32;
        for shift in [18, 12, 6, 0] {
            let index = (n >> shift) & 63;
            let padding = (shift == 6 && chunk.len() < 2) || (shift == 0 && chunk.len() < 3);
            result.push(if padding {
                '='
            } else {
                ALPHABET[index as usize] as char
            });
        }
    }
    result
}
