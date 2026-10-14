// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Color and alpha remain separate; source bytes are never changed.
pub(super) fn apply(image: &mut image::RgbaImage, color: [u8; 3]) {
    let linear: [f32; 256] = std::array::from_fn(|i| {
        let s = i as f32 / 255.;
        if s <= 0.04045 {
            s / 12.92
        } else {
            ((s + 0.055) / 1.055).powf(2.4)
        }
    });
    let tint = color.map(|c| c as f32 / 255.);
    for p in image.pixels_mut() {
        let alpha = p[3] as f32 / 255.;
        if p[3] == 0 {
            p.0 = [0; 4];
            continue;
        }
        for channel in 0..3 {
            let value = linear[p[channel] as usize] * (1. - tint[channel]) + alpha * tint[channel];
            let encoded = if value <= 0.0031308 {
                12.92 * value
            } else {
                1.055 * value.powf(1. / 2.4) - 0.055
            };
            p[channel] = (encoded.clamp(0., 1.) * 255.).round() as u8;
        }
    }
}
