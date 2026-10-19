// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{
    catalog,
    document::*,
    model::Bounds,
    render::{Drawing, Glyph, Scene},
    rich_text::structured::{Align, Grid},
    workspace::Editor,
};

fn specimen(kind: &str, text: &str, width: f32, height: f32) -> Project {
    let mut p = Project {
        next_id: 3,
        ..Project::default()
    };
    let mut node = catalog::get(kind).unwrap().create(2, Point::new(40., 50.));
    node.content = NodeContent::Control {
        kind: kind.into(),
        size: Size::fixed(width, height),
    };
    node.properties
        .insert("text".into(), Value::Text(text.into()));
    p.pages[0].content.nodes.push(node);
    p
}
fn glyphs(scene: &Scene) -> Vec<&Glyph> {
    scene
        .items
        .iter()
        .filter_map(|item| match &item.drawing {
            Drawing::Glyph(g) => Some(g),
            _ => None,
        })
        .collect()
}

#[test]
fn mixed_sizes_share_a_baseline_and_links_use_the_line_box() {
    let p = specimen("paragraph", "{size:24}B{size} [é](page:3)\nx", 200., 80.);
    let scene = Editor::new(p).rendered_scene(1).unwrap();
    let g = glyphs(&scene);
    assert_eq!(g.len(), 4);
    assert_eq!(g[0].baseline, g[2].baseline);
    assert!((g[0].baseline - (50. + 24. / 1.27)).abs() < 0.001);
    assert!((g[3].baseline - (50. + 28.8 + 13. / 1.27)).abs() < 0.001);
    assert_eq!(scene.links[0].bounds.y, 50.);
    assert_eq!(scene.links[0].bounds.h, 24.);
    assert_eq!(scene.hit_link(scene.links[0].bounds.x + 1., 74.1), None);
}

#[test]
fn styled_scene_retains_source_and_uses_real_faces_and_inline_colors() {
    let source = "*Bold* _café_ {color:#cc2200}Red{color} -dim- {size:24}Big{size}";
    let p = specimen("paragraph", source, 600., 150.);
    let before = serde_json::to_vec(&p).unwrap();
    let editor = Editor::new(p);
    let scene = editor.rendered_scene(1).unwrap();
    let glyphs = glyphs(&scene);
    assert_eq!(
        glyphs.iter().map(|g| g.text.as_str()).collect::<String>(),
        "Bold café Red dim Big"
    );
    assert!(
        glyphs
            .iter()
            .any(|g| g.text == "Bold" && g.bold && !g.italic)
    );
    assert!(glyphs.iter().any(|g| g.text == "café" && g.italic));
    assert!(
        glyphs
            .iter()
            .any(|g| g.text == "Red" && g.color == [204, 34, 0, 255])
    );
    assert!(
        glyphs
            .iter()
            .any(|g| g.text == "dim" && g.color == [153, 153, 153, 255])
    );
    assert!(glyphs.iter().any(|g| g.text == "Big" && g.size == 24.));
    assert_eq!(serde_json::to_vec(&editor.document).unwrap(), before);
}

#[test]
fn wrapped_unicode_link_regions_respect_the_visible_control_clip() {
    let p = specimen(
        "paragraph",
        "é [very long linked words across rows](page:8)",
        70.,
        30.,
    );
    let scene = Editor::new(p).rendered_scene(1).unwrap();
    assert!(scene.links.len() >= 2);
    for link in &scene.links {
        let b = link.bounds;
        assert!(b.x >= 40. && b.y >= 50. && b.x + b.w <= 110. && b.y + b.h <= 80.);
        assert_eq!(
            scene.hit_link(b.x + b.w / 2., b.y + b.h / 2.),
            Some("page:8")
        );
    }
    assert_eq!(scene.hit_link(50., 81.), None);
    assert!(
        scene
            .items
            .iter()
            .all(|i| i.clip == Some(Bounds::new(40., 50., 70., 30.)))
    );
}

#[test]
fn a_front_control_blocks_an_underlying_inline_link() {
    let mut p = specimen("paragraph", "[Continue](https://example.org)", 200., 60.);
    let scene = Editor::new(p.clone()).rendered_scene(1).unwrap();
    let b = scene.links[0].bounds;
    let (x, y) = (b.x + b.w / 2., b.y + b.h / 2.);
    assert_eq!(scene.hit_link(x, y), Some("https://example.org"));
    let mut front = catalog::get("rectangle")
        .unwrap()
        .create(3, Point::new(40., 50.));
    front.content = NodeContent::Control {
        kind: "rectangle".into(),
        size: Size::fixed(200., 60.),
    };
    p.pages[0].content.nodes.push(front);
    p.next_id = 4;
    let scene = Editor::new(p.clone()).rendered_scene(1).unwrap();
    assert_eq!(scene.hit_link(x, y), None);
    assert!(!scene.svg().contains("<a href="));
    p.pages[0].content.nodes[1].origin.x = x;
    let scene = Editor::new(p).rendered_scene(1).unwrap();
    assert_eq!(scene.hit_link(b.x + 1., y), Some("https://example.org"));
    assert_eq!(scene.hit_link(x + 1., y), None);
    assert!(scene.svg().contains("<a href="));
    assert!(
        scene
            .links
            .iter()
            .all(|link| link.bounds.x + link.bounds.w <= x)
    );
}

#[test]
fn unweighted_columns_expand_and_unspecified_alignment_inherits() {
    let grid = Grid::parse("a,b,c\n{,0,0L}");
    assert_eq!(
        grid.column_widths(&[20., 30., 50.], 200.),
        vec![40., 60., 100.]
    );
    assert_eq!(grid.columns[0].align, Align::Inherit);
    assert_eq!(grid.columns[1].align, Align::Inherit);
    assert_eq!(grid.columns[2].align, Align::Left);
    let mut p = specimen("table", "A,B,C\n{1,1,1L}", 300., 60.);
    let left = Editor::new(p.clone()).rendered_scene(1).unwrap();
    p.pages[0].content.nodes[0]
        .properties
        .insert("align".into(), Value::Choice("right".into()));
    let right = Editor::new(p).rendered_scene(1).unwrap();
    let (l, r) = (glyphs(&left), glyphs(&right));
    assert!(r[0].x > l[0].x + 50. && r[1].x > l[1].x + 50.);
    assert_eq!(l[2].x, r[2].x);
}

#[test]
fn narrow_sorted_cells_truncate_the_prefix_before_right_alignment() {
    let p = specimen("table", "[Score](page:9) ^\n{1R}", 50., 40.);
    let scene = Editor::new(p).rendered_scene(1).unwrap();
    let text: String = glyphs(&scene).iter().map(|g| g.text.as_str()).collect();
    assert!(!text.is_empty() && text.len() < "Score".len());
    assert!("Score".starts_with(&text), "Unexpected truncation: {text}");
    assert!(glyphs(&scene).iter().all(|g| g.x >= 47.));
    assert_eq!(scene.links.len(), 1);
    let b = scene.links[0].bounds;
    assert_eq!(
        scene.hit_link(b.x + b.w / 2., b.y + b.h / 2.),
        Some("page:9")
    );
    assert!(b.x + b.w <= 63.);
}

#[test]
fn taller_tables_keep_text_rows_fixed_and_fill_empty_rows() {
    let text = "Name ^,Ready\nAda,[x]\nBob,[ ]\n{60,40C}";
    let short = Editor::new(specimen("table", text, 220., 100.))
        .rendered_scene(1)
        .unwrap();
    let tall = Editor::new(specimen("table", text, 220., 230.))
        .rendered_scene(1)
        .unwrap();
    let geometry = |s: &Scene| {
        glyphs(s)
            .into_iter()
            .map(|g| (g.text.clone(), g.x, g.baseline))
            .collect::<Vec<_>>()
    };
    assert_eq!(geometry(&short), geometry(&tall));
    assert!(tall.items.len() > short.items.len());
    let text: String = glyphs(&short).iter().map(|g| g.text.as_str()).collect();
    assert_eq!(text, "NameReadyAdaBob");
    let choices = |s: &Scene| {
        s.items
            .iter()
            .filter_map(|item| match &item.drawing {
                Drawing::Icon { id, bounds, .. } => Some((id.clone(), *bounds)),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    let symbols = choices(&short);
    assert_eq!(symbols.len(), 2);
    assert_eq!(symbols[0].0, "square-check-regular");
    assert_eq!(symbols[1].0, "square-regular");
    assert!(symbols.iter().all(|(_, b)| b.w == 16. && b.h == 16.));
    assert_eq!(symbols, choices(&tall));
}

#[test]
fn svg_and_png_preserve_positioned_text_fonts_color_and_safe_links() {
    let p = specimen(
        "paragraph",
        "{color:#cc2200}Red{color} [site](https://example.org/?a=1&b=2)",
        200.,
        60.,
    );
    let scene = Editor::new(p).rendered_scene(1).unwrap();
    let svg = scene.svg();
    assert_eq!(svg.matches("data:font/ttf;base64,").count(), 4);
    assert!(svg.contains("SIL OPEN FONT LICENSE"));
    assert!(svg.contains("https://example.org/?a=1&amp;b=2"));
    for g in glyphs(&scene) {
        if let Some(text) = &g.positioned {
            assert!(svg.contains(&format!("transform=\"translate({} {})\"", g.x, g.baseline)));
            assert!(svg.contains(&text.svg(g.color)));
        } else {
            assert!(svg.contains(&format!("<text x=\"{}\" y=\"{}\"", g.x, g.baseline)));
        }
    }
    let png = scene.png().unwrap();
    let pixels = resvg::tiny_skia::Pixmap::decode_png(&png).unwrap();
    assert_eq!((pixels.width(), pixels.height()), (248, 108));
    assert!(
        pixels
            .pixels()
            .iter()
            .any(|p| p.red() > 150 && p.green() < 80 && p.blue() < 80)
    );
    assert!(
        pixels
            .pixels()
            .iter()
            .any(|p| p.blue() > 150 && p.red() < 80)
    );
}

#[test]
fn shaped_runs_preserve_original_unicode_and_styles_after_joined_text() {
    let source =
        "office ffi fi fl cafe\u{301} 👩\u{200d}💻 *Strong* [שלום](page:2) {color:red}After{color}";
    let p = specimen("paragraph", source, 1800., 100.);
    let bytes = p.encode().unwrap();
    let editor = Editor::new(p);
    let scene = editor.rendered_scene(1).unwrap();
    let runs = glyphs(&scene);
    assert_eq!(
        runs.iter().map(|g| g.text.as_str()).collect::<String>(),
        "office ffi fi fl cafe\u{301} 👩\u{200d}💻 Strong שלום After"
    );
    assert!(runs.iter().any(|g| g.text.contains("office ffi fi fl")));
    assert!(runs.iter().any(|g| g.text == "Strong" && g.bold));
    assert!(
        runs.iter()
            .any(|g| g.text == "After" && g.color == [207, 42, 39, 255])
    );
    assert!(
        runs.iter()
            .filter_map(|g| g.positioned.as_ref())
            .flat_map(|s| &s.runs)
            .flat_map(|r| &r.glyphs)
            .any(|g| matches!(&*g.ink, wireframe::text_engine::Ink::Bitmap { .. }))
    );
    assert!(scene.svg().contains("data:image/png;base64,"));
    assert!(!scene.svg().contains("<text "));
    let linked = runs.iter().find(|g| g.text == "שלום").unwrap();
    assert_eq!(scene.links.len(), 1);
    assert_eq!(scene.links[0].target, "page:2");
    assert!((scene.links[0].bounds.x - linked.x).abs() < 0.01);
    assert!((scene.links[0].bounds.w - linked.advance).abs() < 0.01);
    assert!(scene.svg().contains("office ffi fi fl cafe\u{301} "));
    assert!(scene.png().is_ok());
    assert_eq!(editor.document.encode().unwrap(), bytes);
}

#[test]
fn positioned_fallback_exports_are_independent_of_available_fonts() {
    for source in ["Ελληνικά", "👩\u{200d}💻", "ABC שלום 123", "你好世界"] {
        let scene = Editor::new(specimen("label", source, 240., 80.))
            .rendered_scene(1)
            .unwrap();
        let runs = glyphs(&scene);
        assert_eq!(runs.len(), 1);
        let positioned = runs[0].positioned.as_ref().unwrap();
        assert_eq!(positioned.source, source);
        assert!(
            positioned
                .runs
                .iter()
                .any(|r| !r.font_name.starts_with("ComicNeue"))
        );
        let svg = scene.svg();
        assert!(!svg.contains("<text "));
        // Render with an empty font database: positioned paths/bitmaps must be sufficient.
        let tree = resvg::usvg::Tree::from_str(&svg, &resvg::usvg::Options::default()).unwrap();
        let mut isolated =
            resvg::tiny_skia::Pixmap::new(tree.size().width() as u32, tree.size().height() as u32)
                .unwrap();
        resvg::render(
            &tree,
            resvg::tiny_skia::Transform::identity(),
            &mut isolated.as_mut(),
        );
        let exported = resvg::tiny_skia::Pixmap::decode_png(&scene.png().unwrap()).unwrap();
        assert_eq!(isolated.data(), exported.data());
    }
}

#[test]
fn automatic_label_includes_vertical_padding_and_preserves_requested_linebox() {
    let mut project = Project::default();
    let mut label = catalog::get("label")
        .unwrap()
        .create(2, Point::new(50., 50.));
    label
        .properties
        .insert("font_size".into(), Value::Number(24.));
    label
        .properties
        .insert("text".into(), Value::Text("Example".into()));
    project.pages[0].content.nodes.push(label);
    project.next_id = 3;
    for sketch in [false, true] {
        project.settings.sketch = sketch;
        let editor = Editor::new(project.clone());
        assert_eq!(editor.layout_page(1).unwrap().nodes[0].bounds.height, 32.);
        let scene = editor.rendered_scene(1).unwrap();
        assert!((glyphs(&scene)[0].baseline - (50. + 4. + 24. / 1.27)).abs() < 0.001);
    }
}
