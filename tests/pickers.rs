// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{
    date_controls::Date,
    document::{self, Link, Project, Value},
    layout::{TextMeasurer, TextMetrics, TextStyle},
    model::Kind,
    pickers::{self, Ink, Mode, Options},
    render::Drawing,
    tablet::Clock,
    workspace::Editor,
};
struct Metrics;
impl TextMeasurer for Metrics {
    fn measure(&self, text: &str, style: TextStyle, _: Option<f32>) -> TextMetrics {
        let rows: Vec<_> = text.split('\n').collect();
        TextMetrics {
            width: rows.iter().map(|s| s.len()).max().unwrap_or(0) as f32 * style.size / 2.,
            height: rows.len() as f32 * style.size,
            rows: rows.len(),
        }
    }
}
fn date() -> Date {
    Date::new(2028, 2, 29).unwrap()
}
fn clock() -> Clock {
    Clock::new(0, 5).unwrap()
}
fn layout(mode: Mode, w: f32, h: f32) -> pickers::Layout {
    pickers::layout(
        Options {
            mode,
            width: w,
            height: h,
            sketch: false,
            project_font: 13.,
        },
        date(),
        clock(),
        &Metrics,
    )
}
fn set(e: &mut Editor, id: u64, key: &str, value: Value) {
    e.begin("Edit picker");
    e.set_control_property(id, key, value).unwrap();
    e.commit();
}
#[test]
fn month_uses_gregorian_days_and_orientation_while_year_mode_preserves_spacing() {
    let month = layout(Mode::Month, 320., 473.);
    assert!(month.labels.iter().any(|l| l.text == "29" && l.bold));
    assert!(!month.labels.iter().any(|l| l.text == "30"));
    assert_eq!(month.arrows.len(), 2);
    assert!(
        month
            .labels
            .iter()
            .any(|l| l.text == "Tue, Feb 29" && l.size == 13.)
    );
    assert!(
        layout(Mode::Month, 300., 300.)
            .labels
            .iter()
            .any(|l| l.text == "Tue\nFeb 29")
    );
    for (w, h, count) in [(320., 473., 7), (570., 303., 5)] {
        let year = layout(Mode::Year, w, h);
        let labels: Vec<_> = year
            .labels
            .iter()
            .filter(|l| l.text.len() == 4 && l.size != 13.)
            .collect();
        assert_eq!(labels.len(), count);
        assert_eq!(labels.iter().filter(|l| l.size == 32.).count(), 1);
        assert!(year.arrows.is_empty());
        assert!(year.selection.is_none());
    }
}
#[test]
fn button_visibility_is_strict_and_keeps_source_slots() {
    for mode in [Mode::Month, Mode::Year, Mode::Time] {
        for (w, expected) in [
            (34., vec![]),
            (35., vec![1]),
            (96., vec![1]),
            (97., vec![0, 1]),
        ] {
            let l = layout(mode, w, 200.);
            assert_eq!(
                l.labels.iter().filter_map(|l| l.button).collect::<Vec<_>>(),
                expected
            );
        }
    }
}
#[test]
fn midnight_is_twelve_and_analog_selection_has_the_selected_foreground() {
    for hour in [0, 12, 23] {
        let l = pickers::layout(
            Options {
                mode: Mode::Time,
                width: 320.,
                height: 473.,
                sketch: false,
                project_font: 13.,
            },
            date(),
            Clock::new(hour, 5).unwrap(),
            &Metrics,
        );
        let expected = if hour == 23 { "11:05" } else { "12:05" };
        assert!(l.labels.iter().any(|l| l.text == expected));
        let selected = l.labels.iter().find(|l| l.ink == Ink::Selected).unwrap();
        assert_eq!(selected.text, if hour == 23 { "11" } else { "12" });
        assert!(l.hand.is_some());
    }
    assert_eq!(wireframe::app_bar::foreground([0, 0, 0, 255]), [255; 4]);
}
#[test]
fn narrow_rendered_ok_keeps_its_destination_and_tiny_links_are_clipped() {
    for kind in [Kind::DatePicker, Kind::TimePicker] {
        let mut e = Editor::with_clock(Project::default(), clock);
        e.add(kind, 20., 30.);
        let id = *e.selection.iter().next().unwrap();
        let page = e.page;
        set(
            &mut e,
            id,
            "row_links",
            Value::Links(vec![Some(Link::Back), Some(Link::Page(page))]),
        );
        e.selected_mut(|c| {
            c.bounds.w = 60.;
            c.bounds.h = 200.;
        });
        let s = e.rendered_scene(e.page).unwrap();
        assert_eq!(s.links.len(), 1);
        assert_eq!(s.links[0].target, format!("page:{}", e.page));
        assert!(s.items.iter().all(|i| i.clip.is_some()));
        e.selected_mut(|c| c.bounds.h = 1.);
        assert!(e.rendered_scene(e.page).unwrap().links.is_empty());
    }
}
#[test]
fn mode_and_manual_size_survive_history_storage_and_local_component_edits() {
    let mut e = Editor::with_date(Project::default(), date);
    e.add(Kind::DatePicker, 20., 20.);
    let id = *e.selection.iter().next().unwrap();
    e.selected_mut(|c| {
        c.bounds.w = 570.;
        c.bounds.h = 303.;
    });
    let before = e.document.clone();
    set(&mut e, id, "date_mode", Value::Choice("year".into()));
    assert_eq!(
        (
            e.selected_views()[0].bounds.w,
            e.selected_views()[0].bounds.h
        ),
        (570., 303.)
    );
    let after = e.document.clone();
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    assert_eq!(e.document, after);
    let encoded = e.document.encode().unwrap();
    assert_eq!(document::decode(&encoded).unwrap(), e.document);
    assert!(
        e.set_control_property(id, "date_mode", Value::Choice("invalid".into()))
            .is_err()
    );
    assert_eq!(after, e.document);
    e.create_component("Date dialog".into()).unwrap();
    let definitions = e.document.definitions.clone();
    assert!(e.enter(id));
    let child = e.nodes()[0].id;
    set(&mut e, child, "date_mode", Value::Choice("month".into()));
    assert_eq!(definitions, e.document.definitions);
    e.undo();
    assert_eq!(
        e.control_properties(child).unwrap().1["date_mode"],
        Value::Choice("year".into())
    );
}
#[test]
fn moving_time_picker_retains_clock_and_resizing_refreshes_it() {
    use std::sync::atomic::{AtomicU8, Ordering};
    static HOUR: AtomicU8 = AtomicU8::new(1);
    fn source() -> Clock {
        Clock::new(HOUR.load(Ordering::SeqCst), 5).unwrap()
    }
    fn time(e: &Editor) -> String {
        e.rendered_scene(e.page)
            .unwrap()
            .items
            .iter()
            .filter_map(|i| {
                if let Drawing::Glyph(g) = &i.drawing {
                    Some(g.text.clone())
                } else {
                    None
                }
            })
            .find(|s| s.contains(':'))
            .unwrap()
    }
    let mut e = Editor::with_clock(Project::default(), source);
    e.add(Kind::TimePicker, 0., 0.);
    assert_eq!(time(&e), "1:05");
    HOUR.store(2, Ordering::SeqCst);
    e.selected_mut(|c| c.bounds.x = 50.);
    assert_eq!(time(&e), "1:05");
    e.selected_mut(|c| c.bounds.w = 321.);
    assert_eq!(time(&e), "2:05");
    let saved = e.document.encode().unwrap();
    HOUR.store(3, Ordering::SeqCst);
    let reopened = Editor::with_clock(document::decode(&saved).unwrap(), source);
    assert_eq!(time(&reopened), "3:05");
    assert_eq!(saved, reopened.document.encode().unwrap());
}
#[test]
fn date_mode_refreshes_snapshot_and_exports_use_the_same_scene() {
    use std::sync::atomic::{AtomicU32, Ordering};
    static DAY: AtomicU32 = AtomicU32::new(1);
    fn source() -> Date {
        Date::new(2028, 2, DAY.load(Ordering::SeqCst)).unwrap()
    }
    fn heading(e: &Editor) -> String {
        e.rendered_scene(e.page)
            .unwrap()
            .items
            .iter()
            .filter_map(|i| {
                if let Drawing::Glyph(g) = &i.drawing {
                    Some(g.text.clone())
                } else {
                    None
                }
            })
            .find(|s| s.contains("Feb"))
            .unwrap()
    }
    let mut e = Editor::with_date(Project::default(), source);
    e.add(Kind::DatePicker, 0., 0.);
    let id = *e.selection.iter().next().unwrap();
    assert!(heading(&e).ends_with("01"));
    DAY.store(2, Ordering::SeqCst);
    e.selected_mut(|c| c.bounds.x = 10.);
    assert!(heading(&e).ends_with("01"));
    set(&mut e, id, "date_mode", Value::Choice("year".into()));
    assert!(heading(&e).ends_with("02"));
    let scene = e.rendered_scene(e.page).unwrap();
    assert!(scene.svg().contains("clipPath"));
    assert!(scene.png().is_ok());
}
