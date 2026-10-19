// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use std::sync::Arc;
use wireframe::{
    catalog,
    document::*,
    fonts::{self, ProjectFont},
    workspace::Editor,
};

fn font() -> Arc<ProjectFont> {
    Arc::new(
        ProjectFont::new(
            "Test family".into(),
            [
                fonts::BOLD_ITALIC,
                fonts::ITALIC,
                fonts::BOLD,
                fonts::REGULAR,
            ]
            .map(<[u8]>::to_vec),
            "Retained <notice> & attribution".into(),
        )
        .unwrap(),
    )
}
fn project() -> Project {
    // This fixture represents an existing document with no embedded font.
    let mut p = Project {
        font: None,
        ..Project::default()
    };
    let mut label = catalog::get("label").unwrap().create(2, Point::new(0., 0.));
    label
        .properties
        .insert("text".into(), Value::Text("Checkbox".into()));
    p.pages[0].content.nodes.push(label);
    let mut second = p.pages[0].clone();
    second.id = 3;
    p.pages.push(second);
    p.next_id = 4;
    p
}

#[test]
fn new_projects_embed_distinct_default_faces_without_restyling_old_files() {
    let new = Project::default();
    let family = new.font.as_ref().unwrap();
    assert_eq!(family.name(), "Wireframe Draft");
    let faces = family.faces();
    for i in 0..4 {
        ProjectFont::inspect_face(faces[i]).unwrap();
        for other in &faces[i + 1..] {
            assert_ne!(faces[i], *other);
        }
    }
    assert!(
        family
            .notices()
            .contains("Copyright 2014 The Comic Neue Project Authors")
    );
    assert!(family.notices().contains("SIL OPEN FONT LICENSE"));
    assert!(family.notices().contains("Renamed Wireframe Draft"));
    let saved = new.encode().unwrap();
    assert_eq!(decode(&saved).unwrap(), new);
    assert_eq!(decode(&saved).unwrap().encode().unwrap(), saved);
    let old = project().encode().unwrap();
    assert!(decode(&old).unwrap().font.is_none());
    assert_eq!(decode(&old).unwrap().encode().unwrap(), old);
    let second = Project::default();
    assert!(Arc::ptr_eq(family, second.font.as_ref().unwrap()));
}

#[test]
fn project_fonts_refresh_all_pages_and_survive_history_and_storage() {
    let mut e = Editor::new(project());
    let original = e.document.clone();
    let old_scene = e.rendered_scene(1).unwrap();
    let old_width = old_scene.bounds.w;
    assert_eq!(e.rendered_scene(3).unwrap().bounds.w, old_width);
    let defaults = eframe::egui::FontDefinitions::default();
    let sans = defaults.font_data["Ubuntu-Light"].font.to_vec();
    let imported_font = Arc::new(
        ProjectFont::new(
            "Test sans".into(),
            std::array::from_fn(|_| sans.clone()),
            String::new(),
        )
        .unwrap(),
    );
    e.edit("Change project font", |e| {
        e.document.font = Some(imported_font)
    });
    let imported = e.document.clone();
    let new_scene = e.rendered_scene(1).unwrap();
    assert_ne!(new_scene.bounds.w, old_width);
    assert_eq!(e.rendered_scene(3).unwrap().bounds.w, new_scene.bounds.w);
    assert!(Arc::ptr_eq(
        new_scene.font.as_ref().unwrap(),
        imported.font.as_ref().unwrap()
    ));
    assert!(
        old_scene.font.is_none(),
        "A retained scene owns its original font choice"
    );
    let encoded = imported.encode().unwrap();
    let reopened = decode(&encoded).unwrap();
    assert_eq!(reopened, imported);
    assert_eq!(
        Editor::new(reopened).rendered_scene(1).unwrap().svg(),
        new_scene.svg()
    );
    e.undo();
    assert_eq!(e.document, original);
    assert_eq!(e.rendered_scene(1).unwrap().svg(), old_scene.svg());
    assert_eq!(e.rendered_scene(3).unwrap().bounds.w, old_width);
    e.redo();
    assert_eq!(e.document, imported);
    assert_eq!(e.rendered_scene(1).unwrap().svg(), new_scene.svg());
    e.edit("Use bundled font", |e| e.document.font = None);
    assert_eq!(e.rendered_scene(1).unwrap().svg(), old_scene.svg());
    e.undo();
    assert_eq!(e.rendered_scene(1).unwrap().svg(), new_scene.svg());
}

#[test]
fn explicit_face_roles_match_measurement_and_png_even_when_metadata_differs() {
    let imported = font();
    for (bold, italic) in [(false, false), (true, false), (false, true), (true, true)] {
        let mut expected = project();
        expected.pages[0].content.nodes[0]
            .properties
            .insert("bold".into(), Value::Bool(!bold));
        expected.pages[0].content.nodes[0]
            .properties
            .insert("italic".into(), Value::Bool(!italic));
        let mut actual = project();
        actual.font = Some(imported.clone());
        actual.pages[0].content.nodes[0]
            .properties
            .insert("bold".into(), Value::Bool(bold));
        actual.pages[0].content.nodes[0]
            .properties
            .insert("italic".into(), Value::Bool(italic));
        let expected = Editor::new(expected).rendered_scene(1).unwrap();
        let actual = Editor::new(actual).rendered_scene(1).unwrap();
        assert_eq!(actual.bounds, expected.bounds);
        assert_eq!(
            actual.png().unwrap(),
            expected.png().unwrap(),
            "role {bold}/{italic}"
        );
    }
}

#[test]
fn embedded_svg_faces_and_notices_are_portable() {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    let mut p = project();
    let font = font();
    p.font = Some(font.clone());
    let svg = Editor::new(p).rendered_scene(1).unwrap().svg();
    assert_eq!(svg.matches("@font-face").count(), 4);
    for face in font.faces() {
        assert!(svg.contains(&STANDARD.encode(face)));
    }
    assert!(svg.contains("<path"));
    assert!(!svg.contains("<text "));
    assert!(svg.contains("aria-label=\"Checkbox\""));
    assert!(svg.contains("Retained &lt;notice&gt; &amp; attribution"));
    assert!(!svg.contains("file:"));
}

#[test]
fn malformed_fonts_fail_at_the_document_boundary_and_old_projects_keep_defaults() {
    let old = project().encode().unwrap();
    assert!(!String::from_utf8_lossy(&old).contains("\"font\":"));
    assert!(decode(&old).unwrap().font.is_none());
    let mut p = project();
    p.font = Some(font());
    let mut json = serde_json::to_value(&p).unwrap();
    json["font"]["faces"][2] = "AAAA".into();
    assert!(decode(&serde_json::to_vec(&json).unwrap()).is_err());
    json["font"]["faces"][2] = "not base64!".into();
    assert!(decode(&serde_json::to_vec(&json).unwrap()).is_err());
    assert!(
        ProjectFont::inspect_face(b"ttcf0000")
            .unwrap_err()
            .contains("collection")
    );
    assert!(ProjectFont::inspect_face(&vec![0; fonts::MAX_FACE_BYTES + 1]).is_err());
    assert!(
        ProjectFont::new(
            "\n".into(),
            font().faces().map(<[u8]>::to_vec),
            String::new()
        )
        .is_err()
    );
}

#[test]
fn default_cedilla_letters_preserve_case_and_canonical_forms() {
    let font = fonts::default_project_font();
    for (role, face) in font.faces().into_iter().enumerate() {
        for (composed, decomposed, base, opposite) in
            [("ţ", "t\u{0327}", "t", "Ț"), ("Ţ", "T\u{0327}", "T", "ț")]
        {
            let shape = |s: &str| wireframe::text_engine::shape(face, s, 24.).unwrap();
            let a = shape(composed);
            let b = shape(decomposed);
            assert!(
                (a.width - shape(base).width).abs() < 0.01,
                "role {role}: {composed} must retain its base letter's advance"
            );
            assert_ne!(
                a.svg([0, 0, 0, 255]),
                shape(opposite).svg([0, 0, 0, 255]),
                "role {role}: {composed} must retain case"
            );
            assert_eq!(a.svg([0, 0, 0, 255]), b.svg([0, 0, 0, 255]));
            assert_eq!(a.width, b.width);
            assert!(
                a.runs
                    .iter()
                    .all(|r| r.font_name.contains("WireframeDraft")),
                "The default face must supply the character without system substitution"
            );
            for accent in ["\u{0301}", "\u{0308}"] {
                let a = shape(&format!("{composed}{accent}"));
                let b = shape(&format!("{decomposed}{accent}"));
                assert_eq!(a.svg([0, 0, 0, 255]), b.svg([0, 0, 0, 255]));
            }
        }
        // A new below-mark glyph retains pair positioning on both sides.
        for (new, existing) in [("Ţ", "Ț"), ("ţ", "ț")] {
            for neighbor in ["A", "V", "T", "a", "o", "y", ".", ","] {
                for (a, b) in [
                    (format!("{new}{neighbor}"), format!("{existing}{neighbor}")),
                    (format!("{neighbor}{new}"), format!("{neighbor}{existing}")),
                ] {
                    let a = wireframe::text_engine::shape(face, &a, 24.).unwrap();
                    let b = wireframe::text_engine::shape(face, &b, 24.).unwrap();
                    assert!(
                        (a.width - b.width).abs() < 0.01,
                        "role {role}: pair advances differ for {new}/{existing} near {neighbor}"
                    );
                }
            }
        }
    }
}

#[test]
fn default_breve_letters_use_project_font_and_preserve_canonical_positioning() {
    let font = fonts::default_project_font();
    for (role, face) in font.faces().into_iter().enumerate() {
        let shape = |s: &str| wireframe::text_engine::shape(face, s, 24.).unwrap();
        for (composed, decomposed, base) in [("ğ", "g\u{0306}", "g"), ("Ğ", "G\u{0306}", "G")] {
            let a = shape(composed);
            assert!(
                a.runs
                    .iter()
                    .all(|r| r.font_name.contains("WireframeDraft")),
                "role {role}: {composed} must use the project font, got {:?}",
                a.runs.iter().map(|r| &r.font_name).collect::<Vec<_>>()
            );
            assert!((a.width - shape(base).width).abs() < 0.01);
            for (prefix, suffix) in [
                ("", ""),
                ("A", "V"),
                ("T", "a"),
                ("o", "y"),
                ("", "\u{0301}"),
                ("", "\u{0327}"),
                ("", "\u{0328}"),
                ("", "\u{0327}\u{0301}"),
            ] {
                let a = shape(&format!("{prefix}{composed}{suffix}"));
                let b = shape(&format!("{prefix}{decomposed}{suffix}"));
                assert_eq!(
                    a.svg([0, 0, 0, 255]),
                    b.svg([0, 0, 0, 255]),
                    "role {role}: {prefix}{composed}{suffix}"
                );
                assert!((a.width - b.width).abs() < 1e-9);
                if suffix.is_ascii() {
                    let base = shape(&format!("{prefix}{base}{suffix}"));
                    assert!((a.width - base.width).abs() < 0.01);
                }
            }
        }
        // Above accents with the same combining class keep their source order.
        assert_ne!(
            shape("g\u{0301}\u{0306}").svg([0, 0, 0, 255]),
            shape("g\u{0306}\u{0301}").svg([0, 0, 0, 255])
        );
        let name = shape("Çağrı GÜNEŞ");
        assert!(
            name.runs
                .iter()
                .all(|r| r.font_name.contains("WireframeDraft"))
        );
    }
}

#[test]
fn default_operators_are_legible_and_space_variants_have_equal_advance() {
    let font = fonts::default_project_font();
    for face in font.faces() {
        for size in [12., 17., 28.] {
            let shape = |text: &str| wireframe::text_engine::shape(face, text, size).unwrap();
            let x = shape("x").raster([0, 0, 0, 255], 2.).unwrap().unwrap();
            for symbol in ["+", "×"] {
                let shaped = shape(symbol);
                let ink = shaped.raster([0, 0, 0, 255], 2.).unwrap().unwrap();
                assert!((shaped.width / size - 0.65).abs() < 0.002);
                assert!(ink.bounds.height() >= x.bounds.height() * 0.9);
                assert!(ink.bounds.height() <= x.bounds.height() * 1.1);
                assert!(
                    shaped
                        .runs
                        .iter()
                        .all(|r| r.font_name.contains("WireframeDraft"))
                );
            }
            for (ordinary, protected) in [
                ("Price 24.50", "Price\u{a0}24.50"),
                ("2 × 19.95", "2\u{a0}×\u{a0}19.95"),
            ] {
                let a = shape(ordinary);
                let b = shape(protected);
                // The portable shaper accumulates word advances in f32.
                // Keep the native bound and require identical rendered pixels
                // below; only the scalar total permits portable rounding.
                let tolerance = if cfg!(target_os = "macos") {
                    1e-9
                } else {
                    2. * f64::from(f32::EPSILON) * a.width.max(b.width).max(1.)
                };
                assert!(
                    (a.width - b.width).abs() < tolerance,
                    "{ordinary:?} at {size}: {} versus {} (tolerance {tolerance})",
                    a.width,
                    b.width
                );
                let a = a.raster([0, 0, 0, 255], 2.).unwrap().unwrap();
                let b = b.raster([0, 0, 0, 255], 2.).unwrap().unwrap();
                assert_eq!(a.bounds, b.bounds);
                assert_eq!(a.pixels.size, b.pixels.size);
                assert_eq!(a.pixels.pixels, b.pixels.pixels);
            }
        }
    }
}
