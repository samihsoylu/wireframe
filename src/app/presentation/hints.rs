// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use eframe::egui::Color32;

/// Temporary presentation feedback, independent of saved aid settings and Undo.
#[derive(Default)]
pub(super) struct Hints {
    held: bool,
    revealed: Option<f64>,
    fading: Option<f64>,
}

pub(super) struct Frame {
    color_progress: f32,
    opacity: f32,
    pub animating: bool,
}

impl Hints {
    pub fn update(
        &mut self,
        enabled: bool,
        alt: bool,
        blank_press: bool,
        now: f64,
    ) -> Option<Frame> {
        if !enabled {
            *self = Self::default();
            return None;
        }
        if alt && !self.held {
            self.revealed = Some(now);
            self.fading = None;
        } else if !alt && self.held {
            self.fading = Some(now);
        } else if !alt && blank_press {
            self.revealed = Some(now);
            self.fading = Some(now);
        }
        self.held = alt;
        let revealed = self.revealed?;
        let opacity =
            self.fading
                .map_or(1., |start| (1. - (now - start) / 0.4).clamp(0., 1.)) as f32;
        if opacity <= 0. {
            self.revealed = None;
            self.fading = None;
            return None;
        }
        // Brief white feedback precedes the linear transition to the link color.
        let color_progress = ((now - revealed - 0.04) / 0.04).clamp(0., 1.) as f32;
        Some(Frame {
            color_progress,
            opacity,
            animating: color_progress < 1. || self.fading.is_some(),
        })
    }
}

impl Frame {
    pub fn color(&self, broken: bool) -> Color32 {
        let target = if broken {
            [224., 102., 102.]
        } else {
            [61., 155., 232.]
        };
        let channel = |to: f32| (255. + (to - 255.) * self.color_progress).round() as u8;
        let alpha = ((0.3 + 0.5 * self.color_progress) * self.opacity * 255.).round() as u8;
        Color32::from_rgba_unmultiplied(
            channel(target[0]),
            channel(target[1]),
            channel(target[2]),
            alpha,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn resting_and_disabled_aid_produce_no_overlay_or_animation() {
        let mut hints = Hints::default();
        assert!(hints.update(true, false, false, 0.).is_none());
        assert!(hints.update(false, true, true, 1.).is_none());
        assert!(hints.update(true, false, false, 2.).is_none());
        hints.update(true, true, false, 3.);
        assert!(hints.update(false, true, false, 3.1).is_none());
        assert!(hints.update(true, false, false, 3.2).is_none());
    }
    #[test]
    fn held_reveal_settles_and_release_or_blank_press_fades_to_nothing() {
        let mut hints = Hints::default();
        let initial = hints.update(true, true, false, 1.).unwrap();
        assert_eq!(
            initial.color(false),
            Color32::from_rgba_unmultiplied(255, 255, 255, 77)
        );
        assert!(initial.animating);
        let held = hints.update(true, true, true, 2.).unwrap();
        assert_eq!(
            held.color(false),
            Color32::from_rgba_unmultiplied(61, 155, 232, 204)
        );
        assert!(
            !held.animating,
            "held Alt must not repaint forever or restart on a blank click"
        );
        hints.update(true, false, false, 3.);
        let fading = hints.update(true, false, false, 3.2).unwrap();
        assert!(fading.animating);
        assert_eq!(
            fading.color(false),
            Color32::from_rgba_unmultiplied(61, 155, 232, 102)
        );
        assert!(hints.update(true, false, false, 3.41).is_none());
        hints.update(true, false, true, 4.);
        assert!(hints.update(true, false, false, 4.2).unwrap().animating);
        assert!(hints.update(true, false, false, 4.41).is_none());
    }
}
