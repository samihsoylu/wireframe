// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use crate::model::Bounds;

const AIDS: [Aid; 6] = [
    Aid::Navigator,
    Aid::Notes,
    Aid::Comments,
    Aid::LinkHints,
    Aid::BigCursor,
    Aid::Markup,
];

#[test]
fn presets_and_custom_follow_controller_lifetime() {
    let mut session = Session::default();
    assert!(AIDS.into_iter().all(|a| session.settings().aid(a)));
    assert_eq!(session.settings().zoom, Zoom::Fit);
    assert!(!session.select(Preset::Review));
    session.toggle(Aid::BigCursor);
    session.toggle(Aid::Markup);
    let custom = session.settings().clone();
    assert_eq!(custom.preset, Preset::Custom);
    session.select(Preset::UserTest);
    assert!(AIDS.into_iter().all(|a| !session.settings().aid(a)));
    assert_eq!(session.settings().zoom, Zoom::ActualSize);
    session.select(Preset::Review);
    session.select(Preset::Custom);
    assert_eq!(session.settings(), &custom);
    let restored = Session::new(custom.clone()).unwrap();
    assert_eq!(restored.settings(), &custom);
    session.select(Preset::Review);
    let mut next = Session::new(session.settings().clone()).unwrap();
    next.select(Preset::Custom);
    assert!(next.settings().big_cursor && next.settings().markup);
}

#[test]
fn first_custom_retains_values_and_panel_changes_are_custom() {
    let mut session = Session::default();
    session.select(Preset::UserTest);
    session.select(Preset::Custom);
    assert!(AIDS.into_iter().all(|a| !session.settings().aid(a)));
    assert_eq!(session.settings().zoom, Zoom::ActualSize);
    session.select(Preset::Review);
    session.set_panel_sizes(240., 411., 350.).unwrap();
    assert_eq!(session.settings().preset, Preset::Custom);
    session.select(Preset::UserTest);
    session.select(Preset::Review);
    assert_eq!(
        (
            session.settings().navigator_width,
            session.settings().side_width,
            session.settings().notes_height
        ),
        (240., 411., 350.)
    );
    let before = session.settings().clone();
    assert!(session.set_panel_sizes(f32::NAN, 100., 100.).is_err());
    assert_eq!(session.settings(), &before);
}

#[test]
fn centering_does_not_select_or_refresh_custom() {
    let mut session = Session::default();
    session.set_excluded([11].into()).unwrap();
    assert_eq!(session.settings().preset, Preset::Review);
    session.toggle(Aid::Notes);
    session.set_excluded([12].into()).unwrap();
    session.select(Preset::UserTest);
    assert_eq!(session.settings().excluded, [12].into());
    session.select(Preset::Review);
    assert_eq!(session.settings().excluded, [12].into());
    session.select(Preset::Custom);
    assert_eq!(session.settings().excluded, [11].into());
    let before = session.settings().clone();
    assert!(session.set_excluded([0].into()).is_err());
    assert_eq!(session.settings(), &before);
}

#[test]
fn zoom_is_an_individual_custom_choice() {
    let mut session = Session::default();
    session.toggle_zoom();
    assert_eq!(session.settings().zoom, Zoom::ActualSize);
    assert_eq!(session.settings().preset, Preset::Custom);
    session.toggle_zoom();
    assert_eq!(session.settings().zoom, Zoom::Fit);
}

fn extent(surface: Id, w: f32, h: f32) -> PageExtent {
    PageExtent {
        surface,
        with_markup: Bounds::new(0., 0., w, h),
        without_markup: Bounds::new(0., 0., w, h),
    }
}
#[test]
fn mixed_size_surfaces_share_independent_maxima_without_upscaling() {
    let pages = [
        extent(1, 200., 100.),
        extent(2, 800., 300.),
        extent(3, 300., 600.),
    ];
    for id in 1..=3 {
        assert_eq!(
            centered_view(&Settings::default(), &pages, id, [1000., 800.]).unwrap(),
            Framing {
                zoom: 1.,
                offset: [100., 100.]
            }
        );
    }
}
#[test]
fn excluded_alternate_uses_own_extent_and_does_not_enlarge_other_surfaces() {
    let pages = [extent(1, 200., 100.), extent(2, 800., 600.)];
    let settings = Settings {
        excluded: [2].into(),
        ..Settings::default()
    };
    assert_eq!(
        centered_view(&settings, &pages, 1, [1000., 800.])
            .unwrap()
            .offset,
        [400., 350.]
    );
    assert_eq!(
        centered_view(&settings, &pages, 2, [1000., 800.])
            .unwrap()
            .offset,
        [100., 100.]
    );
    let settings = Settings {
        excluded: [1, 2].into(),
        ..settings
    };
    assert_eq!(
        centered_view(&settings, &pages, 1, [1000., 800.])
            .unwrap()
            .offset,
        [400., 350.]
    );
}
#[test]
fn fit_floor_and_actual_size_are_separate_from_editor_zoom_limits() {
    let settings = Settings::default();
    assert_eq!(
        centered_view(&settings, &[extent(1, 2000., 1000.)], 1, [1000., 800.]).unwrap(),
        Framing {
            zoom: 0.5,
            offset: [0., 150.]
        }
    );
    assert_eq!(
        centered_view(&settings, &[extent(1, 100000., 100000.)], 1, [1000., 800.]).unwrap(),
        Framing {
            zoom: 0.05,
            offset: [0., 0.]
        }
    );
    let settings = Settings {
        zoom: Zoom::ActualSize,
        ..settings
    };
    assert_eq!(
        centered_view(&settings, &[extent(1, 2000., 1000.)], 1, [1000., 800.]).unwrap(),
        Framing {
            zoom: 1.,
            offset: [0., 0.]
        }
    );
}
#[test]
fn hidden_markup_compensates_offsets_and_preserves_half_point_rounding() {
    let mut page = extent(1, 800., 600.);
    page.without_markup = Bounds::new(20., 40., 400., 200.);
    let settings = Settings {
        markup: false,
        ..Settings::default()
    };
    assert_eq!(
        centered_view(&settings, &[page], 1, [1000., 800.])
            .unwrap()
            .offset,
        [260., 220.]
    );
    assert_eq!(
        centered_view(&settings, &[page], 1, [1001., 801.])
            .unwrap()
            .offset,
        [260.5, 220.5]
    );
    page.with_markup = Bounds::new(-100., -100., 800., 600.);
    assert_eq!(
        centered_view(&settings, &[page], 1, [1000., 800.])
            .unwrap()
            .offset,
        [160., 120.]
    );
}
#[test]
fn empty_zero_viewport_and_invalid_bounds_are_explicit() {
    assert_eq!(
        centered_view(&Settings::default(), &[extent(1, 0., 0.)], 1, [1000., 800.]).unwrap(),
        Framing {
            zoom: 1.,
            offset: [500., 400.]
        }
    );
    assert_eq!(
        centered_view(&Settings::default(), &[extent(1, 20., 10.)], 1, [0., 0.]).unwrap(),
        Framing {
            zoom: 1.,
            offset: [0., 0.]
        }
    );
    assert!(centered_view(&Settings::default(), &[], 1, [1000., 800.]).is_err());
    assert!(
        centered_view(
            &Settings::default(),
            &[extent(1, f32::NAN, 10.)],
            1,
            [1000., 800.]
        )
        .is_err()
    );
}

#[test]
fn preferences_roundtrip_per_project_without_persisting_old_custom() {
    let dir = tempfile::tempdir().unwrap();
    let mut preferences = Preferences::default();
    preferences.load(dir.path()).unwrap();
    let mut session = Session::default();
    session.toggle(Aid::BigCursor);
    preferences.remember("a", session.settings()).unwrap();
    session.select(Preset::UserTest);
    preferences.remember("b", session.settings()).unwrap();
    session.select(Preset::Review);
    preferences.remember("a", session.settings()).unwrap();
    let mut reopened = Preferences::default();
    reopened.load(dir.path()).unwrap();
    assert_eq!(reopened.get("b").preset, Preset::UserTest);
    let mut a = Session::new(reopened.get("a")).unwrap();
    a.select(Preset::Custom);
    assert!(a.settings().big_cursor);
    assert_eq!(reopened.get("missing"), Settings::default());
    let json: serde_json::Value = serde_json::from_slice(
        &std::fs::read(dir.path().join("presentation-settings.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(json["a"].as_object().unwrap().len(), 12);
}
#[test]
fn unreadable_preferences_are_preserved_and_session_choices_still_work() {
    for bytes in [
        b"{broken".as_slice(),
        br#"{"a":{"side_width":-1}}"#,
        br#"{"a":{"zoom":"unknown"}}"#,
    ] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("presentation-settings.json");
        std::fs::write(&path, bytes).unwrap();
        let mut p = Preferences::default();
        assert!(p.load(dir.path()).is_err());
        let mut s = Session::default();
        s.select(Preset::UserTest);
        assert!(p.remember("a", s.settings()).is_err());
        assert_eq!(p.get("a"), *s.settings());
        assert_eq!(std::fs::read(path).unwrap(), bytes);
    }
}
#[test]
fn failed_write_can_retry_the_identical_choice() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("presentation-settings.json");
    let mut p = Preferences::default();
    p.load(dir.path()).unwrap();
    std::fs::create_dir(&path).unwrap();
    let mut s = Session::default();
    s.toggle(Aid::Notes);
    assert!(p.remember("a", s.settings()).is_err());
    std::fs::remove_dir(&path).unwrap();
    p.remember("a", s.settings()).unwrap();
    let mut reopened = Preferences::default();
    reopened.load(dir.path()).unwrap();
    assert_eq!(reopened.get("a"), *s.settings());
}

#[test]
fn oversized_settings_are_not_overwritten_and_new_fields_default() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("presentation-settings.json");
    let bytes = vec![b' '; 4 * 1024 * 1024 + 1];
    std::fs::write(&path, &bytes).unwrap();
    let mut p = Preferences::default();
    assert!(p.load(dir.path()).unwrap_err().contains("size limit"));
    assert!(p.remember("a", &Settings::default()).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    std::fs::write(&path, br#"{"a":{"notes":false}}"#).unwrap();
    p.load(dir.path()).unwrap();
    assert_eq!(
        p.get("a"),
        Settings {
            notes: false,
            ..Settings::default()
        }
    );
}

#[test]
fn invalid_settings_never_replace_the_current_project_choice() {
    let mut p = Preferences::default();
    p.remember("a", &Settings::default()).unwrap();
    let invalid = Settings {
        navigator_width: f32::INFINITY,
        ..Settings::default()
    };
    assert!(Session::new(invalid.clone()).is_err());
    assert!(p.remember("a", &invalid).is_err());
    assert_eq!(p.get("a"), Settings::default());
    let too_many = Settings {
        excluded: (1..=50_001).collect(),
        ..Settings::default()
    };
    assert!(Session::new(too_many).is_err());
}
