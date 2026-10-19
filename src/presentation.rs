// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Presentation preferences and layout, independent of document edit history.
mod framing;
mod preferences;
use crate::model::Id;
pub use framing::{Framing, PageExtent, centered_view};
pub use preferences::Preferences;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Preset {
    #[default]
    Review,
    UserTest,
    Custom,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Zoom {
    #[default]
    Fit,
    ActualSize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Aid {
    Navigator,
    Notes,
    Comments,
    LinkHints,
    BigCursor,
    Markup,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub preset: Preset,
    pub navigator: bool,
    pub notes: bool,
    pub comments: bool,
    pub link_hints: bool,
    pub big_cursor: bool,
    pub markup: bool,
    pub zoom: Zoom,
    pub navigator_width: f32,
    pub side_width: f32,
    pub notes_height: f32,
    /// Stable surface IDs distinguish official pages and their alternate versions.
    pub excluded: BTreeSet<Id>,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            preset: Preset::Review,
            navigator: true,
            notes: true,
            comments: true,
            link_hints: true,
            big_cursor: true,
            markup: true,
            zoom: Zoom::Fit,
            navigator_width: 200.,
            side_width: 300.,
            notes_height: 300.,
            excluded: BTreeSet::new(),
        }
    }
}
impl Settings {
    pub fn aid(&self, aid: Aid) -> bool {
        match aid {
            Aid::Navigator => self.navigator,
            Aid::Notes => self.notes,
            Aid::Comments => self.comments,
            Aid::LinkHints => self.link_hints,
            Aid::BigCursor => self.big_cursor,
            Aid::Markup => self.markup,
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        if [self.navigator_width, self.side_width, self.notes_height]
            .into_iter()
            .any(|value| !value.is_finite() || !(1. ..=10_000.).contains(&value))
        {
            return Err("Presentation panel dimensions are invalid".into());
        }
        if self.excluded.contains(&0) || self.excluded.len() > 50_000 {
            return Err("Presentation centering choices are invalid".into());
        }
        Ok(())
    }
}

/// One presentation controller. Only the current settings are persisted;
/// historical Custom recall belongs to this controller's lifetime.
#[derive(Clone, Debug)]
pub struct Session {
    current: Settings,
    last_custom: Option<Settings>,
}
impl Default for Session {
    fn default() -> Self {
        Self::new(Settings::default()).expect("default presentation settings")
    }
}
impl Session {
    pub fn new(current: Settings) -> Result<Self, String> {
        current.validate()?;
        let last_custom = (current.preset == Preset::Custom).then(|| current.clone());
        Ok(Self {
            current,
            last_custom,
        })
    }
    pub fn settings(&self) -> &Settings {
        &self.current
    }
    pub fn select(&mut self, preset: Preset) -> bool {
        if self.current.preset == preset {
            return false;
        }
        match preset {
            Preset::Custom => {
                if let Some(custom) = &self.last_custom {
                    self.current = custom.clone();
                }
            }
            Preset::Review | Preset::UserTest => {
                let enabled = preset == Preset::Review;
                self.current.navigator = enabled;
                self.current.notes = enabled;
                self.current.comments = enabled;
                self.current.link_hints = enabled;
                self.current.big_cursor = enabled;
                self.current.markup = enabled;
                self.current.zoom = if enabled { Zoom::Fit } else { Zoom::ActualSize };
            }
        }
        self.current.preset = preset;
        true
    }
    fn remember_custom(&mut self) {
        self.current.preset = Preset::Custom;
        self.last_custom = Some(self.current.clone());
    }
    pub fn set_aid(&mut self, aid: Aid, enabled: bool) {
        match aid {
            Aid::Navigator => self.current.navigator = enabled,
            Aid::Notes => self.current.notes = enabled,
            Aid::Comments => self.current.comments = enabled,
            Aid::LinkHints => self.current.link_hints = enabled,
            Aid::BigCursor => self.current.big_cursor = enabled,
            Aid::Markup => self.current.markup = enabled,
        }
        self.remember_custom();
    }
    pub fn toggle(&mut self, aid: Aid) {
        self.set_aid(aid, !self.current.aid(aid));
    }
    pub fn set_zoom(&mut self, zoom: Zoom) {
        self.current.zoom = zoom;
        self.remember_custom();
    }
    pub fn toggle_zoom(&mut self) {
        self.set_zoom(match self.current.zoom {
            Zoom::Fit => Zoom::ActualSize,
            Zoom::ActualSize => Zoom::Fit,
        });
    }
    pub fn set_panel_sizes(&mut self, navigator: f32, side: f32, notes: f32) -> Result<(), String> {
        let mut next = self.current.clone();
        next.navigator_width = navigator;
        next.side_width = side;
        next.notes_height = notes;
        next.validate()?;
        self.current = next;
        self.remember_custom();
        Ok(())
    }
    /// Centering changes do not select Custom or update its remembered snapshot.
    pub fn set_excluded(&mut self, excluded: BTreeSet<Id>) -> Result<(), String> {
        let mut next = self.current.clone();
        next.excluded = excluded;
        next.validate()?;
        self.current = next;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
