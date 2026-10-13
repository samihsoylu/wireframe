// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use crate::scene::{color, escape};
use std::sync::LazyLock;
static EMBEDDED_FONTS: LazyLock<String> = LazyLock::new(|| {
    let mut css = String::from("<style>");
    for (weight, style, data) in [
        (400, "normal", crate::fonts::REGULAR),
        (700, "normal", crate::fonts::BOLD),
        (400, "italic", crate::fonts::ITALIC),
        (700, "italic", crate::fonts::BOLD_ITALIC),
    ] {
        css.push_str(&format!("@font-face{{font-family:'Comic Neue';font-weight:{weight};font-style:{style};src:url(data:font/ttf;base64,{}) format('truetype');}}",base64(data)));
    }
    css.push_str("</style><metadata>");
    css.push_str(&escape(include_str!(
        "../../assets/fonts/comic-neue/OFL.txt"
    )));
    css.push_str("</metadata>");
    css
});
impl Scene {
    pub fn svg(&self) -> String {
        let b = self.bounds;
        let w = b.w + 48.;
        let h = b.h + 48.;
        let mut out = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"{} {} {w} {h}\"><title>{}</title>{}<rect x=\"{}\" y=\"{}\" width=\"{w}\" height=\"{h}\" fill=\"white\"/>",
            b.x - 24.,
            b.y - 24.,
            escape(&self.title),
            *EMBEDDED_FONTS,
            b.x - 24.,
            b.y - 24.
        );
        for (i, item) in self.items.iter().enumerate() {
            if let Some(b) = item.clip {
                out.push_str(&format!("<defs><clipPath id=\"c{i}\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"/></clipPath></defs><g clip-path=\"url(#c{i})\">",b.x,b.y,b.w,b.h));
            }
            match &item.drawing {
    Drawing::Primitive(p)=>out.push_str(&crate::scene::primitive_svg(p.clone())),
    Drawing::Glyph(g)=>out.push_str(&format!("<text x=\"{}\" y=\"{}\" font-family=\"Comic Neue,Ubuntu,sans-serif\" font-size=\"{}\" font-weight=\"{}\" font-style=\"{}\" fill=\"{}\" fill-opacity=\"{}\" xml:space=\"preserve\">{}</text>",g.x,g.baseline,g.size,if g.bold{700}else{400},if g.italic{"italic"}else{"normal"},color([g.color[0],g.color[1],g.color[2]]),g.color[3]as f32/255.,escape(&g.text))),
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
        crate::scene::png_svg(&self.svg())
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
