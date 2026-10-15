// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Original video artwork, cropped and transformed through fixed edge regions.
use super::{media_controls::Art, *};
impl Scene {
    pub(super) fn video(&mut self, layer: &Layer, sketch: bool) {
        let b = layer.control.bounds;
        let l = crate::video::layout(b.w, b.h, sketch);
        let clip = clip_with(layer.clip, b);
        let mut a = Art {
            scene: self,
            origin: [b.x, b.y],
            scale: [1., 1.],
            clip,
        };
        let bg = l.background;
        a.polygon(
            vec![
                [bg.x, bg.y],
                [bg.x + bg.w, bg.y],
                [bg.x + bg.w, bg.y + bg.h],
                [bg.x, bg.y + bg.h],
            ],
            [255; 3],
        );
        for region in l.regions {
            // The center is blank; its white fill belongs to the whole control.
            if region.row == 1 && region.column == 1 {
                continue;
            }
            let s = region.source;
            let d = region.destination;
            let scale = [d.w / s.w, d.h / s.h];
            let destination = Bounds::new(b.x + d.x, b.y + d.y, d.w, d.h);
            let mut a = Art {
                scene: self,
                origin: [
                    destination.x - s.x * scale[0],
                    destination.y - s.y * scale[1],
                ],
                scale,
                clip: clip_with(clip, destination),
            };
            artwork(&mut a, sketch, l.ink, l.progress);
        }
    }
}
fn artwork(a: &mut Art<'_>, sketch: bool, ink: [u8; 3], progress: [u8; 3]) {
    let white = [255; 3];
    let stroke = if sketch { 1.8 } else { 1. };
    a.line(
        if sketch {
            vec![[1., 1.], [299., 1.5], [298.8, 199.], [1.2, 198.5], [1., 1.]]
        } else {
            vec![
                [0.5, 0.5],
                [299.5, 0.5],
                [299.5, 199.5],
                [0.5, 199.5],
                [0.5, 0.5],
            ]
        },
        ink,
        stroke,
    );
    let divider = if sketch { 163. } else { 161.5 };
    a.line(vec![[0.5, divider], [299.5, divider]], ink, stroke);
    let (cx, cy, r) = if sketch {
        (20., 180., 13.)
    } else {
        (19.5, 179.5, 10.5)
    };
    a.ellipse(
        Bounds::new(cx - r, cy - r, 2. * r, 2. * r),
        if sketch { white } else { ink },
        sketch.then_some((ink, 1.8)),
    );
    let triangle = vec![[cx - 3., cy - 5.], [cx + 5., cy], [cx - 3., cy + 5.]];
    if sketch {
        let mut p = triangle;
        p.push(p[0]);
        a.line(p, ink, 1.5);
    } else {
        a.polygon(triangle, white);
    }
    let (left, fill_end, top, bottom) = if sketch {
        (42., 146., 175., 185.)
    } else {
        (41., 137., 174.5, 185.5)
    };
    a.polygon(
        vec![
            [left, top],
            [fill_end, top],
            [fill_end, bottom],
            [left, bottom],
        ],
        progress,
    );
    a.line(
        vec![
            [left, top],
            [230., top],
            [230., bottom],
            [left, bottom],
            [left, top],
        ],
        ink,
        if sketch { 1.8 } else { 1. },
    );
    let speaker_fill = if sketch { white } else { ink };
    a.polygon(
        vec![[239., 177.], [243., 177.], [243., 184.], [239., 184.]],
        speaker_fill,
    );
    a.polygon(
        vec![[243., 177.], [249., 171.], [249., 190.], [243., 184.]],
        speaker_fill,
    );
    if sketch {
        a.line(
            vec![
                [239., 177.],
                [243., 177.],
                [249., 171.],
                [249., 190.],
                [243., 184.],
                [239., 184.],
                [239., 177.],
            ],
            ink,
            1.7,
        );
    }
    for radius in [4., 8.] {
        a.line(
            (0..16)
                .map(|i| {
                    let t = -0.8 + i as f32 * 1.6 / 15.;
                    [249.5 + radius * t.cos(), 180.5 + radius * t.sin()]
                })
                .collect(),
            ink,
            if sketch { 1.6 } else { 1.3 },
        );
    }
    if sketch {
        a.line(
            vec![
                [276., 176.],
                [285., 176.],
                [285., 185.],
                [276., 185.],
                [276., 176.],
            ],
            ink,
            1.6,
        );
        for (x, y, dx, dy) in [
            (270., 171., -1., -1.),
            (290., 171., 1., -1.),
            (270., 190., -1., 1.),
            (290., 190., 1., 1.),
        ] {
            a.line(vec![[x - dx * 2., y - dy * 2.], [x, y]], ink, 1.6);
            a.polygon(
                vec![
                    [x + dx * 2., y + dy * 2.],
                    [x - dx * 2., y + dy * 2.],
                    [x + dx * 2., y - dy * 2.],
                ],
                ink,
            );
        }
    } else {
        a.polygon(
            vec![[264., 171.], [287., 171.], [287., 190.], [264., 190.]],
            ink,
        );
        a.polygon(
            vec![[266., 180.], [277., 180.], [277., 188.], [266., 188.]],
            white,
        );
        a.line(vec![[279., 178.], [284., 173.], [279., 173.]], white, 1.2);
        a.line(vec![[284., 173.], [284., 178.]], white, 1.2);
    }
}
