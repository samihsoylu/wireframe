// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Opt-in, windowless macOS snapshots with the production egui OpenGL renderer.
//! No window, native input, user project, or installed application is involved.
use super::*;
use eframe::glow::{self, HasContext};
use std::{
    ffi::{CString, c_char, c_void},
    sync::Arc,
};

#[link(name = "OpenGL", kind = "framework")]
unsafe extern "C" {
    fn CGLChoosePixelFormat(
        attributes: *const u32,
        format: *mut *mut c_void,
        count: *mut i32,
    ) -> i32;
    fn CGLDestroyPixelFormat(format: *mut c_void) -> i32;
    fn CGLCreateContext(format: *mut c_void, share: *mut c_void, context: *mut *mut c_void) -> i32;
    fn CGLSetCurrentContext(context: *mut c_void) -> i32;
    fn CGLGetCurrentContext() -> *mut c_void;
    fn CGLDestroyContext(context: *mut c_void) -> i32;
}
unsafe extern "C" {
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
}
struct Context {
    raw: *mut c_void,
    previous: *mut c_void,
}
impl Context {
    fn new() -> Self {
        unsafe {
            // OpenGL 3.2 core, RGBA, offline renderer allowed. SDK CGLTypes.h.
            let attributes = [99, 0x3200, 8, 24, 11, 8, 96, 0];
            let mut format = std::ptr::null_mut();
            let mut count = 0;
            assert_eq!(
                CGLChoosePixelFormat(attributes.as_ptr(), &mut format, &mut count),
                0
            );
            assert!(!format.is_null() && count > 0);
            let mut raw = std::ptr::null_mut();
            let result = CGLCreateContext(format, std::ptr::null_mut(), &mut raw);
            CGLDestroyPixelFormat(format);
            assert_eq!(result, 0);
            let context = Self {
                raw,
                previous: CGLGetCurrentContext(),
            };
            assert_eq!(CGLSetCurrentContext(raw), 0);
            context
        }
    }
}
impl Drop for Context {
    fn drop(&mut self) {
        unsafe {
            CGLSetCurrentContext(self.previous);
            CGLDestroyContext(self.raw);
        }
    }
}

pub(super) struct Offscreen {
    _context: Context,
    gl: Arc<glow::Context>,
    painter: eframe::egui_glow::Painter,
    framebuffer: glow::Framebuffer,
    texture: glow::Texture,
    size: [u32; 2],
}
impl Offscreen {
    pub fn new() -> Self {
        let context = Context::new();
        unsafe {
            let gl = Arc::new(glow::Context::from_loader_function(|name| {
                let name = CString::new(name).unwrap();
                dlsym((-2_isize) as *mut c_void, name.as_ptr()).cast_const()
            }));
            let painter = eframe::egui_glow::Painter::new(
                gl.clone(),
                "",
                None,
                eframe::NativeOptions::default().dithering,
            )
            .unwrap();
            let framebuffer = gl.create_framebuffer().unwrap();
            let texture = gl.create_texture().unwrap();
            Self {
                _context: context,
                gl,
                painter,
                framebuffer,
                texture,
                size: [0, 0],
            }
        }
    }
    pub fn max_texture_side(&self) -> usize {
        unsafe { self.gl.get_parameter_i32(glow::MAX_TEXTURE_SIZE) as usize }
    }
    pub fn renderer(&self) -> String {
        unsafe {
            format!(
                "{} / {}",
                self.gl.get_parameter_string(glow::RENDERER),
                self.gl.get_parameter_string(glow::VERSION)
            )
        }
    }
    pub fn draw(&mut self, ctx: &egui::Context, output: &mut egui::FullOutput, points: Vec2) {
        let density = output.pixels_per_point;
        let size = [
            (points.x * density).round() as u32,
            (points.y * density).round() as u32,
        ];
        let meshes = ctx.tessellate(output.shapes.clone(), density);
        // Opacity scenes use the same callbacks as a native window. Other
        // callback types would not be supported by this OpenGL painter.
        assert!(meshes.iter().all(|p| match &p.primitive {
            egui::epaint::Primitive::Mesh(_) => true,
            egui::epaint::Primitive::Callback(c) =>
                c.callback.is::<eframe::egui_glow::CallbackFn>(),
        }));
        unsafe {
            self.gl
                .bind_framebuffer(glow::FRAMEBUFFER, Some(self.framebuffer));
            if self.size != size {
                self.gl.bind_texture(glow::TEXTURE_2D, Some(self.texture));
                self.gl.tex_image_2d(
                    glow::TEXTURE_2D,
                    0,
                    glow::RGBA8 as i32,
                    size[0] as i32,
                    size[1] as i32,
                    0,
                    glow::RGBA,
                    glow::UNSIGNED_BYTE,
                    glow::PixelUnpackData::Slice(None),
                );
                self.gl.tex_parameter_i32(
                    glow::TEXTURE_2D,
                    glow::TEXTURE_MIN_FILTER,
                    glow::LINEAR as i32,
                );
                self.gl.tex_parameter_i32(
                    glow::TEXTURE_2D,
                    glow::TEXTURE_MAG_FILTER,
                    glow::LINEAR as i32,
                );
                self.gl.framebuffer_texture_2d(
                    glow::FRAMEBUFFER,
                    glow::COLOR_ATTACHMENT0,
                    glow::TEXTURE_2D,
                    Some(self.texture),
                    0,
                );
                assert_eq!(
                    self.gl.check_framebuffer_status(glow::FRAMEBUFFER),
                    glow::FRAMEBUFFER_COMPLETE
                );
                self.size = size;
            }
        }
        self.painter.clear(size, [1., 1., 1., 1.]);
        self.painter
            .paint_and_update_textures(size, density, &meshes, &mut output.textures_delta);
        unsafe {
            assert_eq!(self.gl.get_error(), glow::NO_ERROR);
        }
    }
    pub fn save(&self, path: &std::path::Path) {
        let [width, height] = self.size;
        let mut pixels = vec![0; width as usize * height as usize * 4];
        unsafe {
            self.gl
                .bind_framebuffer(glow::FRAMEBUFFER, Some(self.framebuffer));
            self.gl.read_pixels(
                0,
                0,
                width as i32,
                height as i32,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelPackData::Slice(Some(&mut pixels)),
            );
            assert_eq!(self.gl.get_error(), glow::NO_ERROR);
        }
        let mut image = image::RgbaImage::from_raw(width, height, pixels).unwrap();
        image::imageops::flip_vertical_in_place(&mut image);
        image.save(path).unwrap();
    }
}
impl Drop for Offscreen {
    fn drop(&mut self) {
        self.painter.destroy();
        unsafe {
            self.gl.delete_framebuffer(self.framebuffer);
            self.gl.delete_texture(self.texture);
        }
    }
}

#[test]
#[ignore = "Opt-in OpenGL compositing; requires WIREFRAME_RENDER_DIR"]
fn nested_opacity_keeps_overlap_masks_clips_and_tile_boundaries() {
    use wireframe::render::{Drawing, Item, Scene, compositor::Compositor, mask::BottomMask};
    let root = std::path::PathBuf::from(std::env::var_os("WIREFRAME_RENDER_DIR").unwrap());
    std::fs::create_dir_all(&root).unwrap();
    let mut gpu = Offscreen::new();
    let item = |drawing| Item {
        drawing,
        opacity: 1.,
        clip: None,
    };
    let rect = |x, y, w, h, fill| {
        item(Drawing::Primitive(Primitive::Rect {
            b: Bounds::new(x, y, w, h),
            fill,
            stroke: false,
            radius: 0.,
        }))
    };
    let mut blue = rect(200., 40., 100., 20., [0, 0, 255]);
    blue.clip = Some(Bounds::new(240., 40., 40., 20.));
    let scene = Scene {
        title: "Nested fading".into(),
        bounds: Bounds::new(0., 0., 1300., 128.),
        links: vec![],
        font: None,
        items: vec![
            item(Drawing::BeginOpacity(0.5)),
            rect(10., 10., 1280., 100., [0; 3]),
            item(Drawing::BeginMask(BottomMask {
                bounds: Bounds::new(0., 0., 1300., 128.),
                edge: vec![[0., 70.], [1300., 70.]],
                keep_above: true,
            })),
            item(Drawing::BeginOpacity(0.5)),
            rect(100., 20., 1100., 60., [255, 0, 0]),
            rect(150., 30., 1100., 60., [255, 0, 0]),
            item(Drawing::BeginOpacity(0.5)),
            blue,
            item(Drawing::EndOpacity),
            item(Drawing::EndOpacity),
            item(Drawing::EndMask),
            item(Drawing::EndOpacity),
            rect(1100., 50., 30., 20., [0, 255, 0]),
        ],
    };
    let mut evidence = Vec::new();
    for density in [1., 2.] {
        let ctx = egui::Context::default();
        let compositor = Compositor::install(&ctx, &gpu.gl).unwrap();
        let size = Vec2::new(1300., 128.);
        let mut tile_boundary = 0;
        for frame in 0..2 {
            let mut input = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
                time: Some(frame as f64 * 0.1),
                max_texture_side: Some(gpu.max_texture_side()),
                ..Default::default()
            };
            input
                .viewports
                .get_mut(&egui::ViewportId::ROOT)
                .unwrap()
                .native_pixels_per_point = Some(density);
            let mut output = ctx.run_ui(input, |ui| {
                let painter = ui.ctx().layer_painter(egui::LayerId::background());
                WireframeApp::paint_scene(&painter, &scene, Pos2::ZERO, 1.);
                painter.rect_filled(
                    Rect::from_min_size(egui::pos2(20., 115.), egui::vec2(1260., 5.)),
                    0.,
                    Color32::BLUE,
                );
            });
            let outer = output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Callback(callback) => Some(callback.rect),
                    _ => None,
                })
                .expect("native opacity callback");
            tile_boundary = (outer.left() * output.pixels_per_point).floor() as u32 + 1024;
            gpu.draw(&ctx, &mut output, size);
        }
        let file = root.join(format!("nested-opacity-{density}x.png"));
        gpu.save(&file);
        let image = image::open(&file).unwrap().into_rgba8();
        let mut samples = Vec::new();
        for (name, x, y, expected) in [
            ("outside", 5, 5, [255, 255, 255]),
            ("outer", 50, 50, [127, 127, 127]),
            ("inner", 120, 40, [191, 127, 127]),
            ("overlap", 180, 40, [191, 127, 127]),
            ("deep", 260, 50, [159, 127, 159]),
            ("clip", 220, 50, [191, 127, 127]),
            ("mask", 180, 80, [127, 127, 127]),
            ("later opaque", 1110, 60, [0, 255, 0]),
            ("later plain", 1050, 117, [0, 0, 255]),
        ] {
            let actual = image
                .get_pixel((x as f32 * density) as u32, (y as f32 * density) as u32)
                .0;
            samples.push(serde_json::json!({"name":name,"actual":actual,"expected":expected}));
            assert!(
                actual[..3]
                    .iter()
                    .zip(expected)
                    .all(|(a, e)| (i32::from(*a) - e).abs() <= 3),
                "{density}x {name}: {actual:?} expected {expected:?}"
            );
            assert_eq!(actual[3], 255);
        }
        for (name, x) in [
            ("before tile", tile_boundary - 1),
            ("after tile", tile_boundary + 1),
        ] {
            let actual = image.get_pixel(x, (40. * density) as u32).0;
            let expected = [191, 127, 127];
            samples.push(
                serde_json::json!({"name":name,"pixel_x":x,"actual":actual,"expected":expected}),
            );
            assert!(
                actual[..3]
                    .iter()
                    .zip(expected)
                    .all(|(a, e)| (i32::from(*a) - e).abs() <= 3),
                "{density}x {name}: {actual:?}"
            );
            assert_eq!(actual[3], 255);
        }
        evidence.push(
            serde_json::json!({"density":density,"tile_boundary_px":tile_boundary,"samples":samples,"renderer":gpu.renderer()}),
        );
        compositor.destroy(&gpu.gl);
    }
    std::fs::write(
        root.join("nested-opacity-pixels.json"),
        serde_json::to_vec_pretty(&evidence).unwrap(),
    )
    .unwrap();
}
