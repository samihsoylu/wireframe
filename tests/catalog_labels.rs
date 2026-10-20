// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{
    model::Kind,
    workspace::{
        CatalogItem::{self, *},
        CatalogPreset, Editor,
    },
};

#[test]
fn built_in_library_labels_and_order_are_complete() {
    let editor = Editor::default();
    let expected = [
        ("Accordion", Control(Kind::Accordion)),
        ("Alert Box", Control(Kind::Alert)),
        ("Alert Box (Android)", Preset(CatalogPreset::AndroidAlert)),
        ("App Bar", Control(Kind::AppBar)),
        ("Arrow", Control(Kind::Arrow)),
        ("Breadcrumbs", Control(Kind::Breadcrumbs)),
        ("Browser Window", Control(Kind::Browser)),
        ("Button", Control(Kind::Button)),
        ("Button Bar", Control(Kind::ButtonBar)),
        ("Calendar", Control(Kind::Calendar)),
        ("Callout", Control(Kind::Callout)),
        ("Chart: Bar", Control(Kind::BarChart)),
        ("Chart: Column", Control(Kind::ColumnChart)),
        ("Chart: Line", Control(Kind::LineChart)),
        ("Chart: Pie", Control(Kind::PieChart)),
        ("Checkbox", Control(Kind::Checkbox)),
        ("Checkbox Group", Control(Kind::CheckboxGroup)),
        ("Circle Button", Control(Kind::CircleButton)),
        ("Color Picker", Control(Kind::ColorPicker)),
        ("ComboBox", Control(Kind::Dropdown)),
        ("Comment", Control(Kind::StickyNote)),
        ("Cover Flow", Control(Kind::CoverFlow)),
        ("Data Grid", Control(Kind::Table)),
        ("Date Chooser", Control(Kind::DateChooser)),
        ("Date Picker", Control(Kind::DatePicker)),
        ("Field Set", Control(Kind::FieldSet)),
        ("H.Curly Brace", Control(Kind::HorizontalBrace)),
        ("H.Rule", Control(Kind::Line)),
        ("H.Scroll Bar", Control(Kind::HorizontalScrollbar)),
        ("H.Slider", Control(Kind::HorizontalSlider)),
        (
            "H.Slider (Android)",
            Preset(CatalogPreset::AndroidHorizontalSlider),
        ),
        ("H.Splitter", Control(Kind::HorizontalSplitter)),
        ("Help Button", Control(Kind::HelpButton)),
        ("Icon", Control(Kind::Icon)),
        ("Icon and Text Label", Control(Kind::IconLabel)),
        ("Image", Control(Kind::Image)),
        ("iOS Keyboard", Control(Kind::MobileKeyboard)),
        ("iOS Menu", Control(Kind::MobileMenu)),
        ("iOS Picker", Control(Kind::MobilePicker)),
        ("iPad", Control(Kind::Tablet)),
        ("iPhone", Control(Kind::Phone)),
        ("Link", Control(Kind::Link)),
        ("Link Bar", Control(Kind::LinkBar)),
        ("List", Control(Kind::List)),
        ("List with Icons", Preset(CatalogPreset::IconList)),
        ("Menu", Control(Kind::Menu)),
        ("Menu Bar", Control(Kind::MenuBar)),
        ("Modal Screen", Control(Kind::ModalOverlay)),
        ("Multiline Button", Control(Kind::MultilineButton)),
        ("Num. Stepper", Control(Kind::NumericStepper)),
        ("ON/OFF Switch", Control(Kind::Toggle)),
        ("Playback", Control(Kind::Playback)),
        ("Pointy Button", Control(Kind::PointedButton)),
        (
            "Pointy Button (iOS)",
            Preset(CatalogPreset::IosPointedButton),
        ),
        ("Popover", Control(Kind::Popover)),
        ("Progress Bar", Control(Kind::Progress)),
        (
            "Progress Bar (Android)",
            Preset(CatalogPreset::AndroidProgress),
        ),
        ("Radio Button", Control(Kind::Radio)),
        ("Radio Button Group", Control(Kind::RadioGroup)),
        ("Rectangle", Control(Kind::Rectangle)),
        ("Red X", Control(Kind::RedCross)),
        ("Scratch-Out", Control(Kind::ScratchOut)),
        ("Search Box", Control(Kind::Search)),
        ("Search Box (Android)", Preset(CatalogPreset::AndroidSearch)),
        ("Shape", Control(Kind::Shape)),
        ("Site Map", Control(Kind::SiteMap)),
        ("Smartphone", Control(Kind::Smartphone)),
        ("Squiggly Block of Text", Control(Kind::TextBlock)),
        ("Squiggly Line of Text", Control(Kind::TextLine)),
        ("Street Map", Control(Kind::StreetMap)),
        ("Tab Bar", Control(Kind::Tabs)),
        ("Tag Cloud", Control(Kind::TagCloud)),
        ("Text Area", Control(Kind::TextArea)),
        ("Text Input", Control(Kind::TextInput)),
        (
            "Text Input (Android)",
            Preset(CatalogPreset::AndroidTextInput),
        ),
        ("Text Label", Control(Kind::Label)),
        ("Text Paragraph", Control(Kind::Paragraph)),
        ("Text Subtitle", Control(Kind::Subtitle)),
        ("Text Title", Control(Kind::Heading)),
        ("Time Picker", Control(Kind::TimePicker)),
        ("Toolbar", Control(Kind::FormattingToolbar)),
        ("Tooltip", Control(Kind::Tooltip)),
        ("Tree Pane", Control(Kind::Tree)),
        ("V.Curly Brace", Control(Kind::VerticalBrace)),
        ("V.Rule", Control(Kind::VerticalRule)),
        ("V.Scroll Bar", Control(Kind::VerticalScrollbar)),
        ("V.Slider", Control(Kind::VerticalSlider)),
        (
            "V.Slider (Android)",
            Preset(CatalogPreset::AndroidVerticalSlider),
        ),
        ("V.Splitter", Control(Kind::VerticalSplitter)),
        ("V.Tabs", Control(Kind::VerticalTabs)),
        ("Video Player", Control(Kind::Video)),
        ("Volume Slider", Control(Kind::VolumeSlider)),
        ("Webcam", Control(Kind::Webcam)),
        ("Window", Control(Kind::Panel)),
    ];
    let actual: Vec<_> = editor
        .catalog_entries()
        .into_iter()
        .map(|e| (e.name, e.item))
        .collect();
    assert_eq!(actual, expected.map(|(name, item)| (name.to_owned(), item)));
}

#[test]
fn new_labels_and_legacy_searches_retain_creation_identity() {
    let editor = Editor::default();
    for entry in editor.catalog_entries() {
        let legacy = match entry.item {
            Control(kind) => kind.name(),
            Preset(preset) => preset.name(),
            _ => unreachable!(),
        };
        for query in [entry.name.as_str(), legacy] {
            assert!(
                editor
                    .search_catalog(query)
                    .iter()
                    .any(|found| found.item == entry.item),
                "{query}: {:?}",
                entry.item
            );
        }
    }
    for (query, kind) in [
        ("chart col", Kind::ColumnChart),
        ("radio button g", Kind::RadioGroup),
        ("combo", Kind::Dropdown),
        ("sticky note", Kind::StickyNote),
        ("horizontal rule", Kind::Line),
        ("dialog", Kind::Alert),
    ] {
        assert!(
            editor
                .search_catalog(query)
                .iter()
                .any(|e| e.item == CatalogItem::Control(kind)),
            "{query}"
        );
    }
}

#[test]
fn legacy_preset_prefixes_keep_existing_match_priority_and_order() {
    let editor = Editor::default();
    let matches = editor.search_catalog("horizontal");
    let position = |item| matches.iter().position(|entry| entry.item == item).unwrap();
    assert!(
        position(Control(Kind::HorizontalSlider))
            < position(Preset(CatalogPreset::AndroidHorizontalSlider))
    );
    assert!(
        position(Preset(CatalogPreset::AndroidHorizontalSlider))
            < position(Control(Kind::HorizontalSplitter))
    );
}
