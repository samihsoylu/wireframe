// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use eframe::egui::{Color32, Pos2, Rect, Vec2};
use std::sync::{LazyLock, Mutex};
use unicode_segmentation::UnicodeSegmentation;
use wireframe::text_entry::{
    Affinity, Caret, Draft, Selection,
    layout::{Engine, Layout},
};
static ENGINE: LazyLock<Mutex<Engine>> = LazyLock::new(|| Mutex::new(Engine::default()));

#[test]
fn every_visible_cell_maps_to_one_original_grapheme() {
    let mut engine = ENGINE.lock().unwrap_or_else(|e| e.into_inner());
    for source in [
        "ABC שלום 123",
        "שלום",
        "مرحبا",
        "office ffi",
        "a\u{301}\u{327}",
        "👩‍💻",
        "🇳🇱",
        "क्‍ष",
        "abc\r\nשלום\n123\n",
    ] {
        let layout = Layout::new(&mut engine, source, 13., 400., 2.).unwrap();
        assert_eq!(layout.source(), source);
        let expected: Vec<_> = source
            .grapheme_indices(true)
            .filter(|(_, g)| !g.contains(['\r', '\n']))
            .map(|(i, g)| i..i + g.len())
            .collect();
        let mut actual: Vec<_> = layout
            .rows
            .iter()
            .flat_map(|r| r.cells.iter().map(|c| c.source.clone()))
            .collect();
        actual.sort_by_key(|r| r.start);
        assert_eq!(actual, expected, "{source:?}");
        for cell in layout.rows.iter().flat_map(|r| &r.cells) {
            if cell.rect.width() < 0.1 {
                continue;
            }
            let y = cell.rect.center().y;
            let left = layout.hit(Pos2::new(cell.rect.left() + cell.rect.width() * 0.2, y));
            let right = layout.hit(Pos2::new(cell.rect.right() - cell.rect.width() * 0.2, y));
            assert_eq!(
                left.byte,
                if cell.rtl {
                    cell.source.end
                } else {
                    cell.source.start
                },
                "{source:?} {cell:?}"
            );
            assert_eq!(
                right.byte,
                if cell.rtl {
                    cell.source.start
                } else {
                    cell.source.end
                },
                "{source:?} {cell:?}"
            );
        }
    }
}

#[test]
fn pointer_before_number_edits_correct_source_location() {
    let mut engine = ENGINE.lock().unwrap_or_else(|e| e.into_inner());
    let source = "ABC שלום 123";
    let layout = Layout::new(&mut engine, source, 13., 400., 2.).unwrap();
    let cell = layout
        .rows
        .iter()
        .flat_map(|r| &r.cells)
        .find(|c| &source[c.source.clone()] == "1")
        .unwrap();
    let hit = layout.hit(Pos2::new(cell.rect.left() + 0.1, cell.rect.center().y));
    let mut draft = Draft::new(source.into());
    draft.move_to(hit, false);
    draft.insert("|");
    assert_eq!(draft.text(), "ABC שלום |123");
}

#[test]
fn bidi_boundary_has_two_distinct_caret_locations() {
    let mut engine = ENGINE.lock().unwrap_or_else(|e| e.into_inner());
    let layout = Layout::new(&mut engine, "ABC שלום 123", 13., 400., 2.).unwrap();
    let upstream = layout
        .caret_rect(Caret {
            byte: 4,
            affinity: Affinity::Upstream,
        })
        .unwrap();
    let downstream = layout
        .caret_rect(Caret {
            byte: 4,
            affinity: Affinity::Downstream,
        })
        .unwrap();
    assert!((upstream.left() - downstream.left()).abs() > 10.);
    assert_eq!(upstream.top(), downstream.top());
}

#[test]
fn wrapped_boundary_keeps_both_rows_addressable() {
    let mut engine = ENGINE.lock().unwrap_or_else(|e| e.into_inner());
    let source = "alpha beta gamma delta epsilon";
    let layout = Layout::new(&mut engine, source, 13., 70., 2.).unwrap();
    assert!(layout.rows.len() >= 3);
    let byte = layout.rows[0].source.end;
    assert_eq!(byte, layout.rows[1].source.start);
    let a = layout
        .caret_rect(Caret {
            byte,
            affinity: Affinity::Upstream,
        })
        .unwrap();
    let b = layout
        .caret_rect(Caret {
            byte,
            affinity: Affinity::Downstream,
        })
        .unwrap();
    assert!(a.top() < b.top());
    assert_eq!(
        layout.vertical(Caret::new(0), true, 0.).byte,
        layout.rows[1].source.start
    );
}

#[test]
fn selection_highlights_source_range_without_crossing_unselected_text() {
    let mut engine = ENGINE.lock().unwrap_or_else(|e| e.into_inner());
    let source = "ABC שלום 123";
    let layout = Layout::new(&mut engine, source, 13., 400., 2.).unwrap();
    let selection = Selection {
        anchor: Caret::new(4),
        focus: Caret::new(12),
    };
    let rects = layout.selection_rects(selection);
    assert!(!rects.is_empty());
    for cell in layout.rows.iter().flat_map(|r| &r.cells) {
        let selected = cell.source.start >= 4 && cell.source.end <= 12;
        assert_eq!(
            rects.iter().any(|r| r.contains(cell.rect.center())),
            selected,
            "{cell:?}"
        );
    }
}

#[test]
fn line_endings_empty_rows_and_visual_movement_are_addressable() {
    let mut engine = ENGINE.lock().unwrap_or_else(|e| e.into_inner());
    let source = "ab\r\n\ncd\n";
    let layout = Layout::new(&mut engine, source, 13., 400., 2.).unwrap();
    assert_eq!(layout.rows.len(), 4);
    assert!(layout.caret_rect(Caret::new(source.len())).is_some());
    let end_first = layout.caret_rect(Caret::new(2)).unwrap();
    let left = layout.horizontal(Caret::new(1), false);
    assert_eq!(left.byte, 0);
    let right = layout.horizontal(Caret::new(1), true);
    assert_eq!(right.byte, 2);
    assert!(end_first.left() > 0.);
    let all = layout.selection_rects(Selection {
        anchor: Caret::new(0),
        focus: Caret::new(source.len()),
    });
    assert!(all.iter().any(|r| r.top() == layout.rows[1].rect.top()));
}

#[test]
fn viewport_raster_is_cropped_from_the_same_layout() {
    let mut engine = ENGINE.lock().unwrap_or_else(|e| e.into_inner());
    let mut layout = Layout::new(&mut engine, "ABC שלום\n👩‍💻 a\u{301}", 16., 200., 2.).unwrap();
    let full = layout
        .raster(
            &mut engine,
            Rect::from_min_size(Pos2::ZERO, Vec2::new(200., 60.)),
            Color32::BLACK,
        )
        .unwrap();
    let crop = layout
        .raster(
            &mut engine,
            Rect::from_min_size(Pos2::new(0., 20.), Vec2::new(200., 30.)),
            Color32::BLACK,
        )
        .unwrap();
    assert!(full.pixels.iter().any(|c| c.a() != 0));
    for y in 0..crop.size[1] {
        assert_eq!(
            &crop.pixels[y * 400..(y + 1) * 400],
            &full.pixels[(y + 40) * 400..(y + 41) * 400]
        );
    }
}

#[test]
fn invalid_layout_or_raster_dimensions_fail_explicitly() {
    let mut engine = ENGINE.lock().unwrap_or_else(|e| e.into_inner());
    assert!(Layout::new(&mut engine, "abc", f32::NAN, 100., 2.).is_err());
    assert!(Layout::new(&mut engine, "abc", 13., f32::INFINITY, 2.).is_err());
    assert!(Layout::new(&mut engine, &"a".repeat(1024 * 1024 + 1), 13., 100., 2.).is_err());
    let mut layout = Layout::new(&mut engine, "abc", 13., 100., 2.).unwrap();
    assert!(
        layout
            .raster(
                &mut engine,
                Rect::from_min_size(Pos2::ZERO, Vec2::splat(10000.)),
                Color32::BLACK
            )
            .is_err()
    );
}

#[test]
fn unwrapped_lines_keep_full_width_and_visible_rtl_coordinates() {
    let mut engine = ENGINE.lock().unwrap_or_else(|e| e.into_inner());
    for source in ["ABC שלום 123 ".repeat(80), "שלום مرحبا ".repeat(80)] {
        let mut layout = Layout::with_wrap(&mut engine, &source, 13., 100., 2., false).unwrap();
        assert_eq!(layout.rows.len(), 1);
        assert!(layout.size.x > 100.);
        assert!(
            layout.rows[0]
                .cells
                .iter()
                .all(|c| c.rect.left() >= 0. && c.rect.right() <= layout.size.x + 0.01)
        );
        assert_eq!(layout.rows[0].cells.len(), source.graphemes(true).count());
        let right = layout.size.x - 100.;
        let image = layout
            .raster(
                &mut engine,
                Rect::from_min_size(Pos2::new(right, 0.), Vec2::new(100., 25.)),
                Color32::BLACK,
            )
            .unwrap();
        assert!(image.pixels.iter().any(|p| p.a() > 0));
    }
}
