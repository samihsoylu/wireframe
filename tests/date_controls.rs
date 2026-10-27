// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{
    date_controls::{self, Date},
    document::{self, Value},
    layout::{TextMeasurer, TextMetrics, TextStyle},
    model::Kind,
    render::Drawing,
    rich_text::RichText,
    workspace::Editor,
};
struct Metrics;
impl TextMeasurer for Metrics {
    fn measure(&self, text: &str, style: TextStyle, _: Option<f32>) -> TextMetrics {
        TextMetrics {
            width: text.len() as f32 * style.size / 2.,
            height: style.size * text.split('\n').count() as f32,
            rows: 1,
        }
    }
}
fn fixed() -> Date {
    Date::new(2028, 2, 29).unwrap()
}
fn style() -> TextStyle {
    TextStyle {
        size: 13.,
        bold: false,
        italic: false,
    }
}
fn set(e: &mut Editor, id: u64, key: &str, v: Value) {
    e.begin("Edit date control");
    e.set_control_property(id, key, v).unwrap();
    e.commit();
}
#[test]
fn calendar_uses_gregorian_years_and_six_complete_weeks() {
    assert!(Date::new(1900, 2, 29).is_none());
    assert!(Date::new(2000, 2, 29).is_some());
    assert!(Date::new(2100, 2, 29).is_none());
    for (year, month, day, days) in [
        (2000, 2, 29, 29),
        (2024, 2, 29, 29),
        (2026, 2, 28, 28),
        (2026, 12, 31, 31),
        (2027, 1, 1, 31),
    ] {
        let l = date_controls::calendar(
            Date::new(year, month, day).unwrap(),
            300.,
            300.,
            13.,
            true,
            &Metrics,
        );
        assert_eq!(l.days.len(), 42);
        assert_eq!(l.days.iter().filter(|d| !d.muted).count(), days);
        assert!(l.selection.is_some());
    }
}
#[test]
fn calendar_size_thresholds_and_headings_are_deterministic() {
    for sketch in [false, true] {
        for (w, h, grid) in [(199., 200., false), (200., 199., false), (200., 200., true)] {
            let l = date_controls::calendar(fixed(), w, h, 13., sketch, &Metrics);
            assert_eq!(l.rules.len(), if grid { 13 } else { 1 });
        }
        let small = date_controls::calendar(fixed(), 96., 96., 13., sketch, &Metrics);
        let large = date_controls::calendar(fixed(), 300., 300., 13., sketch, &Metrics);
        assert_eq!(small.font_size, 8.);
        assert_eq!(large.font_size, 13.);
        assert!(large.heading.unwrap().text.contains("2028"));
        assert_eq!(
            date_controls::calendar(fixed(), 500., 500., 72., sketch, &Metrics).font_size,
            19.
        );
        assert!(
            date_controls::calendar(fixed(), 20., 20., 13., sketch, &Metrics)
                .heading
                .is_none()
        );
    }
    assert!(
        date_controls::calendar(fixed(), 300., 60., 13., false, &Metrics)
            .days
            .is_empty()
    );
}
#[test]
fn chooser_retains_width_and_icon_overflow_but_computes_height() {
    for sketch in [false, true] {
        let text = RichText::plain("Date");
        let auto = date_controls::chooser(&text, style(), None, sketch, &Metrics);
        assert_eq!(auto.width, 90.);
        assert_eq!(auto.height, if sketch { 25. } else { 24. });
        let tiny = date_controls::chooser(&text, style(), Some(1.), sketch, &Metrics);
        assert!(tiny.field.is_none());
        assert_eq!(tiny.icon.x, -23.);
        let boundary = date_controls::chooser(&text, style(), Some(35.), sketch, &Metrics);
        assert_eq!(boundary.field.is_some(), !sketch);
        let large = date_controls::chooser(
            &text,
            TextStyle {
                size: 24.,
                ..style()
            },
            Some(50.),
            sketch,
            &Metrics,
        );
        assert_eq!(large.width, 50.);
        assert_eq!(large.height, if sketch { 36. } else { 35. });
        let soft = date_controls::chooser(
            &RichText::parse("A\\rB", false),
            style(),
            None,
            sketch,
            &Metrics,
        );
        assert!(soft.height > auto.height);
    }
}
#[test]
fn editor_supports_properties_history_and_storage() {
    let mut e = Editor::with_date(document::Project::default(), fixed);
    e.add(Kind::Calendar, 10., 20.);
    e.add(Kind::DateChooser, 200., 20.);
    let id = *e.selection.iter().next().unwrap();
    set(&mut e, id, "text", Value::Text("A\\rB".into()));
    set(&mut e, id, "state", Value::Choice("disabled".into()));
    set(&mut e, id, "border_color", Value::Color([255, 0, 0, 255]));
    e.selected_mut(|c| c.bounds.w = 50.);
    let before = e.document.encode().unwrap();
    e.begin("Auto size");
    e.auto_size_control(id).unwrap();
    e.commit();
    let after = e.document.encode().unwrap();
    assert_ne!(before, after);
    e.undo();
    assert_eq!(before, e.document.encode().unwrap());
    e.redo();
    assert_eq!(after, e.document.encode().unwrap());
    let reopened = Editor::with_date(document::decode(&after).unwrap(), fixed);
    assert_eq!(after, reopened.document.encode().unwrap());
    let scene = reopened.rendered_scene(reopened.page).unwrap();
    assert!(!scene.items.is_empty());
    assert!(scene.links.is_empty());
    assert!(
        wireframe::catalog::get("calendar")
            .unwrap()
            .properties
            .iter()
            .all(|p| !p.visible)
    );
}
#[test]
fn rendering_preserves_disabled_text_and_transparent_icon_apertures() {
    let mut e = Editor::default();
    e.document.settings.sketch = false;
    e.add(Kind::DateChooser, 50., 20.);
    let id = *e.selection.iter().next().unwrap();
    set(&mut e, id, "state", Value::Choice("disabled".into()));
    let scene = e.rendered_scene(e.page).unwrap();
    assert!(
        scene
            .items
            .iter()
            .any(|i| matches!(&i.drawing,Drawing::Glyph(g) if g.color[..3]==[128,128,139]))
    );
    e.selected_mut(|c| c.bounds.w = 1.);
    let scene = e.rendered_scene(e.page).unwrap();
    assert!(scene.items.iter().all(|i| i.clip.is_none()));
    assert!(scene.items.iter().any(|item| matches!(&item.drawing,
        Drawing::Icon { id, color, .. } if id == "calendar-days-regular" && *color == [207,207,207,255]
    )));
}
#[test]
fn date_snapshot_is_per_control_and_refreshes_after_content_relayout() {
    use std::sync::atomic::{AtomicU32, Ordering};
    static DAY: AtomicU32 = AtomicU32::new(1);
    fn today() -> Date {
        Date::new(2028, 2, DAY.load(Ordering::SeqCst)).unwrap()
    }
    fn headings(e: &Editor) -> Vec<String> {
        e.rendered_scene(e.page)
            .unwrap()
            .items
            .iter()
            .filter_map(|i| match &i.drawing {
                Drawing::Glyph(g) if g.text.contains("FEBRUARY") => Some(g.text.clone()),
                _ => None,
            })
            .collect()
    }
    let mut e = Editor::with_date(document::Project::default(), today);
    e.add(Kind::Calendar, 20., 20.);
    e.selected_mut(|c| {
        c.bounds.w = 300.;
        c.bounds.h = 60.;
    });
    assert!(headings(&e).iter().any(|s| s.starts_with("1 ")));
    DAY.store(2, Ordering::SeqCst);
    e.selected_mut(|c| c.bounds.x += 10.);
    assert!(headings(&e).iter().any(|s| s.starts_with("1 ")));
    e.selected_mut(|c| c.bounds.w = 310.);
    assert!(headings(&e).iter().any(|s| s.starts_with("2 ")));
    let saved = e.document.encode().unwrap();
    DAY.store(3, Ordering::SeqCst);
    let reopened = Editor::with_date(document::decode(&saved).unwrap(), today);
    assert!(headings(&reopened).iter().any(|s| s.starts_with("3 ")));
    assert_eq!(saved, reopened.document.encode().unwrap());
}
