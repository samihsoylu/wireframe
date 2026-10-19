// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use cosmic_text::{
    Attrs, Buffer, Family, FontSystem, Metrics, Shaping, SwashCache, Weight, fontdb,
};

#[test]
fn immutable_font_instances_keep_old_layouts_and_raster_caches_valid() {
    let mut db = fontdb::Database::new();
    db.load_font_data(include_bytes!("fixtures/text/variable/variable.ttf").to_vec());
    let source = db.faces().next().unwrap().id;
    let mut system = FontSystem::new_with_locale_and_db("en-US".into(), db);
    let mut cache = SwashCache::new();
    let mut saved = Vec::new();
    let mut widths = Vec::new();
    for (family, size, axes) in [
        ("body", 13., vec![(*b"opsz", 14.), (*b"wdth", 100.)]),
        ("display", 13., vec![(*b"opsz", 64.), (*b"wdth", 100.)]),
        ("condensed", 13., vec![(*b"opsz", 14.), (*b"wdth", 60.)]),
    ] {
        let id = system
            .add_font_instance(source, family.into(), axes, size)
            .unwrap();
        assert_ne!(id, source);
        let mut buffer = Buffer::new(&mut system, Metrics::new(26., 36.));
        buffer.set_text(
            "AVAV",
            &Attrs::new()
                .family(Family::Name(family))
                .weight(Weight::NORMAL),
            Shaping::Advanced,
            None,
        );
        buffer.shape_until_scroll(&mut system, false);
        let run = buffer.layout_runs().next().unwrap();
        assert!(run.glyphs.iter().all(|g| g.font_id == id));
        widths.push(run.line_w);
        for glyph in run.glyphs {
            let key = glyph.physical((0., run.line_y), 1.).cache_key;
            let image = cache.get_image(&mut system, key).as_ref().unwrap();
            assert!(image.data.iter().any(|x| *x != 0));
            saved.push((key, image.data.clone()));
        }
    }
    // The original fixture changes stroke thickness for opsz and advance for wdth.
    assert_eq!(widths[0], widths[1]);
    assert!(widths[2] < widths[0]);
    assert_ne!(
        saved[0].1, saved[4].1,
        "optical size must change the rendered outline"
    );
    for (key, original) in saved.into_iter().rev() {
        assert_eq!(
            cache.get_image(&mut system, key).as_ref().unwrap().data,
            original
        );
        assert_eq!(
            cache.get_image_uncached(&mut system, key).unwrap().data,
            original
        );
        let warm = cache
            .get_outline_commands(&mut system, key)
            .unwrap()
            .to_vec();
        assert_eq!(
            cache
                .get_outline_commands_uncached(&mut system, key)
                .unwrap()
                .as_ref(),
            warm.as_slice()
        );
    }
    let count = system.db().faces().count();
    assert!(
        system
            .add_font_instance(source, "body".into(), vec![], 13.)
            .is_none()
    );
    assert!(
        system
            .add_font_instance(source, "invalid".into(), vec![(*b"opsz", f32::NAN)], 13.)
            .is_none()
    );
    assert!(
        system
            .add_font_instance(source, "invalid".into(), vec![], 0.)
            .is_none()
    );
    assert_eq!(system.db().faces().count(), count);
}
