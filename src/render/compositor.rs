// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Whole-control opacity using the existing canvas meshes and textures.
use eframe::{
    egui,
    glow::{self, HasContext},
};
use std::sync::{Arc, Mutex};

/// Owns one reusable GPU tile. Destruction must run with the rendering context current.
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
    unsafe fn paint(
        &self,
        info: egui::PaintCallbackInfo,
        painter: &eframe::egui_glow::Painter,
        jobs: &[egui::ClippedPrimitive],
        opacity: f32,
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
            let mut clear = [0.; 4];
            gl.get_parameter_f32_slice(glow::COLOR_CLEAR_VALUE, &mut clear);
            for [left, top, right, bottom] in tiles(bounds, self.tile) {
                let width = right - left;
                let height = bottom - top;
                gl.bind_framebuffer(glow::FRAMEBUFFER, self.resources.fbo);
                gl.viewport(-left, -top, sw, sh);
                gl.disable(glow::SCISSOR_TEST);
                gl.clear_color(0., 0., 0., 0.);
                gl.clear(glow::COLOR_BUFFER_BIT);
                self.setup(gl, &info, true);
                for job in jobs {
                    let egui::epaint::Primitive::Mesh(mesh) = &job.primitive else {
                        continue;
                    };
                    let Some(texture) = painter.texture(mesh.texture_id) else {
                        continue;
                    };
                    let clip = job.clip_rect.intersect(region);
                    let x = ((clip.left() * density).round() as i32).max(left);
                    let y = ((clip.top() * density).round() as i32).max(top);
                    let r = ((clip.right() * density).round() as i32).min(right);
                    let b = ((clip.bottom() * density).round() as i32).min(bottom);
                    if r <= x || b <= y {
                        continue;
                    }
                    gl.scissor(x - left, y - top, r - x, b - y);
                    self.mesh(gl, mesh, texture);
                }
                gl.bind_framebuffer(glow::FRAMEBUFFER, painter.intermediate_fbo());
                gl.viewport(0, 0, sw, sh);
                self.setup(gl, &info, false);
                gl.scissor(left, sh - bottom, width, height);
                let rect = egui::Rect::from_min_max(
                    egui::pos2(left as f32 / density, top as f32 / density),
                    egui::pos2(right as f32 / density, bottom as f32 / density),
                );
                let mut mesh = egui::Mesh::default();
                mesh.add_rect_with_uv(
                    rect,
                    egui::Rect::from_min_max(
                        egui::Pos2::ZERO,
                        egui::pos2(
                            width as f32 / self.tile as f32,
                            height as f32 / self.tile as f32,
                        ),
                    ),
                    egui::Color32::WHITE.gamma_multiply(opacity),
                );
                self.mesh(gl, &mesh, self.resources.texture.unwrap());
            }
            gl.clear_color(clear[0], clear[1], clear[2], clear[3]);
            // egui restores its program, VAO, buffers, viewport, blending and
            // scissor state after callbacks; its public FBO is restored above.
        }
    }
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
                gpu.resources.destroy(gl);
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
        let owner = painter
            .ctx()
            .data(|d| d.get_temp::<Arc<Self>>(egui::Id::new(Self::KEY)));
        let usable = owner.as_ref().is_some_and(|c| {
            let state = c.0.lock().unwrap();
            state.gpu.is_some()
        });
        if !usable {
            // Headless painting and explicitly reported unsupported GL contexts
            // retain readable content using the former per-part approximation.
            for job in jobs {
                if let egui::epaint::Primitive::Mesh(mut mesh) = job.primitive {
                    for v in &mut mesh.vertices {
                        v.color = v.color.gamma_multiply(opacity);
                    }
                    painter
                        .with_clip_rect(job.clip_rect)
                        .add(egui::Shape::mesh(mesh));
                }
            }
            return;
        }
        let region = jobs
            .iter()
            .filter_map(|job| match &job.primitive {
                egui::epaint::Primitive::Mesh(mesh) => {
                    Some(mesh.calc_bounds().intersect(job.clip_rect))
                }
                _ => None,
            })
            .fold(egui::Rect::NOTHING, egui::Rect::union)
            .intersect(painter.clip_rect());
        if !region.is_positive() {
            return;
        }
        let owner = owner.unwrap();
        painter.add(egui::Shape::Callback(egui::PaintCallback {
            rect: region,
            callback: Arc::new(eframe::egui_glow::CallbackFn::new(move |info, p| {
                let state = owner.0.lock().unwrap();
                if let Some(gpu) = &state.gpu {
                    unsafe {
                        gpu.paint(info, p, &jobs, opacity);
                    }
                }
            })),
        }));
    }
}
#[cfg(test)]
mod tests {
    use super::*;
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
