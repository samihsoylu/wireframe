// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Catalog choices are creation recipes, not new saved control kinds.
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CatalogPreset {
    AndroidAlert,
    AndroidHorizontalSlider,
    IconList,
    AndroidProgress,
    AndroidSearch,
    AndroidTextInput,
    AndroidVerticalSlider,
    IosPointedButton,
}
impl CatalogPreset {
    pub const ALL: [Self; 8] = [
        Self::AndroidAlert,
        Self::AndroidHorizontalSlider,
        Self::IconList,
        Self::AndroidProgress,
        Self::AndroidSearch,
        Self::AndroidTextInput,
        Self::AndroidVerticalSlider,
        Self::IosPointedButton,
    ];
    pub fn kind(self) -> Kind {
        match self {
            Self::AndroidAlert => Kind::Alert,
            Self::AndroidHorizontalSlider => Kind::HorizontalSlider,
            Self::IconList => Kind::List,
            Self::AndroidProgress => Kind::Progress,
            Self::AndroidSearch => Kind::Search,
            Self::AndroidTextInput => Kind::TextInput,
            Self::AndroidVerticalSlider => Kind::VerticalSlider,
            Self::IosPointedButton => Kind::PointedButton,
        }
    }
    pub fn id(self) -> &'static str {
        match self {
            Self::AndroidAlert => "android_alert",
            Self::AndroidHorizontalSlider => "android_horizontal_slider",
            Self::IconList => "icon_list",
            Self::AndroidProgress => "android_progress",
            Self::AndroidSearch => "android_search",
            Self::AndroidTextInput => "android_text_input",
            Self::AndroidVerticalSlider => "android_vertical_slider",
            Self::IosPointedButton => "ios_pointed_button",
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::AndroidAlert => "Alert dialog (Android)",
            Self::AndroidHorizontalSlider => "Horizontal slider (Android)",
            Self::IconList => "List with icons",
            Self::AndroidProgress => "Progress bar (Android)",
            Self::AndroidSearch => "Search (Android)",
            Self::AndroidTextInput => "Text input (Android)",
            Self::AndroidVerticalSlider => "Vertical slider (Android)",
            Self::IosPointedButton => "Pointed button (iOS)",
        }
    }
    pub(super) fn apply(self, node: &mut Node) {
        let p = &mut node.properties;
        let choice = |s: &str| Value::Choice(s.into());
        match self {
            Self::AndroidAlert => {
                p.insert("button_style".into(), choice("inline"));
                p.insert("align".into(), choice("left"));
            }
            Self::AndroidHorizontalSlider | Self::AndroidVerticalSlider => {
                p.insert("slider_thumb".into(), choice("circle"));
            }
            Self::IconList => {
                p.insert("border".into(), choice("none"));
                p.insert(
                    "row_icons".into(),
                    Value::Icons(
                        ["star-solid", "heart-solid", "circle-solid"]
                            .map(|id| Some(IconSource::Catalog(id.into())))
                            .to_vec(),
                    ),
                );
            }
            Self::AndroidProgress => {
                p.insert("progress_style".into(), choice("thin"));
            }
            Self::AndroidSearch => {
                p.insert("shape".into(), choice("rectangle"));
                p.insert("microphone_icon".into(), Value::Bool(true));
            }
            Self::AndroidTextInput => {
                p.insert("border".into(), choice("line"));
            }
            Self::IosPointedButton => {
                p.insert("border".into(), choice("solid"));
            }
        }
    }
}
