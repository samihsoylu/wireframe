// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Whole-control opacity using the existing canvas meshes and textures.
use eframe::{
    egui,
    glow::{self, HasContext},
};
use std::sync::{Arc, Mutex};

/// Owns reusable GPU tiles by opacity depth. Destruction requires the current GL context.
#[derive(Default)]
pub struct Compositor(Mutex<State>);
#[derive(Default)]
struct State {
    gpu: Option<Gpu>,
}
#[derive(Default)]
struct Resources {
    program: Option<glow::Program>,
    vao: Option<glow::VertexArray>,
    vertices: Option<glow::Buffer>,
    indices: Option<glow::Buffer>,
    fbo: Option<glow::Framebuffer>,
    texture: Option<glow::Texture>,
}
impl Resources {
    unsafe fn destroy(&mut self, gl: &glow::Context) {
        unsafe {
            if let Some(v) = self.texture.take() {
                gl.delete_texture(v);
            }
            if let Some(v) = self.fbo.take() {
                gl.delete_framebuffer(v);
            }
            if let Some(v) = self.vertices.take() {
                gl.delete_buffer(v);
            }
            if let Some(v) = self.indices.take() {
                gl.delete_buffer(v);
            }
            if let Some(v) = self.vao.take() {
                gl.delete_vertex_array(v);
            }
            if let Some(v) = self.program.take() {
                gl.delete_program(v);
            }
        }
    }
}
struct Gpu {
    resources: Resources,
    size: glow::UniformLocation,
    flip: glow::UniformLocation,
    tile: i32,
    child: Option<Box<Gpu>>,
}
impl Gpu {
    unsafe fn new(gl: &glow::Context) -> Result<Self, String> {
        // This compositor uses desktop GL 3 or GLES 3 APIs. Keep older
        // contexts on the explicit degraded path instead of calling absent APIs.
        if gl.version().major < 3 {
            return Err("Whole-control fading requires OpenGL 3 or newer".into());
        }
        let mut r = Resources::default();
        let result = (|| unsafe {
            let program = gl.create_program()?;
            r.program = Some(program);
            let prefix = if gl.version().is_embedded {
                "#version 300 es\nprecision highp float;\n"
            } else if gl.version().major == 3 && gl.version().minor < 2 {
                "#version 130\n"
            } else {
                "#version 150\n"
            };
            for (kind, body) in [
                (
                    glow::VERTEX_SHADER,
                    "in vec2 position; in vec2 uv; in vec4 color; uniform vec2 size; uniform float flip_y; out vec2 texture_uv; out vec4 tint; void main(){gl_Position=vec4(2.0*position.x/size.x-1.0,1.0-2.0*position.y/size.y,0.0,1.0); gl_Position.y*=flip_y; texture_uv=uv; tint=color;}",
                ),
                (
                    glow::FRAGMENT_SHADER,
                    "in vec2 texture_uv; in vec4 tint; uniform sampler2D pixels; out vec4 result; void main(){result=texture(pixels,texture_uv)*tint;}",
                ),
            ] {
                let shader = gl.create_shader(kind)?;
                gl.shader_source(shader, &format!("{prefix}{body}"));
                gl.compile_shader(shader);
                if !gl.get_shader_compile_status(shader) {
                    let error = gl.get_shader_info_log(shader);
                    gl.delete_shader(shader);
                    return Err(error);
                }
                gl.attach_shader(program, shader);
                gl.delete_shader(shader);
            }
            gl.link_program(program);
            if !gl.get_program_link_status(program) {
                return Err(gl.get_program_info_log(program));
            }
            r.vao = Some(gl.create_vertex_array()?);
            r.vertices = Some(gl.create_buffer()?);
            r.indices = Some(gl.create_buffer()?);
            gl.bind_vertex_array(r.vao);
            gl.bind_buffer(glow::ARRAY_BUFFER, r.vertices);
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, r.indices);
            for (name, n, kind, normalized, offset) in [
                ("position", 2, glow::FLOAT, false, 0),
                ("uv", 2, glow::FLOAT, false, 8),
                ("color", 4, glow::UNSIGNED_BYTE, true, 16),
            ] {
                let location = gl
                    .get_attrib_location(program, name)
                    .ok_or("Missing compositor vertex attribute")?;
                gl.enable_vertex_attrib_array(location);
                gl.vertex_attrib_pointer_f32(location, n, kind, normalized, 20, offset);
            }
            let size = gl
                .get_uniform_location(program, "size")
                .ok_or("Missing compositor size uniform")?;
            let flip = gl
                .get_uniform_location(program, "flip_y")
                .ok_or("Missing compositor orientation uniform")?;
            gl.use_program(Some(program));
            gl.uniform_1_i32(gl.get_uniform_location(program, "pixels").as_ref(), 0);
            r.fbo = Some(gl.create_framebuffer()?);
            r.texture = Some(gl.create_texture()?);
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, r.texture);
            for (key, value) in [
                (glow::TEXTURE_MIN_FILTER, glow::NEAREST),
                (glow::TEXTURE_MAG_FILTER, glow::NEAREST),
                (glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE),
                (glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE),
            ] {
                gl.tex_parameter_i32(glow::TEXTURE_2D, key, value as i32);
            }
            let tile = gl.get_parameter_i32(glow::MAX_TEXTURE_SIZE).min(1024);
            if tile <= 0 {
                return Err("No compositor texture capacity".into());
            }
            // Fixed allocation (at most 4 MiB); large controls are tiled without
            // scaling their geometry or reallocating during pointer movement.
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA8 as i32,
                tile,
                tile,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(None),
            );
            let previous = gl.get_parameter_framebuffer(glow::FRAMEBUFFER_BINDING);
            gl.bind_framebuffer(glow::FRAMEBUFFER, r.fbo);
            gl.framebuffer_texture_2d(
                glow::FRAMEBUFFER,
                glow::COLOR_ATTACHMENT0,
                glow::TEXTURE_2D,
                r.texture,
                0,
            );
            let complete =
                gl.check_framebuffer_status(glow::FRAMEBUFFER) == glow::FRAMEBUFFER_COMPLETE;
            gl.bind_framebuffer(glow::FRAMEBUFFER, previous);
            if !complete {
                return Err("Cannot allocate the control compositing surface".into());
            }
            Ok((size, flip, tile))
        })();
        match result {
            Ok((size, flip, tile)) => Ok(Self {
                resources: r,
                size,
                flip,
                tile,
                child: None,
            }),
            Err(e) => {
                unsafe {
                    r.destroy(gl);
                }
                Err(e)
            }
        }
    }
    unsafe fn setup(&self, gl: &glow::Context, info: &egui::PaintCallbackInfo, flipped: bool) {
        unsafe {
            gl.use_program(self.resources.program);
            gl.bind_vertex_array(self.resources.vao);
            gl.bind_buffer(glow::ARRAY_BUFFER, self.resources.vertices);
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, self.resources.indices);
            gl.active_texture(glow::TEXTURE0);
            gl.uniform_2_f32(
                Some(&self.size),
                info.screen_size_px[0] as f32 / info.pixels_per_point,
                info.screen_size_px[1] as f32 / info.pixels_per_point,
            );
            gl.uniform_1_f32(Some(&self.flip), if flipped { -1. } else { 1. });
            gl.enable(glow::BLEND);
            gl.disable(glow::DEPTH_TEST);
            gl.disable(glow::CULL_FACE);
            gl.color_mask(true, true, true, true);
            // egui already disables framebuffer sRGB when supported. Do not
            // issue desktop-only state changes on GLES contexts.
            gl.blend_equation_separate(glow::FUNC_ADD, glow::FUNC_ADD);
            gl.blend_func_separate(
                glow::ONE,
                glow::ONE_MINUS_SRC_ALPHA,
                glow::ONE,
                glow::ONE_MINUS_SRC_ALPHA,
            );
            gl.enable(glow::SCISSOR_TEST);
        }
    }
    unsafe fn mesh(&self, gl: &glow::Context, mesh: &egui::Mesh, texture: glow::Texture) {
        let mut vertices = Vec::with_capacity(mesh.vertices.len() * 20);
        for v in &mesh.vertices {
            for f in [v.pos.x, v.pos.y, v.uv.x, v.uv.y] {
                vertices.extend(f.to_ne_bytes());
            }
            vertices.extend(v.color.to_array());
        }
        let indices: Vec<u8> = mesh.indices.iter().flat_map(|i| i.to_ne_bytes()).collect();
        unsafe {
            gl.bind_texture(glow::TEXTURE_2D, Some(texture));
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, &vertices, glow::STREAM_DRAW);
            gl.buffer_data_u8_slice(glow::ELEMENT_ARRAY_BUFFER, &indices, glow::STREAM_DRAW);
            gl.draw_elements(
                glow::TRIANGLES,
                mesh.indices.len() as i32,
                glow::UNSIGNED_INT,
                0,
            );
        }
    }
    unsafe fn prepare(&mut self, gl: &glow::Context, depth: usize) -> Result<(), String> {
        if depth > 1 {
            if self.child.is_none() {
                self.child = Some(Box::new(unsafe { Self::new(gl)? }));
            }
            unsafe { self.child.as_mut().unwrap().prepare(gl, depth - 1)? };
        }
        Ok(())
    }
    unsafe fn destroy(&mut self, gl: &glow::Context) {
        unsafe {
            if let Some(child) = &mut self.child {
                child.destroy(gl);
            }
            self.resources.destroy(gl);
        }
    }
    // Every depth owns one reusable tile. Child groups are composed before
    // their opacity is applied onto the parent, preserving overlap semantics.
    unsafe fn paint_tile(
        &mut self,
        info: &egui::PaintCallbackInfo,
        painter: &eframe::egui_glow::Painter,
        group: &Group,
        bounds: [i32; 4],
        region: egui::Rect,
    ) {
        let gl = painter.gl();
        let density = info.pixels_per_point;
        let [sw, sh] = info.screen_size_px.map(|v| v as i32);
        let [left, top, right, bottom] = bounds;
        unsafe {
            gl.bind_framebuffer(glow::FRAMEBUFFER, self.resources.fbo);
            gl.viewport(-left, -top, sw, sh);
            gl.disable(glow::SCISSOR_TEST);
            gl.clear_color(0., 0., 0., 0.);
            gl.clear(glow::COLOR_BUFFER_BIT);
            self.setup(gl, info, true);
            for job in &group.jobs {
                let clip = job.clip_rect.intersect(region);
                let x = ((clip.left() * density).round() as i32).max(left);
                let y = ((clip.top() * density).round() as i32).max(top);
                let r = ((clip.right() * density).round() as i32).min(right);
                let b = ((clip.bottom() * density).round() as i32).min(bottom);
                if r <= x || b <= y {
                    continue;
                }
                let child_texture =
                    if let egui::epaint::Primitive::Callback(callback) = &job.primitive {
                        if let Some(nested) = callback.callback.downcast_ref::<Group>() {
                            let child = self.child.as_mut().expect("prepared opacity depth");
                            child.paint_tile(
                                info,
                                painter,
                                nested,
                                bounds,
                                clip.intersect(callback.rect),
                            );
                            Some((child.resources.texture.unwrap(), nested.opacity))
                        } else {
                            None
                        }
                    } else {
                        None
                    };
                if child_texture.is_some() {
                    gl.bind_framebuffer(glow::FRAMEBUFFER, self.resources.fbo);
                    gl.viewport(-left, -top, sw, sh);
                    self.setup(gl, info, true);
                }
                gl.scissor(x - left, y - top, r - x, b - y);
                match &job.primitive {
                    egui::epaint::Primitive::Mesh(mesh) => {
                        if let Some(texture) = painter.texture(mesh.texture_id) {
                            self.mesh(gl, mesh, texture);
                        }
                    }
                    egui::epaint::Primitive::Callback(_) => {
                        if let Some((texture, opacity)) = child_texture {
                            self.mesh(gl, &tile_mesh(bounds, density, self.tile, opacity), texture);
                        }
                    }
                }
            }
        }
    }
    unsafe fn paint(
        &mut self,
        info: egui::PaintCallbackInfo,
        painter: &eframe::egui_glow::Painter,
        group: &Group,
    ) {
        let gl = painter.gl();
        let density = info.pixels_per_point;
        let [sw, sh] = info.screen_size_px.map(|v| v as i32);
        let screen = egui::Rect::from_min_max(
            egui::Pos2::ZERO,
            egui::pos2(sw as f32 / density, sh as f32 / density),
        );
        let region = info.viewport.intersect(info.clip_rect).intersect(screen);
        let bounds = pixel_bounds(region, density, [sw, sh]);
        if bounds[2] <= bounds[0] || bounds[3] <= bounds[1] {
            return;
        }
        unsafe {
            let destination = gl.get_parameter_framebuffer(glow::FRAMEBUFFER_BINDING);
            let mut clear = [0.; 4];
            gl.get_parameter_f32_slice(glow::COLOR_CLEAR_VALUE, &mut clear);
            let fallback;
            let group = if self.prepare(gl, group.depth()).is_ok() {
                group
            } else {
                // Allocation failure retains visible content with approximate
                // inner fading, using the already allocated outer surface.
                fallback = group.flattened();
                &fallback
            };
            for tile in tiles(bounds, self.tile) {
                self.paint_tile(&info, painter, group, tile, region);
                let [left, top, right, bottom] = tile;
                gl.bind_framebuffer(glow::FRAMEBUFFER, destination);
                gl.viewport(0, 0, sw, sh);
                self.setup(gl, &info, false);
                gl.scissor(left, sh - bottom, right - left, bottom - top);
                self.mesh(
                    gl,
                    &tile_mesh(tile, density, self.tile, group.opacity),
                    self.resources.texture.unwrap(),
                );
            }
            gl.clear_color(clear[0], clear[1], clear[2], clear[3]);
            // egui restores its other GL state after callbacks. Restore the
            // actual caller's framebuffer, including offscreen render targets.
            gl.bind_framebuffer(glow::FRAMEBUFFER, destination);
        }
    }
}
fn tile_mesh(
    [left, top, right, bottom]: [i32; 4],
    density: f32,
    edge: i32,
    opacity: f32,
) -> egui::Mesh {
    let mut mesh = egui::Mesh::default();
    mesh.add_rect_with_uv(
        egui::Rect::from_min_max(
            egui::pos2(left as f32 / density, top as f32 / density),
            egui::pos2(right as f32 / density, bottom as f32 / density),
        ),
        egui::Rect::from_min_max(
            egui::Pos2::ZERO,
            egui::pos2(
                (right - left) as f32 / edge as f32,
                (bottom - top) as f32 / edge as f32,
            ),
        ),
        egui::Color32::WHITE.gamma_multiply(opacity),
    );
    mesh
}
#[derive(Clone)]
struct Group {
    jobs: Vec<egui::ClippedPrimitive>,
    opacity: f32,
}
impl Group {
    fn depth(&self) -> usize {
        1 + self
            .jobs
            .iter()
            .filter_map(|job| match &job.primitive {
                egui::epaint::Primitive::Callback(c) => {
                    c.callback.downcast_ref::<Self>().map(Self::depth)
                }
                _ => None,
            })
            .max()
            .unwrap_or(0)
    }
    fn bounds(&self) -> egui::Rect {
        self.jobs
            .iter()
            .map(|job| {
                match &job.primitive {
                    egui::epaint::Primitive::Mesh(mesh) => mesh.calc_bounds(),
                    egui::epaint::Primitive::Callback(c) => c.rect,
                }
                .intersect(job.clip_rect)
            })
            .fold(egui::Rect::NOTHING, egui::Rect::union)
    }
    fn flattened(&self) -> Self {
        fn append(
            group: &Group,
            opacity: f32,
            clip: egui::Rect,
            out: &mut Vec<egui::ClippedPrimitive>,
        ) {
            for job in &group.jobs {
                let clip = clip.intersect(job.clip_rect);
                match &job.primitive {
                    egui::epaint::Primitive::Mesh(mesh) => {
                        let mut mesh = mesh.clone();
                        for vertex in &mut mesh.vertices {
                            vertex.color = vertex.color.gamma_multiply(opacity);
                        }
                        out.push(egui::ClippedPrimitive {
                            clip_rect: clip,
                            primitive: egui::epaint::Primitive::Mesh(mesh),
                        });
                    }
                    egui::epaint::Primitive::Callback(c) => {
                        if let Some(child) = c.callback.downcast_ref::<Group>() {
                            append(child, opacity * child.opacity, clip.intersect(c.rect), out);
                        }
                    }
                }
            }
        }
        let mut jobs = Vec::new();
        append(self, 1., egui::Rect::EVERYTHING, &mut jobs);
        Self {
            jobs,
            opacity: self.opacity,
        }
    }
}
// Opacity callbacks remain data until the enclosing scene has finished its
// opacity and clipping scopes. Only then are outer groups bound to the GPU.
pub(super) fn masked_callback(
    callback: &egui::PaintCallback,
    mask: &super::mask::BottomMask,
    origin: egui::Pos2,
    scale: f32,
) -> Option<egui::PaintCallback> {
    let mut group = callback.callback.downcast_ref::<Group>()?.clone();
    for job in &mut group.jobs {
        match &mut job.primitive {
            egui::epaint::Primitive::Mesh(mesh) => *mesh = mask.clip_mesh(mesh, origin, scale),
            egui::epaint::Primitive::Callback(c) => {
                if let Some(clipped) = masked_callback(c, mask, origin, scale) {
                    *c = clipped;
                }
            }
        }
    }
    Some(egui::PaintCallback {
        rect: group.bounds().intersect(callback.rect),
        callback: Arc::new(group),
    })
}
fn pixel_bounds(region: egui::Rect, density: f32, screen: [i32; 2]) -> [i32; 4] {
    [
        (region.left() * density)
            .floor()
            .clamp(0., screen[0] as f32) as i32,
        (region.top() * density).floor().clamp(0., screen[1] as f32) as i32,
        (region.right() * density)
            .ceil()
            .clamp(0., screen[0] as f32) as i32,
        (region.bottom() * density)
            .ceil()
            .clamp(0., screen[1] as f32) as i32,
    ]
}
fn tiles([left, top, right, bottom]: [i32; 4], edge: i32) -> impl Iterator<Item = [i32; 4]> {
    (top..bottom).step_by(edge as usize).flat_map(move |y| {
        (left..right)
            .step_by(edge as usize)
            .map(move |x| [x, y, (x + edge).min(right), (y + edge).min(bottom)])
    })
}
impl Compositor {
    const KEY: &'static str = "wireframe-control-compositor";
    /// Initialize once with the current GL context and attach to this workspace.
    pub fn install(ctx: &egui::Context, gl: &glow::Context) -> Result<Arc<Self>, String> {
        let gpu = unsafe { Gpu::new(gl) }?;
        let owner = Arc::new(Self(Mutex::new(State { gpu: Some(gpu) })));
        ctx.data_mut(|d| d.insert_temp(egui::Id::new(Self::KEY), owner.clone()));
        Ok(owner)
    }
    pub fn destroy(&self, gl: &glow::Context) {
        if let Some(mut gpu) = self.0.lock().unwrap().gpu.take() {
            unsafe {
                gpu.destroy(gl);
            }
        }
    }
    /// Replace only the enclosed, already-painted control with an isolated draw.
    /// Called after its masks, so the meshes retain clipped UVs and edge coverage.
    pub fn finish(painter: &egui::Painter, start: usize, opacity: f32) {
        let id = painter.layer_id();
        let shapes = painter.ctx().graphics_mut(|g| {
            let Some(list) = g.get_mut(id) else {
                return vec![];
            };
            let shapes: Vec<_> = list.all_entries().skip(start).cloned().collect();
            for i in start..start + shapes.len() {
                list.reset_shape(egui::layers::ShapeIdx(i));
            }
            shapes
        });
        let jobs = painter
            .ctx()
            .tessellate(shapes, painter.ctx().pixels_per_point());
        let group = Group { jobs, opacity };
        let region = group.bounds().intersect(painter.clip_rect());
        if region.is_positive() {
            painter.add(egui::Shape::Callback(egui::PaintCallback {
                rect: region,
                callback: Arc::new(group),
            }));
        }
    }
    /// Finalize outer opacity groups after all scene masks and scopes close.
    pub fn resolve(painter: &egui::Painter, start: usize) {
        let owner = painter
            .ctx()
            .data(|d| d.get_temp::<Arc<Self>>(egui::Id::new(Self::KEY)));
        let usable = owner
            .as_ref()
            .is_some_and(|c| c.0.lock().unwrap().gpu.is_some());
        painter.ctx().graphics_mut(|graphics| {
            let Some(list) = graphics.get_mut(painter.layer_id()) else {
                return;
            };
            let entries: Vec<_> = list.all_entries().skip(start).cloned().collect();
            for (index, entry) in entries.into_iter().enumerate() {
                let egui::Shape::Callback(callback) = &entry.shape else {
                    continue;
                };
                let Some(group) = callback.callback.downcast_ref::<Group>() else {
                    continue;
                };
                let group = group.clone();
                let shape = if usable {
                    let owner = owner.as_ref().unwrap().clone();
                    egui::Shape::Callback(egui::PaintCallback {
                        rect: callback.rect,
                        callback: Arc::new(eframe::egui_glow::CallbackFn::new(
                            move |info, painter| {
                                let mut state = owner.0.lock().unwrap();
                                if let Some(gpu) = &mut state.gpu {
                                    unsafe {
                                        gpu.paint(info, painter, &group);
                                    }
                                }
                            },
                        )),
                    })
                } else {
                    // Headless or explicitly unsupported GL fallback retains
                    // every nested group using per-part opacity approximation.
                    let flat = group.flattened();
                    egui::Shape::Vec(
                        flat.jobs
                            .into_iter()
                            .filter_map(|job| {
                                if let egui::epaint::Primitive::Mesh(mut mesh) = job.primitive {
                                    for vertex in &mut mesh.vertices {
                                        vertex.color = vertex.color.gamma_multiply(flat.opacity);
                                    }
                                    let clip = job.clip_rect.intersect(entry.clip_rect);
                                    let mask = super::mask::BottomMask {
                                        bounds: crate::model::Bounds::new(
                                            clip.left(),
                                            clip.top(),
                                            clip.width(),
                                            clip.height(),
                                        ),
                                        edge: vec![
                                            [clip.left(), clip.bottom()],
                                            [clip.right(), clip.bottom()],
                                        ],
                                        keep_above: true,
                                    };
                                    Some(egui::Shape::mesh(mask.clip_mesh(
                                        &mesh,
                                        egui::Pos2::ZERO,
                                        1.,
                                    )))
                                } else {
                                    None
                                }
                            })
                            .collect(),
                    )
                };
                list.set(
                    egui::layers::ShapeIdx(start + index),
                    entry.clip_rect,
                    shape,
                );
            }
        });
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nested_groups_keep_clip_and_order_without_gl() {
        let ctx = egui::Context::default();
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(64., 64.),
                )),
                ..Default::default()
            },
            |ui| {
                let painter = ui.ctx().layer_painter(egui::LayerId::background());
                let bounds =
                    |x, y, w, h| egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(w, h));
                painter.rect_filled(bounds(40., 40., 10., 10.), 0., egui::Color32::BLUE);
                let start = painter
                    .ctx()
                    .graphics(|g| g.get(painter.layer_id()).unwrap().next_idx().0);
                painter
                    .with_clip_rect(bounds(10., 0., 10., 30.))
                    .rect_filled(bounds(0., 0., 30., 30.), 0., egui::Color32::RED);
                Compositor::finish(&painter, start, 0.5);
                super::super::mask::finish(
                    &painter,
                    start,
                    &super::super::mask::BottomMask {
                        bounds: crate::model::Bounds::new(0., 0., 30., 30.),
                        edge: vec![[0., 15.], [30., 15.]],
                        keep_above: true,
                    },
                    egui::Pos2::ZERO,
                    1.,
                );
                Compositor::finish(&painter, start, 0.5);
                Compositor::resolve(&painter, start);
                painter.rect_filled(bounds(50., 50., 10., 10.), 0., egui::Color32::GREEN);
            },
        );
        output.textures_delta.clear();
        let jobs = ctx.tessellate(output.shapes, output.pixels_per_point);
        let mut red = 0;
        let mut blue = 0;
        let mut green = 0;
        for job in jobs {
            let egui::epaint::Primitive::Mesh(mesh) = job.primitive else {
                panic!("unresolved opacity callback")
            };
            for vertex in mesh.vertices {
                let [r, g, b, a] = vertex.color.to_array();
                if r > 0 && g == 0 && b == 0 {
                    red += 1;
                    assert!((10.0..=20.0).contains(&vertex.pos.x));
                    assert!(vertex.pos.y <= 15.);
                    assert!(a <= 64);
                } else if b == 255 {
                    blue += 1;
                    assert_eq!(a, 255);
                } else if g == 255 {
                    green += 1;
                    assert_eq!(a, 255);
                }
            }
        }
        assert!(red > 0 && blue > 0 && green > 0);
    }
    #[test]
    fn fractional_bounds_and_large_tiles_cover_each_pixel_once() {
        let bounds = pixel_bounds(
            egui::Rect::from_min_max(egui::pos2(-2., 4.25), egui::pos2(2400.3, 1100.1)),
            2.,
            [5000, 3000],
        );
        assert_eq!(bounds, [0, 8, 4801, 2201]);
        let parts: Vec<_> = tiles(bounds, 1024).collect();
        assert_eq!(parts.len(), 15);
        let area: i64 = parts
            .iter()
            .map(|[l, t, r, b]| i64::from(r - l) * i64::from(b - t))
            .sum();
        assert_eq!(
            area,
            i64::from(bounds[2] - bounds[0]) * i64::from(bounds[3] - bounds[1])
        );
        for (i, a) in parts.iter().enumerate() {
            assert!(a[2] - a[0] <= 1024 && a[3] - a[1] <= 1024);
            for b in &parts[i + 1..] {
                assert!(a[2] <= b[0] || b[2] <= a[0] || a[3] <= b[1] || b[3] <= a[1]);
            }
        }
    }
}
