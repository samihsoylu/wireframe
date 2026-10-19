// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;

#[test]
fn formatting_keeps_literal_list_prefixes_and_unknown_icons() {
    let source = "*Bold* _Italic_ &Underlined& -Muted- ~Strike~\n- first\n* second\n:star-solid: :star-regular: :unknown-icon:";
    let parts = spans(source);
    assert_eq!(parts.iter().filter(|s| s.icon.is_some()).count(), 2);
    let display: String = parts.iter().map(|s| s.text.as_str()).collect();
    assert_eq!(
        display,
        "Bold Italic Underlined Muted Strike\n- first\n* second\n\u{2003} \u{2003} :unknown-icon:"
    );
    for (text, predicate) in [
        ("Bold", (|s: &Style| s.bold) as fn(&Style) -> bool),
        ("Italic", |s| s.italic),
        ("Underlined", |s| s.underline),
        ("Muted", |s| s.disabled),
        ("Strike", |s| s.strike),
    ] {
        assert!(parts.iter().any(|s| s.text == text && predicate(&s.style)));
    }
}

#[test]
fn explicit_links_have_color_without_changing_click_behavior_or_raw_addresses() {
    let parts = spans(
        "[Website](https://example.com) [Mail](mailto:notes@example.com) [No target] [Bare host](example.com) www.example.com",
    );
    for name in ["Website", "Mail", "No target", "Bare host"] {
        let span = parts.iter().find(|s| s.text == name).unwrap();
        assert_eq!(ink(&span.style), Color32::from_rgb(6, 123, 186));
        assert!(!span.style.underline);
    }
    assert!(
        parts
            .iter()
            .any(|s| s.text.contains("www.example.com") && !s.style.link)
    );
}

#[test]
fn wrapped_styled_unicode_and_icons_share_density_scaled_geometry() {
    let text = "*Heading* שלום مرحبا 👩‍💻\n{size:20}Large{size} :star-solid: :star-regular:\nA longer line to wrap across a narrow notes panel.";
    let mut engine = Engine::default();
    let mut low = NotesLayout::new(&mut engine, text, 13., 140., 1.).unwrap();
    let mut high = NotesLayout::new(&mut engine, text, 13., 140., 2.).unwrap();
    assert_eq!(low.layout.source, high.layout.source);
    assert!(low.layout.buffer.layout_runs().count() > 3);
    assert_eq!(low.icons.len(), 2);
    assert_eq!(high.icons.len(), 2);
    assert!((low.size().y - high.size().y).abs() < 1.);
    for ((_, a, _), (_, b, _)) in low.icons.iter().zip(&high.icons) {
        assert!((a.min - b.min).length() < 1.);
        assert!(a.is_finite() && a.is_positive());
    }
    let viewport = Rect::from_min_size(Pos2::ZERO, Vec2::new(140., 80.));
    let a = low.raster(&mut engine, viewport).unwrap();
    let b = high.raster(&mut engine, viewport).unwrap();
    assert_eq!(a.size, [140, 80]);
    assert_eq!(b.size, [280, 160]);
    assert!(a.pixels.iter().any(|p| p.a() > 0));
    assert!(b.pixels.iter().any(|p| p.a() > 0));
}

#[test]
fn color_and_decorations_reach_the_native_raster() {
    let mut engine = Engine::default();
    let viewport = Rect::from_min_size(Pos2::ZERO, Vec2::new(200., 50.));
    let raster = |engine: &mut Engine, text| {
        NotesLayout::new(engine, text, 16., 200., 2.)
            .unwrap()
            .raster(engine, viewport)
            .unwrap()
    };
    let plain = raster(&mut engine, "Sample");
    for text in ["*Sample*", "_Sample_", "&Sample&", "~Sample~", "-Sample-"] {
        assert_ne!(plain.pixels, raster(&mut engine, text).pixels, "{text}");
    }
    let red = raster(&mut engine, "{color:red}Sample{color}");
    assert!(
        red.pixels
            .iter()
            .any(|p| p.a() == 255 && *p == Color32::from_rgb(207, 42, 39))
    );
}

#[test]
fn dimensions_and_viewport_allocations_remain_bounded() {
    let mut engine = Engine::default();
    for (size, width, scale) in [
        (f32::NAN, 200., 1.),
        (13., f32::INFINITY, 1.),
        (13., 200., 0.),
    ] {
        assert!(NotesLayout::new(&mut engine, "Notes", size, width, scale).is_err());
    }
    assert!(NotesLayout::new(&mut engine, &"x".repeat(1024 * 1024 + 1), 13., 200., 1.).is_err());
    let mut notes = NotesLayout::new(&mut engine, "{size:65535}Safe{size}", 13., 200., 1.).unwrap();
    assert!(notes.size().y < 100.);
    assert!(
        notes
            .raster(
                &mut engine,
                Rect::from_min_size(Pos2::ZERO, Vec2::splat(16384.))
            )
            .is_err()
    );
}

#[test]
fn notes_ink_preserves_explicit_colors_and_muted_precedence() {
    for (source, color) in [
        ("Plain", Color32::BLACK),
        ("-Muted-", Color32::from_gray(179)),
        ("{color:blue5}Blue{color}", Color32::from_rgb(8, 83, 148)),
        ("{color:bogus}[Link]{color}", Color32::BLACK),
        ("-{color:red}Muted{color}-", Color32::from_gray(179)),
    ] {
        let parts = spans(source);
        assert_eq!(parts.len(), 1, "{source}");
        assert_eq!(ink(&parts[0].style), color, "{source}");
    }
}
