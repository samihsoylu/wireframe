// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::{Settings, Zoom};
use crate::model::{Bounds, Id};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Framing {
    pub zoom: f32,
    /// Artwork translation in viewport points, before document coordinates.
    pub offset: [f32; 2],
}

#[derive(Clone, Copy, Debug)]
pub struct PageExtent {
    pub surface: Id,
    pub with_markup: Bounds,
    pub without_markup: Bounds,
}
impl PageExtent {
    fn bounds(self, markup: bool) -> Bounds {
        if markup {
            self.with_markup
        } else {
            self.without_markup
        }
    }
}

/// Included surfaces share independent maximum width and height. An excluded
/// surface fits its own bounds. The result never enlarges the artwork.
pub fn centered_view(
    settings: &Settings,
    pages: &[PageExtent],
    current: Id,
    size: [f32; 2],
) -> Result<Framing, String> {
    settings.validate()?;
    if size.into_iter().any(|n| !n.is_finite() || n < 0.)
        || pages.iter().any(|page| {
            [page.with_markup, page.without_markup]
                .into_iter()
                .any(|b| {
                    [b.x, b.y, b.w, b.h].into_iter().any(|n| !n.is_finite()) || b.w < 0. || b.h < 0.
                })
        })
    {
        return Err("Presentation bounds are invalid".into());
    }
    let page = pages
        .iter()
        .find(|page| page.surface == current)
        .ok_or("Presentation page is unavailable")?;
    let bounds = page.bounds(settings.markup);
    let content = if settings.excluded.contains(&current) {
        [bounds.w, bounds.h]
    } else {
        pages
            .iter()
            .filter(|p| !settings.excluded.contains(&p.surface))
            .fold([0_f32; 2], |max, p| {
                let b = p.bounds(settings.markup);
                [max[0].max(b.w), max[1].max(b.h)]
            })
    };
    let zoom = if settings.zoom == Zoom::Fit && size[0] > 0. && size[1] > 0. {
        (size[0] / content[0])
            .min(size[1] / content[1])
            .clamp(0.05, 1.)
    } else {
        1.
    };
    let markup_offset = [bounds.x - page.with_markup.x, bounds.y - page.with_markup.y];
    let origin = [bounds.x, bounds.y];
    let offset: [f32; 2] = std::array::from_fn(|axis| {
        let margin =
            ((size[axis] - content[axis] * zoom).ceil() / 2. - markup_offset[axis]).max(0.);
        margin - origin[axis] * zoom
    });
    if !zoom.is_finite() || offset.iter().any(|n| !n.is_finite()) {
        return Err("Presentation view is invalid".into());
    }
    Ok(Framing { zoom, offset })
}
