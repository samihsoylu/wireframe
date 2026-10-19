// SPDX-License-Identifier: GPL-3.0-only
// Regional traversal with rendering conventions adapted from resvg.
// Upstream rendering/clip/image conventions: Copyright the Resvg Authors,
// Apache-2.0 OR MIT; original source and licenses remain in this directory.
use tiny_skia as sk;
type Result<T> = std::result::Result<T, String>;
struct Frame {
    logical: sk::IntSize,
    area: sk::IntRect,
    pixels: sk::Pixmap,
}
impl Frame {
    fn new(logical: sk::IntSize, area: sk::IntRect) -> Result<Self> {
        if area.x() < 0
            || area.y() < 0
            || area.right() as u32 > logical.width()
            || area.bottom() as u32 > logical.height()
        {
            return Err("Invalid regional frame".into());
        }
        Ok(Self {
            logical,
            area,
            pixels: sk::Pixmap::new(area.width(), area.height())
                .ok_or("Cannot allocate regional frame")?,
        })
    }
    fn empty(&self) -> Result<Self> {
        Self::new(self.logical, self.area)
    }
    fn origin(&self) -> (u32, u32) {
        (self.area.x() as u32, self.area.y() as u32)
    }
    fn path(
        &mut self,
        path: &sk::Path,
        paint: &sk::Paint,
        rule: sk::FillRule,
        stroke: Option<&sk::Stroke>,
        ts: sk::Transform,
    ) -> Result<()> {
        sk::paint_region(
            path,
            paint,
            rule,
            stroke,
            ts,
            self.logical,
            self.origin(),
            &mut self.pixels.as_mut(),
        )
        .map_err(str::to_owned)
    }
    fn rect(&mut self, rect: sk::Rect, paint: &sk::Paint, ts: sk::Transform) -> Result<()> {
        sk::paint_rect_region(
            rect,
            paint,
            ts,
            self.logical,
            self.origin(),
            &mut self.pixels.as_mut(),
        )
        .map_err(str::to_owned)
    }
}
/// Rasterize a region while retaining the full scene/layer
/// coordinate systems. No full-size intermediate pixel allocation is made.
pub fn render_region(
    tree: &usvg::Tree,
    transform: sk::Transform,
    region: sk::IntRect,
    mut checkpoint: impl FnMut() -> Result<()>,
) -> Result<sk::Pixmap> {
    checkpoint()?;
    let size = tree.size().to_int_size();
    if size.width() > 20000 || size.height() > 20000 {
        return Err("Canvas exceeds 20000 pixels".into());
    }
    let mut frame = Frame::new(size, region)?;
    let ctx = crate::render::Context {
        max_bbox: crate::max_filter_bbox(size.width(), size.height()),
    };
    nodes(tree.root(), &ctx, transform, &mut frame, 0, &mut checkpoint)?;
    checkpoint()?;
    Ok(frame.pixels)
}
fn nodes(
    parent: &usvg::Group,
    ctx: &crate::render::Context,
    ts: sk::Transform,
    out: &mut Frame,
    depth: usize,
    check: &mut dyn FnMut() -> Result<()>,
) -> Result<()> {
    if depth > 128 {
        return Err("Regional traversal depth exceeded".into());
    }
    for node in parent.children() {
        check()?;
        match node {
            usvg::Node::Path(p) => path(p, sk::BlendMode::SourceOver, ts, out, false)?,
            usvg::Node::Group(g) => group(g, ctx, ts, out, depth + 1, check)?,
            usvg::Node::Text(t) => group(t.flattened(), ctx, ts, out, depth + 1, check)?,
            usvg::Node::Image(i) => image(i, ts, out, depth + 1, check)?,
        }
    }
    Ok(())
}
fn paint<'a>(
    source: &'a usvg::Paint,
    opacity: usvg::Opacity,
    mode: sk::BlendMode,
    aa: bool,
) -> Result<sk::Paint<'a>> {
    let mut p = sk::Paint {
        blend_mode: mode,
        anti_alias: aa,
        ..Default::default()
    };
    p.shader = match source {
        usvg::Paint::Color(c) => sk::Shader::SolidColor(sk::Color::from_rgba8(
            c.red,
            c.green,
            c.blue,
            opacity.to_u8(),
        )),
        usvg::Paint::LinearGradient(g) => {
            crate::path::convert_linear_gradient(g, opacity).ok_or("Invalid gradient")?
        }
        usvg::Paint::RadialGradient(g) => {
            crate::path::convert_radial_gradient(g, opacity).ok_or("Invalid gradient")?
        }
        usvg::Paint::Pattern(_) => {
            return Err("SVG paint patterns are not yet supported by regional traversal".into());
        }
    };
    Ok(p)
}
fn path(
    p: &usvg::Path,
    mode: sk::BlendMode,
    ts: sk::Transform,
    out: &mut Frame,
    fill_only: bool,
) -> Result<()> {
    if !p.is_visible() {
        return Ok(());
    }
    let fill = |out: &mut Frame| -> Result<()> {
        let Some(fill) = p.fill() else {
            return Ok(());
        };
        if p.data().bounds().width() == 0. || p.data().bounds().height() == 0. {
            return Ok(());
        }
        let paint = paint(
            fill.paint(),
            fill.opacity(),
            mode,
            p.rendering_mode().use_shape_antialiasing(),
        )?;
        let rule = match fill.rule() {
            usvg::FillRule::NonZero => sk::FillRule::Winding,
            usvg::FillRule::EvenOdd => sk::FillRule::EvenOdd,
        };
        out.path(p.data(), &paint, rule, None, ts)
    };
    let stroke = |out: &mut Frame| -> Result<()> {
        let Some(stroke) = p.stroke() else {
            return Ok(());
        };
        let paint = paint(
            stroke.paint(),
            stroke.opacity(),
            mode,
            p.rendering_mode().use_shape_antialiasing(),
        )?;
        out.path(
            p.data(),
            &paint,
            sk::FillRule::Winding,
            Some(&stroke.to_tiny_skia()),
            ts,
        )
    };
    if fill_only {
        return fill(out);
    }
    if p.paint_order() == usvg::PaintOrder::FillAndStroke {
        fill(out)?;
        stroke(out)
    } else {
        stroke(out)?;
        fill(out)
    }
}
fn composite(
    src: &Frame,
    x: i32,
    y: i32,
    opacity: f32,
    mode: sk::BlendMode,
    out: &mut Frame,
) -> Result<()> {
    let rect = src.logical.to_int_rect(x, y).to_rect();
    let paint = sk::Paint {
        shader: sk::Pattern::new(
            src.pixels.as_ref(),
            sk::SpreadMode::Pad,
            sk::FilterQuality::Nearest,
            opacity,
            sk::Transform::from_translate((x + src.area.x()) as f32, (y + src.area.y()) as f32),
        ),
        blend_mode: mode,
        anti_alias: false,
        ..Default::default()
    };
    out.rect(rect, &paint, sk::Transform::identity())
}
fn group(
    g: &usvg::Group,
    ctx: &crate::render::Context,
    ts: sk::Transform,
    out: &mut Frame,
    depth: usize,
    check: &mut dyn FnMut() -> Result<()>,
) -> Result<()> {
    let ts = ts.pre_concat(g.transform());
    if !g.should_isolate() {
        return nodes(g, ctx, ts, out, depth, check);
    }
    if !g.filters().is_empty() || g.mask().is_some() {
        return Err("SVG filters/alpha masks are not yet supported by regional traversal".into());
    }
    let Some(b) = g.layer_bounding_box().transform(ts) else {
        return Ok(());
    };
    let expanded = sk::IntRect::from_xywh(
        (b.x().floor() as i32)
            .checked_sub(2)
            .ok_or("Layer x overflow")?,
        (b.y().floor() as i32)
            .checked_sub(2)
            .ok_or("Layer y overflow")?,
        (b.width().ceil() as u32)
            .checked_add(4)
            .ok_or("Layer width overflow")?,
        (b.height().ceil() as u32)
            .checked_add(4)
            .ok_or("Layer height overflow")?,
    )
    .ok_or("Invalid layer bounds")?;
    let Some(bounds) = crate::geom::fit_to_rect(expanded, ctx.max_bbox) else {
        return Ok(());
    };
    let Some(wanted) = out.area.intersect(&bounds) else {
        return Ok(());
    };
    let child_area = wanted
        .translate(-bounds.x(), -bounds.y())
        .ok_or("Invalid layer region")?;
    let mut child = Frame::new(
        sk::IntSize::from_wh(bounds.width(), bounds.height()).ok_or("Invalid layer size")?,
        child_area,
    )?;
    // Match canonical full-frame arithmetic and context, independent of the
    // retained buffer. Tile dimensions never enter layer-bound calculations.
    let mut dx = b.x();
    let mut dy = b.y();
    dx -= b.x() - bounds.x() as f32;
    dy -= b.y() - bounds.y() as f32;
    let shifted = sk::Transform::from_translate(-dx, -dy).pre_concat(ts);
    nodes(g, ctx, shifted, &mut child, depth, check)?;
    if let Some(clip) = g.clip_path() {
        apply_clip(clip, shifted, &mut child, depth + 1, check)?;
    }
    composite(
        &child,
        bounds.x(),
        bounds.y(),
        g.opacity().get(),
        crate::render::convert_blend_mode(g.blend_mode()),
        out,
    )
}
fn apply_clip(
    clip: &usvg::ClipPath,
    ts: sk::Transform,
    out: &mut Frame,
    depth: usize,
    check: &mut dyn FnMut() -> Result<()>,
) -> Result<()> {
    if depth > 128 {
        return Err("Clip depth exceeded".into());
    }
    check()?;
    let mut coverage = out.empty()?;
    coverage.pixels.fill(sk::Color::BLACK);
    clip_nodes(
        clip.root(),
        sk::BlendMode::Clear,
        ts.pre_concat(clip.transform()),
        &mut coverage,
        depth,
        check,
    )?;
    if let Some(parent) = clip.clip_path() {
        apply_clip(parent, ts, out, depth + 1, check)?;
    }
    let mut mask = sk::Mask::from_pixmap(coverage.pixels.as_ref(), sk::MaskType::Alpha);
    mask.invert();
    out.pixels.apply_mask(&mask);
    Ok(())
}
fn clip_nodes(
    parent: &usvg::Group,
    mode: sk::BlendMode,
    ts: sk::Transform,
    out: &mut Frame,
    depth: usize,
    check: &mut dyn FnMut() -> Result<()>,
) -> Result<()> {
    if depth > 128 {
        return Err("Clip traversal depth exceeded".into());
    }
    for node in parent.children() {
        check()?;
        match node {
            usvg::Node::Path(p) => path(p, mode, ts, out, true)?,
            usvg::Node::Text(t) => clip_nodes(t.flattened(), mode, ts, out, depth + 1, check)?,
            usvg::Node::Group(g) => {
                let ts = ts.pre_concat(g.transform());
                if let Some(clip) = g.clip_path() {
                    let mut child = out.empty()?;
                    clip_nodes(
                        g,
                        sk::BlendMode::SourceOver,
                        ts,
                        &mut child,
                        depth + 1,
                        check,
                    )?;
                    apply_clip(clip, ts, &mut child, depth + 1, check)?;
                    composite(&child, 0, 0, 1., sk::BlendMode::Xor, out)?;
                } else {
                    clip_nodes(g, mode, ts, out, depth + 1, check)?;
                }
            }
            _ => {}
        }
    }
    Ok(())
}
fn image(
    image: &usvg::Image,
    ts: sk::Transform,
    out: &mut Frame,
    depth: usize,
    check: &mut dyn FnMut() -> Result<()>,
) -> Result<()> {
    if !image.is_visible() {
        return Ok(());
    }
    check()?;
    if let usvg::ImageKind::SVG(tree) = image.kind() {
        let mut child = out.empty()?;
        let ctx = crate::render::Context {
            max_bbox: crate::max_filter_bbox(out.logical.width(), out.logical.height()),
        };
        nodes(tree.root(), &ctx, ts, &mut child, depth, check)?;
        return composite(&child, 0, 0, 1., sk::BlendMode::SourceOver, out);
    }
    #[cfg(feature = "raster-images")]
    {
        let raster = crate::image::raster_images::decode_raster(image.kind())
            .ok_or("Cannot decode image")?;
        let quality = match image.rendering_mode() {
            usvg::ImageRendering::OptimizeQuality | usvg::ImageRendering::HighQuality => {
                sk::FilterQuality::Bicubic
            }
            usvg::ImageRendering::Smooth => sk::FilterQuality::Bilinear,
            _ => sk::FilterQuality::Nearest,
        };
        let paint = sk::Paint {
            shader: sk::Pattern::new(
                raster.as_ref(),
                sk::SpreadMode::Pad,
                quality,
                1.,
                sk::Transform::identity(),
            ),
            ..Default::default()
        };
        let rect = sk::Rect::from_xywh(0., 0., raster.width() as f32, raster.height() as f32)
            .ok_or("Invalid image dimensions")?;
        out.rect(rect, &paint, ts)
    }
    #[cfg(not(feature = "raster-images"))]
    {
        Err("Raster decoding unavailable".into())
    }
}
