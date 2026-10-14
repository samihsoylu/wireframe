// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! A transparent, piecewise-linear bottom mask shared by SVG and canvas meshes.
use crate::model::Bounds;
use eframe::egui::{
    self, Color32, Pos2, Vec2,
    epaint::{Mesh, Vertex},
};
#[derive(Clone, Debug)]
pub struct BottomMask {
    pub bounds: Bounds,
    pub edge: Vec<[f32; 2]>,
}
impl BottomMask {
    pub fn polygon(&self) -> Vec<[f32; 2]> {
        let b = self.bounds;
        let mut p = vec![[b.x, b.y], [b.x + b.w, b.y]];
        p.extend(self.edge.iter().rev().copied());
        p
    }
    /// Split the mask at every edge segment. The resulting convex cells do not
    /// overlap, so clipping each source triangle preserves opacity and UVs.
    pub fn clip_mesh(&self, mesh: &Mesh, origin: Pos2, scale: f32) -> Mesh {
        let mut out = Mesh::with_texture(mesh.texture_id);
        let top = origin.y + self.bounds.y * scale;
        for tri in mesh.indices.as_chunks::<3>().0 {
            let input = [
                mesh.vertices[tri[0] as usize],
                mesh.vertices[tri[1] as usize],
                mesh.vertices[tri[2] as usize],
            ];
            let minx = input.iter().map(|v| v.pos.x).fold(f32::INFINITY, f32::min);
            let maxx = input
                .iter()
                .map(|v| v.pos.x)
                .fold(f32::NEG_INFINITY, f32::max);
            for pair in self.edge.windows(2) {
                let a = origin + Vec2::new(pair[0][0], pair[0][1]) * scale;
                let b = origin + Vec2::new(pair[1][0], pair[1][1]) * scale;
                if b.x <= a.x || maxx <= a.x || minx >= b.x {
                    continue;
                }
                let slope = (b.y - a.y) / (b.x - a.x);
                let mut vertices = input.to_vec();
                for (nx, ny, c) in [
                    (1., 0., -a.x),
                    (-1., 0., b.x),
                    (0., 1., -top),
                    (slope, -1., a.y - slope * a.x),
                ] {
                    vertices = clip_plane(&vertices, nx, ny, c);
                    if vertices.len() < 3 {
                        break;
                    }
                }
                if vertices.len() < 3 {
                    continue;
                }
                let base = out.vertices.len() as u32;
                out.vertices.extend_from_slice(&vertices);
                for i in 1..vertices.len() - 1 {
                    out.indices
                        .extend([base, base + i as u32, base + i as u32 + 1]);
                }
            }
        }
        out
    }
}
fn clip_plane(vertices: &[Vertex], nx: f32, ny: f32, c: f32) -> Vec<Vertex> {
    let mut result = Vec::new();
    let Some(mut previous) = vertices.last().copied() else {
        return result;
    };
    let distance = |v: Vertex| nx * v.pos.x + ny * v.pos.y + c;
    let mut dp = distance(previous);
    for &current in vertices {
        let dc = distance(current);
        if (dp >= 0.) != (dc >= 0.) {
            let t = dp / (dp - dc);
            let a = previous.color.to_array();
            let b = current.color.to_array();
            let rgba = std::array::from_fn::<_, 4, _>(|i| {
                (a[i] as f32 + (b[i] as f32 - a[i] as f32) * t)
                    .round()
                    .clamp(0., 255.) as u8
            });
            result.push(Vertex {
                pos: previous.pos + (current.pos - previous.pos) * t,
                uv: previous.uv + (current.uv - previous.uv) * t,
                color: Color32::from_rgba_premultiplied(rgba[0], rgba[1], rgba[2], rgba[3]),
            });
        }
        if dc >= 0. {
            result.push(current);
        }
        previous = current;
        dp = dc;
    }
    result
}
/// Replace only the newly painted shapes in this layer with clipped meshes.
/// Existing shapes, scissor rectangles, texture coordinates and ordering remain.
pub fn finish(painter: &egui::Painter, start: usize, mask: &BottomMask, origin: Pos2, scale: f32) {
    let id = painter.layer_id();
    let shapes = painter.ctx().graphics_mut(|g| {
        let Some(list) = g.get_mut(id) else {
            return vec![];
        };
        let entries: Vec<_> = list.all_entries().skip(start).cloned().collect();
        for i in start..start + entries.len() {
            list.reset_shape(egui::layers::ShapeIdx(i));
        }
        entries
    });
    let primitives = painter
        .ctx()
        .tessellate(shapes, painter.ctx().pixels_per_point());
    painter.ctx().graphics_mut(|g| {
        let Some(list) = g.get_mut(id) else { return };
        for p in primitives {
            if let egui::epaint::Primitive::Mesh(mesh) = p.primitive {
                let mesh = mask.clip_mesh(&mesh, origin, scale);
                if !mesh.indices.is_empty() {
                    list.add(p.clip_rect, egui::Shape::mesh(mesh));
                }
            }
        }
    });
}
