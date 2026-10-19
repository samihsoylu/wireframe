// SPDX-License-Identifier: GPL-3.0-only
// Regional raster adapter; the ordinary drawing API retains its original behavior.
use crate::alpha_runs::AlphaRun;
use crate::blitter::Blitter;
use crate::geom::{IntSizeExt, ScreenIntRect};
use crate::pipeline::RasterPipelineBlitter;
use crate::{
    FillRule, IntRect, IntSize, LengthU32, LineCap, Paint, Path, PathBuilder, PixmapMut, Rect,
    Stroke, Transform,
};
use alloc::vec::Vec;
use core::num::NonZeroU16;

struct RegionBlitter<'a> {
    inner: &'a mut dyn Blitter,
    region: IntRect,
    offset: (u32, u32),
    aa: Vec<u8>,
    runs: Vec<AlphaRun>,
}
impl RegionBlitter<'_> {
    fn clipped(&self, x: u32, y: u32, w: u32, h: u32) -> Option<ScreenIntRect> {
        let rect =
            IntRect::from_xywh((x + self.offset.0) as i32, (y + self.offset.1) as i32, w, h)?;
        let rect = rect.intersect(&self.region)?;
        ScreenIntRect::from_xywh(
            (rect.x() - self.region.x()) as u32,
            (rect.y() - self.region.y()) as u32,
            rect.width(),
            rect.height(),
        )
    }
    fn aa_span(&mut self, x: u32, y: u32, w: u32, alpha: u8) {
        if alpha == 0 {
            return;
        }
        let Some(r) = self.clipped(x, y, w, 1) else {
            return;
        };
        let len = r.width() as usize;
        self.aa[0] = alpha;
        self.runs[0] = NonZeroU16::new(len as u16);
        self.runs[len] = None;
        self.inner.blit_anti_h(
            r.x(),
            r.y(),
            &mut self.aa[..len + 1],
            &mut self.runs[..len + 1],
        );
    }
}
impl Blitter for RegionBlitter<'_> {
    fn blit_h(&mut self, x: u32, y: u32, w: LengthU32) {
        if let Some(r) = self.clipped(x, y, w.get(), 1) {
            self.inner.blit_h(r.x(), r.y(), r.width_safe());
        }
    }
    fn blit_anti_h(&mut self, x: u32, y: u32, aa: &mut [u8], runs: &mut [AlphaRun]) {
        let mut i = 0;
        while let Some(run) = runs[i] {
            self.aa_span(x + i as u32, y, u32::from(run.get()), aa[i]);
            i += usize::from(run.get());
        }
    }
    fn blit_v(&mut self, x: u32, y: u32, h: LengthU32, alpha: u8) {
        if let Some(r) = self.clipped(x, y, 1, h.get()) {
            self.inner
                .blit_v(r.x(), r.y(), LengthU32::new(r.height()).unwrap(), alpha);
        }
    }
    fn blit_anti_h2(&mut self, x: u32, y: u32, a: u8, b: u8) {
        if let Some(r) = self.clipped(x, y, 2, 1) {
            if r.width() == 2 {
                self.inner.blit_anti_h2(r.x(), r.y(), a, b);
            } else {
                let alpha = if self.region.x() as u32 + r.x() == x + self.offset.0 {
                    a
                } else {
                    b
                };
                self.inner
                    .blit_v(r.x(), r.y(), LengthU32::new(1).unwrap(), alpha);
            }
        }
    }
    fn blit_anti_v2(&mut self, x: u32, y: u32, a: u8, b: u8) {
        if let Some(r) = self.clipped(x, y, 1, 2) {
            if r.height() == 2 {
                self.inner.blit_anti_v2(r.x(), r.y(), a, b);
            } else {
                let alpha = if self.region.y() as u32 + r.y() == y + self.offset.1 {
                    a
                } else {
                    b
                };
                self.inner
                    .blit_v(r.x(), r.y(), LengthU32::new(1).unwrap(), alpha);
            }
        }
    }
    fn blit_rect(&mut self, rect: &ScreenIntRect) {
        if let Some(r) = self.clipped(rect.x(), rect.y(), rect.width(), rect.height()) {
            self.inner.blit_rect(&r);
        }
    }
}
fn scan(
    path: &Path,
    paint: &Paint,
    rule: FillRule,
    cap: Option<LineCap>,
    clip: &ScreenIntRect,
    blitter: &mut dyn Blitter,
) {
    match (cap, paint.anti_alias) {
        (Some(cap), true) => crate::scan::hairline_aa::stroke_path(path, cap, clip, blitter),
        (Some(cap), false) => crate::scan::hairline::stroke_path(path, cap, clip, blitter),
        (None, true) => crate::scan::path_aa::fill_path(path, rule, clip, blitter),
        (None, false) => crate::scan::path::fill_path(path, rule, clip, blitter),
    }
}
/// Keep canonical geometry, raster clip and drawing order; translate only the
/// emitted pixel spans. Shader coordinates retain the canonical raster origin.
pub fn paint_region(
    path: &Path,
    paint: &Paint,
    mut rule: FillRule,
    stroke: Option<&Stroke>,
    transform: Transform,
    logical: IntSize,
    origin: (u32, u32),
    out: &mut PixmapMut,
) -> Result<(), &'static str> {
    if logical.width() > 100000 || logical.height() > 100000 {
        return Err("Logical raster dimensions exceed the regional limit");
    }
    if origin.0 > logical.width() || origin.1 > logical.height() {
        return Err("Region origin outside canvas");
    }
    let region = IntRect::from_xywh(origin.0 as i32, origin.1 as i32, out.width(), out.height())
        .ok_or("Invalid region")?;
    if region.right() as u32 > logical.width() || region.bottom() as u32 > logical.height() {
        return Err("Region outside canvas");
    }
    let mut paint = paint.clone();
    let mut path = path.clone();
    let mut cap = None;
    if let Some(stroke) = stroke {
        if stroke.width < 0. {
            return Ok(());
        }
        let scale = tiny_skia_path::PathStroker::compute_resolution_scale(&transform);
        if let Some(dash) = &stroke.dash {
            path = path.dash(dash, scale).ok_or("Cannot dash path")?;
        }
        if let Some(coverage) = crate::painter::treat_as_hairline(&paint, stroke, transform) {
            if coverage != 1. && paint.blend_mode.should_pre_scale_coverage() {
                let scale = (coverage * 256.) as i32;
                paint
                    .shader
                    .apply_opacity(((255 * scale) >> 8) as f32 / 255.);
            }
            cap = Some(stroke.line_cap);
        } else {
            path = path.stroke(stroke, scale).ok_or("Cannot expand stroke")?;
            rule = FillRule::Winding;
        }
    }
    path = path.transform(transform).ok_or("Invalid transform")?;
    paint.shader.transform(transform);
    if crate::painter::is_too_big_for_math(&path) {
        return Err("Path outside raster range");
    }
    if let Some(tiler) = crate::painter::DrawTiler::new(logical.width(), logical.height()) {
        for tile in tiler {
            let ts = Transform::from_translate(-(tile.x() as f32), -(tile.y() as f32));
            path = path.transform(ts).ok_or("Invalid tile transform")?;
            paint.shader.transform(ts);
            if region.intersect(&tile.to_int_rect()).is_some() {
                draw(
                    &path,
                    &paint,
                    rule,
                    cap,
                    &tile.size().to_screen_int_rect(0, 0),
                    region,
                    (tile.x(), tile.y()),
                    out,
                );
            }
            let ts = Transform::from_translate(tile.x() as f32, tile.y() as f32);
            path = path.transform(ts).ok_or("Invalid tile transform")?;
            paint.shader.transform(ts);
        }
    } else {
        draw(
            &path,
            &paint,
            rule,
            cap,
            &logical.to_screen_int_rect(0, 0),
            region,
            (0, 0),
            out,
        );
    }
    Ok(())
}
/// Preserve the renderer's dedicated rectangle scan path used for image/layer
/// composition; a rectangular path is not equivalent at fractional boundaries.
pub fn paint_rect_region(
    rect: Rect,
    paint: &Paint,
    transform: Transform,
    logical: IntSize,
    origin: (u32, u32),
    out: &mut PixmapMut,
) -> Result<(), &'static str> {
    if !transform.is_identity()
        || crate::painter::DrawTiler::new(logical.width(), logical.height()).is_some()
    {
        return paint_region(
            &PathBuilder::from_rect(rect),
            paint,
            FillRule::Winding,
            None,
            transform,
            logical,
            origin,
            out,
        );
    }
    if origin.0 > logical.width() || origin.1 > logical.height() {
        return Err("Region origin outside canvas");
    }
    let region = IntRect::from_xywh(origin.0 as i32, origin.1 as i32, out.width(), out.height())
        .ok_or("Invalid region")?;
    if region.right() as u32 > logical.width() || region.bottom() as u32 > logical.height() {
        return Err("Region outside canvas");
    }
    let mut subpix = out.as_subpixmap();
    let Some(mut pipeline) = RasterPipelineBlitter::new(paint, None, &mut subpix) else {
        return Ok(());
    };
    pipeline.set_shader_origin((region.x(), region.y()));
    let mut blitter = RegionBlitter {
        inner: &mut pipeline,
        region,
        offset: (0, 0),
        aa: alloc::vec![0;region.width() as usize+1],
        runs: alloc::vec![None;region.width() as usize+1],
    };
    let clip = logical.to_screen_int_rect(0, 0);
    if paint.anti_alias {
        crate::scan::fill_rect_aa(&rect, &clip, &mut blitter);
    } else {
        crate::scan::fill_rect(&rect, &clip, &mut blitter);
    }
    Ok(())
}
fn draw(
    path: &Path,
    paint: &Paint,
    rule: FillRule,
    cap: Option<LineCap>,
    clip: &ScreenIntRect,
    region: IntRect,
    offset: (u32, u32),
    out: &mut PixmapMut,
) {
    let mut subpix = out.as_subpixmap();
    let Some(mut pipeline) = RasterPipelineBlitter::new(paint, None, &mut subpix) else {
        return;
    };
    pipeline.set_shader_origin((region.x() - offset.0 as i32, region.y() - offset.1 as i32));
    let mut blitter = RegionBlitter {
        inner: &mut pipeline,
        region,
        offset,
        aa: alloc::vec![0;region.width() as usize+1],
        runs: alloc::vec![None;region.width() as usize+1],
    };
    scan(path, paint, rule, cap, clip, &mut blitter);
}
