// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{
    document::{Properties, Value},
    layout::TextMetrics,
    phone::{Model, Options, Pattern, StatusKind},
};
fn metrics(size: f32) -> (TextMetrics, TextMetrics) {
    (
        TextMetrics {
            width: 1.5 * size,
            height: size,
            rows: 1,
        },
        TextMetrics {
            width: 4. * size,
            height: size,
            rows: 1,
        },
    )
}
#[test]
fn device_fit_stays_centered_and_orientation_preserves_native_ratio() {
    for model in [Model::Classic, Model::Tall, Model::Wide, Model::Notched] {
        for landscape in [false, true] {
            let o = Options {
                model,
                landscape,
                ..Options::default()
            };
            let (nw, nh) = o.native();
            for (w, h) in [(300., 500.), (500., 300.), (1., 1.), (10000., 20.)] {
                let b = o.frame(w, h);
                assert!(b.w <= w + 0.001 && b.h <= h + 0.001);
                assert!((b.x * 2. + b.w - w).abs() < 0.001);
                assert!((b.y * 2. + b.h - h).abs() < 0.001);
                assert!((b.w / b.h - nw / nh).abs() < 0.001);
                assert_eq!(
                    Options { stretch: true, ..o }.frame(w, h),
                    wireframe::model::Bounds::new(0., 0., w, h)
                );
            }
        }
    }
}
#[test]
fn status_visibility_has_strict_style_and_width_boundaries() {
    for model in [Model::Classic, Model::Tall, Model::Wide, Model::Notched] {
        for sketch in [false, true] {
            for landscape in [false, true] {
                let o = Options {
                    model,
                    landscape,
                    stretch: true,
                    ..Options::default()
                };
                let s = model.sections(sketch);
                let h = if landscape { s[1] + s[2] } else { s[0] + s[3] };
                assert!(!o.layout(500., h, sketch, metrics).status_visible);
                assert!(o.layout(500., h + 1., sketch, metrics).status_visible);
                if model == Model::Notched {
                    let w = if landscape { 215. } else { 260. };
                    assert!(!o.layout(w, 500., sketch, metrics).status_visible);
                    assert!(o.layout(w + 1., 500., sketch, metrics).status_visible);
                }
            }
        }
    }
    let o = Options {
        landscape: true,
        stretch: true,
        ..Options::default()
    };
    assert!(!o.layout(300., 200., true, metrics).status_visible);
    assert!(o.layout(300., 200., false, metrics).status_visible);
}
#[test]
fn transparency_removes_only_the_base_fill_and_keeps_pattern_bars() {
    let o = Options {
        model: Model::Notched,
        stretch: true,
        pattern: Pattern::HeaderDarkFooter,
        ..Options::default()
    };
    let opaque = o.layout(300., 500., false, metrics);
    let transparent = Options {
        transparent: true,
        ..o
    }
    .layout(300., 500., false, metrics);
    assert_eq!(opaque.fills.len(), transparent.fills.len() + 1);
    for (a, b) in opaque.fills.iter().skip(1).zip(&transparent.fills) {
        assert_eq!(a.bounds, b.bounds);
        assert_eq!(a.color, b.color);
    }
    assert_eq!(opaque.status_items.len(), transparent.status_items.len());
    assert_eq!(opaque.guides, transparent.guides);
    let white = Options {
        transparent: true,
        pattern: Pattern::White,
        ..o
    }
    .layout(300., 500., false, metrics);
    assert!(white.fills.is_empty());
    assert!(!white.status_items.is_empty());
    let no_status = Options { status: false, ..o }.layout(300., 500., false, metrics);
    assert!(no_status.status_items.is_empty());
    assert_eq!(no_status.fills.len(), 3);
}
#[test]
fn small_fitted_notched_frame_can_keep_time_without_icons() {
    let l = Options {
        model: Model::Notched,
        ..Options::default()
    }
    .layout(270., 430., true, metrics);
    assert!(l.status_items.iter().any(|i| i.kind == StatusKind::Time));
    assert!(
        l.status_items
            .iter()
            .all(|i| matches!(i.kind, StatusKind::Time | StatusKind::Carrier))
    );
    for item in l.status_items {
        assert!(item.size > 0.);
    }
}
#[test]
fn stored_device_defaults_and_explicit_choices_are_distinct() {
    let defaults = Options::from_properties(&Properties::new());
    assert_eq!(defaults.model, Model::Classic);
    assert!(!defaults.stretch);
    assert!(defaults.status);
    let p = Properties::from([
        ("device_model".into(), Value::Choice("notched".into())),
        ("device_resizing".into(), Value::Choice("stretch".into())),
        ("orientation".into(), Value::Choice("landscape".into())),
        ("background_pattern".into(), Value::Choice("dark".into())),
        ("status_bar".into(), Value::Bool(false)),
        ("transparent_background".into(), Value::Bool(true)),
    ]);
    let o = Options::from_properties(&p);
    assert_eq!(o.native(), (867., 436.));
    assert_eq!(o.pattern, Pattern::Dark);
    assert!(o.stretch && o.transparent && !o.status);
}
