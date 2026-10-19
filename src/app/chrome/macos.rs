// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Resolve native accent colors without changing system preferences.
use super::{Color32, ToolbarPalette};
use objc2_app_kit::{NSColor, NSColorSpace};
use objc2_foundation::{MainThreadMarker, NSString, NSUserDefaults};

pub(super) fn toolbar_palette() -> Option<ToolbarPalette> {
    let _main_thread = MainThreadMarker::new()?;
    // Without an explicit system accent, use the product's portable palette.
    // Resolve on each paint so a changed preference is not cached indefinitely.
    NSUserDefaults::standardUserDefaults().objectForKey(&NSString::from_str("AppleAccentColor"))?;
    let selected = NSColor::controlAccentColor();
    let device = selected.colorUsingColorSpace(&NSColorSpace::deviceRGBColorSpace())?;
    let pressed = NSColor::colorWithCalibratedHue_saturation_brightness_alpha(
        device.hueComponent(),
        (device.saturationComponent() - 0.25).clamp(0.0, 1.0),
        (device.brightnessComponent() + 0.1).clamp(0.0, 1.0),
        1.0,
    );
    Some(ToolbarPalette {
        selected: srgb(&selected)?,
        pressed: srgb(&pressed)?,
    })
}

fn srgb(color: &NSColor) -> Option<Color32> {
    let color = color.colorUsingColorSpace(&NSColorSpace::sRGBColorSpace())?;
    let byte = |value: f64| (value.clamp(0.0, 1.0) * 255.0).round() as u8;
    Some(Color32::from_rgb(
        byte(color.redComponent()),
        byte(color.greenComponent()),
        byte(color.blueComponent()),
    ))
}
