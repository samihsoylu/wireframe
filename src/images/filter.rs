// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Grayscale edge enhancement with a two-pixel transparent extent.
use image::RgbaImage;

pub(super) fn sketch(input: &RgbaImage) -> Result<RgbaImage, String> {
    let (w, h) = input.dimensions();
    if u64::from(w + 4) * u64::from(h + 4) > super::MAX_PIXELS {
        return Err("Filtered image exceeds the pixel limit".into());
    }
    let linear: [f32; 256] = std::array::from_fn(|i| {
        let v = i as f32 / 255.;
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    });
    // Only five source rows are needed; avoid a second full floating-point image.
    let mut rows = vec![vec![[0f32; 2]; w as usize]; 5];
    let mut output = RgbaImage::new(w + 4, h + 4);
    for y in 0..h + 4 {
        let row = &mut rows[y as usize % 5];
        row.fill([0., 0.]);
        if y < h {
            for x in 0..w {
                let p = input.get_pixel(x, y).0;
                let value = linear[p[0] as usize] * 0.212671
                    + linear[p[1] as usize] * 0.71516
                    + linear[p[2] as usize] * 0.072169;
                let alpha = f32::from(p[3]) / 255.;
                row[x as usize] = [value * alpha, alpha];
            }
        }
        let sample = |x: i32, y: i32| {
            if x < 0 || y < 0 || x >= w as i32 || y >= h as i32 {
                [0., 0.]
            } else {
                rows[y as usize % 5][x as usize]
            }
        };
        for x in 0..w + 4 {
            let cx = x as i32 - 2;
            let cy = y as i32 - 2;
            let center = sample(cx, cy);
            let mut color = center[0] * 10.;
            let mut alpha = center[1] * 10.;
            for d in [1, 2] {
                for (dx, dy) in [(-d, -d), (d, -d), (-d, d), (d, d)] {
                    let p = sample(cx + dx, cy + dy);
                    color -= p[0];
                    alpha -= p[1];
                }
            }
            // Convert the unpremultiplied linear result before clamping alpha.
            // The kernel can amplify alpha beyond one at opaque edges/interiors.
            let coverage = alpha.clamp(0., 1.);
            let gray = if alpha > 0. {
                let linear = (color / alpha).max(0.);
                let encoded = if linear <= 0.0031308 {
                    12.92 * linear
                } else {
                    1.055 * linear.powf(1. / 2.4) - 0.055
                };
                (encoded * alpha / coverage).clamp(0., 1.)
            } else {
                0.
            };
            let gray = (gray * 255.).round() as u8;
            output.put_pixel(
                x,
                y,
                image::Rgba([gray, gray, gray, (coverage * 255.).round() as u8]),
            );
        }
    }
    Ok(output)
}
