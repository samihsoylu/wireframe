// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Decorative keyboard geometry, independent of its drawing style.
use crate::{
    document::{Properties, Value},
    model::Bounds,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Options {
    pub tablet: bool,
    pub landscape: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    Shift,
    Backspace,
    Globe,
    Microphone,
    Dismiss,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Legend {
    Text(&'static str),
    Icon(Icon),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Surface {
    Letter,
    GrayLetter,
    Mode,
    Space,
    Return,
}
impl Surface {
    pub fn size(self, sketch: bool) -> (f32, f32) {
        match self {
            Self::Letter | Self::GrayLetter => (23., 29.),
            Self::Mode => (33., 29.),
            Self::Space => (96., 29.),
            Self::Return => (47., if sketch { 28. } else { 27. }),
        }
    }
    pub fn light(self) -> bool {
        matches!(self, Self::Letter | Self::Space)
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Key {
    pub bounds: Bounds,
    pub legend: Legend,
    pub surface: Surface,
    pub font_size: f32,
    pub icon_size: f32,
}
#[derive(Debug)]
pub struct Layout {
    pub frame: Bounds,
    pub scale: f32,
    pub keys: Vec<Key>,
}
const ROWS: [&[&str]; 3] = [
    &["Q", "W", "E", "R", "T", "Y", "U", "I", "O", "P"],
    &["A", "S", "D", "F", "G", "H", "J", "K", "L"],
    &["Z", "X", "C", "V", "B", "N", "M"],
];
impl Options {
    pub fn from_properties(p: &Properties) -> Self {
        Self {
            tablet: p.get("keyboard_device") == Some(&Value::Choice("tablet".into())),
            landscape: p.get("orientation") == Some(&Value::Choice("landscape".into())),
        }
    }
    pub fn native(self) -> (f32, f32) {
        match (self.tablet, self.landscape) {
            (false, false) => (226., 160.),
            (false, true) => (403., 114.),
            (true, false) => (546., 188.),
            (true, true) => (728., 251.),
        }
    }
    pub fn layout(self, w: f32, h: f32) -> Layout {
        let (nw, nh) = self.native();
        let scale = if w.is_finite() && h.is_finite() {
            (w.max(0.) / nw).min(h.max(0.) / nh)
        } else {
            0.
        };
        let frame = Bounds::new(
            if w.is_finite() {
                (w.max(0.) - nw * scale) / 2.
            } else {
                0.
            },
            if h.is_finite() {
                (h.max(0.) - nh * scale) / 2.
            } else {
                0.
            },
            nw * scale,
            nh * scale,
        );
        let mut keys = if self.tablet {
            self.tablet_keys()
        } else {
            self.phone_keys()
        };
        for key in &mut keys {
            let b = key.bounds;
            key.bounds = Bounds::new(
                frame.x + b.x * scale,
                frame.y + b.y * scale,
                b.w * scale,
                b.h * scale,
            );
            key.font_size *= scale;
            key.icon_size *= scale;
        }
        Layout { frame, scale, keys }
    }
    fn phone_keys(self) -> Vec<Key> {
        use {self::Icon::*, Legend::*, Surface::*};
        let (kw, big, gap, pitch, start, second, third, last, space, ret) = if self.landscape {
            (33., 40., 4., 28., 17., 34., 70., 342., 170., 71.)
        } else {
            (21., 31., 1., 38., 3., 15., 35., 190., 95., 47.)
        };
        let y = if self.landscape { 1. } else { 11. };
        let mut b = Builder::new(27., 13., 16.);
        b.row(ROWS[0], start, y, kw, gap);
        b.row(ROWS[1], second, y + pitch, kw, gap);
        b.key(Icon(Shift), start, y + 2. * pitch, big, Mode);
        b.row(ROWS[2], third, y + 2. * pitch, kw, gap);
        b.key(Icon(Backspace), last, y + 2. * pitch, big, Mode);
        let gap = if self.landscape { 3. } else { 1. };
        let globe = if self.landscape { big } else { kw };
        let mut x = start;
        let y = y + 3. * pitch;
        for (legend, width, surface) in [
            (Text("123"), big, Mode),
            (Icon(Globe), globe, GrayLetter),
            (Icon(Microphone), kw, Letter),
            (Text("space"), space, Space),
            (Text("return"), ret, Return),
        ] {
            b.key(legend, x, y, width, surface);
            x += width + gap;
        }
        b.keys
    }
    fn tablet_keys(self) -> Vec<Key> {
        use {self::Icon::*, Legend::*, Surface::*};
        let (kw, kh, gap, pitch, start, second, ret, space, font, factor) = if self.landscape {
            (60., 47., 5., 63., 8., 36., 98., 349., 20., 1.3)
        } else {
            (45., 41., 4., 46., 4., 21., 77., 255., 18., 1.4)
        };
        let mut b = Builder::new(kh, font, 24.);
        b.row(ROWS[0], start, start, kw, gap);
        b.key(
            Icon(Backspace),
            start + 10. * (kw + gap),
            start,
            kw,
            GrayLetter,
        );
        b.row(ROWS[1], second, start + pitch, kw, gap);
        b.key(
            Text("return"),
            second + 9. * (kw + gap),
            start + pitch,
            ret,
            Mode,
        );
        let y = start + 2. * pitch;
        b.key(Icon(Shift), start, y, kw, Mode);
        b.row(ROWS[2], start + kw + gap, y, kw, gap);
        b.font = font - 4.;
        b.key(Text("!\n,"), start + 8. * (kw + gap), y, kw, Letter);
        b.key(Text("?\n."), start + 9. * (kw + gap), y, kw, Letter);
        b.key(Icon(Shift), start + 10. * (kw + gap), y, kw, GrayLetter);
        let mut x = start;
        let y = start + 3. * pitch;
        for (legend, width, surface) in [
            (Text(".?123"), kw * factor, Mode),
            (Icon(Globe), kw, GrayLetter),
            (Icon(Microphone), kw, GrayLetter),
            (Text(" "), space, Space),
            (Text(".?123"), kw * factor, Mode),
            (Icon(Dismiss), kw, GrayLetter),
        ] {
            b.font = if matches!(legend, Text(".?123")) {
                font - 4.
            } else {
                font
            };
            b.key(legend, x, y, width, surface);
            x += width + gap;
        }
        b.keys
    }
}
struct Builder {
    keys: Vec<Key>,
    height: f32,
    font: f32,
    icon: f32,
}
impl Builder {
    fn new(height: f32, font: f32, icon: f32) -> Self {
        Self {
            keys: Vec::new(),
            height,
            font,
            icon,
        }
    }
    fn key(&mut self, legend: Legend, x: f32, y: f32, width: f32, surface: Surface) {
        self.keys.push(Key {
            bounds: Bounds::new(x, y, width, self.height),
            legend,
            surface,
            font_size: self.font,
            icon_size: self.icon,
        });
    }
    fn row(&mut self, letters: &[&'static str], x: f32, y: f32, width: f32, gap: f32) {
        for (i, letter) in letters.iter().enumerate() {
            self.key(
                Legend::Text(letter),
                x + i as f32 * (width + gap),
                y,
                width,
                Surface::Letter,
            );
        }
    }
}
