// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use eframe::egui;
use wireframe::{
    layout::{EguiMeasurer, TextMeasurer, TextStyle},
    rich_text::{
        Inline, RichText,
        structured::{Align, CellContent, Choice, Grid, Sort, tree},
    },
};

#[test]
fn line_boxes_follow_font_sizes_and_spacing_independently_of_font_metrics() {
    let ctx = egui::Context::default();
    wireframe::fonts::install(&ctx);
    ctx.run_ui(Default::default(), |_| {
        let measure = EguiMeasurer {
            context: &ctx,
            regular: wireframe::fonts::family(false, false),
            bold: wireframe::fonts::family(true, false),
        };
        let style = TextStyle {
            size: 13.,
            bold: false,
            italic: false,
        };
        for (source, height, rows) in [
            ("One", 13., 1),
            ("One\nTwo", 29., 2),
            ("One\n\nThree\n", 60., 4),
            ("{size:24}Large{size}\nSmall", 42., 2),
            ("{size:8}Small{size}", 8., 1),
            ("", 13., 1),
        ] {
            let text = RichText::parse(source, false);
            let result = measure.measure_rich(&text, style, None, true);
            assert_eq!((result.height, result.rows), (height, rows), "{source}");
            assert_eq!(result.width.fract(), 0.);
        }
        let text = RichText::parse("{size:24}Big{size} [é](page:2)\n[Next](page:3)", false);
        let painted = measure.rich_galley(&text, style, None, true);
        assert_eq!(painted.rows[0].height, 24.);
        assert!((painted.rows[1].top - 28.8).abs() < 0.001);
        assert!((painted.rows[0].baseline - 24. / 1.27).abs() < 0.001);
        assert_eq!(painted.links[0].rect.height(), 24.);
        assert_eq!(painted.links[0].row, 0);
        assert_eq!(painted.links[1].row, 1);
        assert!((painted.links[1].rect.top() - 28.8).abs() < 0.001);
        let wrapped = measure.rich_galley(
            &RichText::plain("one two three four"),
            style,
            Some(30.),
            true,
        );
        assert!(wrapped.rows.len() >= 3);
        for pair in wrapped.rows.windows(2) {
            assert!((pair[1].top - pair[0].top - 15.6).abs() < 0.001);
        }
    })
    .drop_without_applying_deltas();
}

#[test]
fn nested_styles_escape_sequences_and_unicode_preserve_source() {
    let source = r"*Strong _café_* &under& ~cut~ -dim- \*literal\*";
    let parsed = RichText::parse(source, false);
    assert_eq!(parsed.source, source);
    assert_eq!(
        parsed.display_text(true),
        "Strong café under cut dim *literal*"
    );
    let spans: Vec<_> = parsed
        .items
        .iter()
        .filter_map(|v| {
            if let Inline::Span { text, style } = v {
                Some((text, style))
            } else {
                None
            }
        })
        .collect();
    assert!(
        spans
            .iter()
            .any(|(text, s)| *text == "café" && s.bold && s.italic)
    );
    assert!(
        spans
            .iter()
            .any(|(text, s)| *text == "dim" && s.disabled && !s.strike)
    );
    assert!(spans.iter().any(|(text, s)| *text == "cut" && s.strike));
}
#[test]
fn incomplete_formatting_and_spaced_delimiters_remain_literal() {
    for source in [
        "* spaced *",
        "*unclosed",
        "**",
        "[ ] option",
        "{size:-1}word{size}",
        "{color:red}unclosed",
    ] {
        assert_eq!(RichText::parse(source, false).display_text(true), source);
    }
}
#[test]
fn tags_links_and_distinct_line_breaks_have_semantics() {
    let parsed = RichText::parse(
        "{SIZE:24}big{size} {Colour:#0f0}green{colour} [site](https://example.org)\\nnext\nlast",
        false,
    );
    assert_eq!(parsed.display_text(true), "big green site\nnext\nlast");
    assert_eq!(parsed.display_text(false), "big green sitenext\nlast");
    assert!(
        parsed
            .items
            .iter()
            .any(|v| matches!(v,Inline::Span{text,style} if text=="big" && style.size==Some(24)))
    );
    assert!(parsed.items.iter().any(|v| matches!(v,Inline::Span{text,style} if text=="site" && style.target.as_deref()==Some("https://example.org") && style.link)));
}
#[test]
fn bullet_recognition_is_explicit_and_column_sensitive() {
    let source = "* One\n- Two\n  * indented\n* bold*";
    assert_eq!(
        RichText::parse(source, true).display_text(true),
        "• One\n• Two\n  * indented\n• bold*"
    );
    assert!(
        !RichText::parse(source, false)
            .items
            .contains(&Inline::Bullet)
    );
}
#[test]
fn adversarial_unclosed_tags_have_bounded_work_and_preserve_text() {
    for source in ["{".repeat(100_000), "[".repeat(100_000)] {
        let parsed = RichText::parse(&source, false);
        if source.starts_with('{') {
            assert!(parsed.complexity_limited);
        } else {
            assert!(
                !parsed.complexity_limited,
                "Invalid opening boundaries are skipped in linear time"
            );
        }
        assert_eq!(parsed.display_text(true), source);
    }
}
#[test]
fn grid_cells_sort_choices_and_column_weights_are_separate_from_inline_text() {
    let grid =
        Grid::parse("Item ^, Amount ^v, Ready\nOne\\, two,*42*,[x]\nThree,,[ ]\n{60L,20R,20C}");
    assert_eq!(grid.rows.len(), 3);
    assert_eq!(grid.columns.len(), 3);
    assert_eq!(grid.rows[0][0].sort, Some(Sort::Ascending));
    assert_eq!(grid.rows[0][1].sort, Some(Sort::Both));
    assert_eq!(grid.columns[1].align, Align::Right);
    assert_eq!(grid.columns[2].align, Align::Center);
    assert!(
        matches!(&grid.rows[1][0].content,CellContent::Text(t) if t.display_text(true)=="One, two")
    );
    assert_eq!(
        grid.rows[1][2].content,
        CellContent::Choice(Choice::Checked)
    );
    assert_eq!(
        grid.rows[2][2].content,
        CellContent::Choice(Choice::Unchecked)
    );
    assert_eq!(
        grid.column_widths(&[50., 30., 20.], 200.),
        vec![120., 40., 40.]
    );
    let mixed = Grid::parse("a,b,c\n{1,0,1}");
    assert_eq!(
        mixed.column_widths(&[10., 40., 20.], 200.),
        vec![80., 40., 80.]
    );
    assert_eq!(Grid::parse("A,,C").rows[0].len(), 3);
    assert_eq!(
        Grid::parse("[link](https://example.org/a,b), x").rows[0].len(),
        3
    );
}
#[test]
fn tree_indentation_and_markers_require_token_boundaries() {
    let rows = tree("f Root\n. [+] Branch\n..[x] Leaf\n [x]literal");
    assert_eq!(rows[0].marker.as_deref(), Some("folder"));
    assert_eq!(rows[1].depth, 2);
    assert_eq!(rows[1].marker.as_deref(), Some("expand"));
    assert_eq!(rows[2].marker.as_deref(), Some("checked"));
    assert_eq!(rows[3].marker, None);
}
#[test]
fn font_measurement_and_link_areas_use_the_same_wrapped_galley() {
    let context = egui::Context::default();
    let output = context.run_ui(egui::RawInput::default(), |ui| {
        let measurer = EguiMeasurer {
            context: ui.ctx(),
            regular: egui::FontFamily::Proportional,
            bold: egui::FontFamily::Proportional,
        };
        let style = TextStyle {
            size: 13.,
            bold: false,
            italic: false,
        };
        let text = RichText::parse(
            "é [long linked words here](https://example.org)\n[second](page:2)",
            false,
        );
        let painted = measurer.rich_galley(&text, style, Some(70.), true);
        let metrics = measurer.measure_rich(&text, style, Some(70.), true);
        assert_eq!(metrics.width, painted.size.x);
        assert_eq!(metrics.height, painted.size.y);
        assert!(metrics.rows > 2);
        assert!(
            painted
                .links
                .iter()
                .filter(|l| l.target == "https://example.org")
                .count()
                > 1
        );
        assert!(painted.links.iter().any(|l| l.target == "page:2"));
        assert!(
            painted
                .links
                .iter()
                .all(|l| l.rect.is_finite() && l.rect.width() > 0.)
        );
        let large = measurer.measure_rich(
            &RichText::parse("{size:32}Wide{size}", false),
            style,
            None,
            true,
        );
        let small = measurer.measure("Wide", style, None);
        assert!(large.width > small.width * 2. && large.height > small.height * 2.);
    });
    output.drop_without_applying_deltas();
}

#[test]
fn wrapping_preserves_source_graphemes_even_when_glyph_metadata_expands() {
    use unicode_segmentation::UnicodeSegmentation;
    let ctx = egui::Context::default();
    wireframe::fonts::install(&ctx);
    ctx.run_ui(Default::default(), |_| {
        let m = EguiMeasurer {
            context: &ctx,
            regular: wireframe::fonts::family(false, false),
            bold: wireframe::fonts::family(true, false),
        };
        let style = TextStyle {
            size: 24.,
            bold: false,
            italic: false,
        };
        for source in [
            "مرحبا بالعالم مرحبا بالعالم",
            "ABC שלום 123 שלום",
            "👩\u{200d}💻 a\u{301}\u{323} office ffi fi fl",
        ] {
            for width in [1., 30., 70., 200.] {
                let text = m.rich_galley(&RichText::plain(source), style, Some(width), true);
                assert!(text.error.is_none(), "{:?}", text.error);
                let actual: String = text
                    .runs
                    .iter()
                    .flatten()
                    .map(|r| r.text.as_str())
                    .collect();
                assert_eq!(actual, source, "width {width}");
                let mut boundary = 0;
                let valid: Vec<_> = source
                    .grapheme_indices(true)
                    .map(|(i, _)| i)
                    .chain(std::iter::once(source.len()))
                    .collect();
                for row in text.runs {
                    boundary += row.iter().map(|r| r.text.len()).sum::<usize>();
                    assert!(
                        valid.contains(&boundary),
                        "split grapheme at {boundary}: {source}"
                    );
                }
            }
        }
    })
    .drop_without_applying_deltas();
}

#[test]
fn truncation_keeps_whole_graphemes_and_reshapes_the_prefix() {
    use wireframe::rich_text::painting::TextRun;
    let ctx = egui::Context::default();
    wireframe::fonts::install(&ctx);
    ctx.run_ui(Default::default(), |_| {
        let format = egui::text::TextFormat {
            font_id: egui::FontId::new(24., wireframe::fonts::family(false, false)),
            ..Default::default()
        };
        let measure = |s: &str| {
            let mut job = egui::text::LayoutJob::default();
            job.append(s, 0., format.clone());
            ctx.fonts_mut(|f| f.layout_job(job).size().x)
        };
        for cluster in ["e\u{301}", "👩\u{200d}💻"] {
            let source = format!("{cluster}ABC");
            let mut run = TextRun {
                text: source.clone(),
                format: format.clone(),
                x: 0.,
                width: measure(&source),
                face: "canvas-regular".into(),
                face_scale: 1.,
                positioned: None,
            };
            run.truncate(&ctx, measure(cluster) + 0.01).unwrap();
            assert_eq!(run.text, cluster);
            assert!((run.width - measure(cluster)).abs() < 0.01);
            run.truncate(&ctx, 0.).unwrap();
            assert!(run.text.is_empty());
            assert_eq!(run.width, 0.);
        }
    })
    .drop_without_applying_deltas();
}

#[test]
fn wrap_uses_the_selected_engine_and_link_targets_can_share_a_style() {
    let ctx = egui::Context::default();
    wireframe::fonts::install(&ctx);
    ctx.run_ui(Default::default(), |_| {
        let m = EguiMeasurer {
            context: &ctx,
            regular: wireframe::fonts::family(false, false),
            bold: wireframe::fonts::family(true, false),
        };
        let style = TextStyle {
            size: 13.,
            bold: false,
            italic: false,
        };
        let painted = m.rich_galley(
            &RichText::parse(
                "one two three [first](page:2) [second](page:3) four five",
                false,
            ),
            style,
            Some(70.),
            true,
        );
        assert!(painted.error.is_none(), "{:?}", painted.error);
        if cfg!(target_os = "macos") {
            for (runs, row) in painted.runs.iter().zip(&painted.rows) {
                let total: f64 = runs
                    .iter()
                    .map(|r| r.positioned.as_ref().unwrap().width)
                    .sum();
                assert_eq!(total.ceil() as f32, row.width);
            }
        } else {
            assert_eq!(painted.rows.len(), painted.galley.rows.len());
            for (source, actual) in painted.galley.rows.iter().zip(&painted.rows) {
                assert_eq!(source.size.x.ceil(), actual.width);
            }
        }
        assert!(painted.links.iter().any(|l| l.target == "page:2"));
        assert!(painted.links.iter().any(|l| l.target == "page:3"));
        for pair in painted.links.windows(2) {
            if pair[0].row == pair[1].row {
                assert!(pair[0].rect.right() <= pair[1].rect.left() + 0.01);
            }
        }
        let adjacent = m.rich_galley(
            &RichText {
                source: "ffi".into(),
                items: [("f", "page:2"), ("fi", "page:3")]
                    .into_iter()
                    .map(|(text, target)| Inline::Span {
                        text: text.into(),
                        style: wireframe::rich_text::Style {
                            link: true,
                            target: Some(target.into()),
                            ..Default::default()
                        },
                    })
                    .collect(),
                complexity_limited: false,
            },
            style,
            None,
            true,
        );
        assert_eq!(adjacent.galley.job.sections.len(), 2);
        assert_eq!(adjacent.links.len(), 2);
        assert!(adjacent.links.iter().all(|link| link.rect.width() > 0.));
        assert!((adjacent.links[0].rect.right() - adjacent.links[1].rect.left()).abs() < 0.01);
        let source = "مرحبا ".repeat(256);
        let narrow = m.rich_galley(&RichText::plain(&source), style, Some(1.), true);
        assert_eq!(
            narrow
                .runs
                .iter()
                .flatten()
                .map(|r| r.text.as_str())
                .collect::<String>(),
            source
        );
    })
    .drop_without_applying_deltas();
}

#[test]
fn formatting_boundaries_survive_equal_styles_and_trailing_spaces() {
    use wireframe::rich_text::Style;
    let spans = |s: &str| {
        RichText::parse(s, false)
            .items
            .into_iter()
            .filter_map(|item| match item {
                Inline::Span { text, style } => Some((text, style)),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    let bold = Style {
        bold: true,
        ..Default::default()
    };
    assert_eq!(
        spans("*left *right"),
        vec![
            ("left ".into(), bold.clone()),
            ("right".into(), Style::default())
        ]
    );
    assert_eq!(
        spans("*a**b*"),
        vec![("a".into(), bold.clone()), ("b".into(), bold)]
    );
    assert_eq!(spans("{size:24}a{size}{size:24}b{size}").len(), 2);
    for source in [
        "f[i]",
        "[[x]",
        "a_b_",
        "a~b~",
        "a-b-",
        ".[x]",
        "é[x]",
        "\u{00a0}[x]",
    ] {
        assert_eq!(RichText::parse(source, false).display_text(true), source);
        assert!(spans(source).iter().all(|(_, s)| s == &Style::default()));
    }
    for source in ["[x]", " ([x])", ") [x]", "\t[x]", "\n[x]", "line\\n[x]"] {
        assert!(spans(source).iter().any(|(_, s)| s.link), "{source}");
    }
    let adjacent = RichText::parse("[one](page:2)[two](page:3)", false);
    assert_eq!(adjacent.display_text(true), "one[two](page:3)");
    assert_eq!(
        adjacent
            .items
            .iter()
            .filter(|i| matches!(i, Inline::Span { style, .. } if style.target.is_some()))
            .count(),
        1
    );
}

#[cfg(target_os = "macos")]
#[test]
fn positioned_rich_runs_share_measurement_truncation_and_bidi_link_bounds() {
    let ctx = egui::Context::default();
    wireframe::fonts::install(&ctx);
    ctx.run_ui(Default::default(), |_| {
        let m = EguiMeasurer {
            context: &ctx,
            regular: wireframe::fonts::family(false, false),
            bold: wireframe::fonts::family(true, false),
        };
        let style = TextStyle {
            size: 24.,
            bold: false,
            italic: false,
        };
        for source in [
            "ABC [שלום 123](page:2) end",
            "[مرحبا ABC 123](page:2)",
            "*a**b*",
            "{size:24}f{size}i",
            "👩‍💻ABC",
        ] {
            let painted = m.rich_galley(&RichText::parse(source, false), style, None, true);
            assert!(painted.error.is_none(), "{:?}", painted.error);
            let runs = &painted.runs[0];
            let mut x = 0.;
            for run in runs {
                let ink = run.positioned.as_ref().unwrap();
                assert_eq!(run.text, ink.source);
                assert!((f64::from(run.x) - x).abs() < 0.0001);
                x += ink.width;
            }
            assert_eq!(painted.rows[0].width, x.ceil() as f32);
            for link in &painted.links {
                let run = runs.iter().find(|r| r.x == link.rect.left()).unwrap();
                assert!(link.rect.width() > 0.);
                assert!((link.rect.width() - run.width).abs() < 0.0001);
            }
            if source == "👩‍💻ABC" {
                let mut run = runs[0].clone();
                let single = wireframe::text_engine::shape_cached(&ctx, &run.format.font_id, "👩‍💻")
                    .unwrap()
                    .unwrap();
                run.truncate(&ctx, single.width as f32 + 0.01).unwrap();
                assert_eq!(run.text, "👩‍💻");
                assert_eq!(run.positioned.as_ref().unwrap().source, run.text);
                assert!((run.width - single.width as f32).abs() < 0.001);
                run.truncate(&ctx, 0.).unwrap();
                assert!(run.text.is_empty());
                assert_eq!(run.positioned.as_ref().unwrap().width, 0.);
            }
        }
    })
    .drop_without_applying_deltas();
}

#[cfg(target_os = "macos")]
#[test]
fn positioned_layout_reports_limits_without_legacy_fallback() {
    let ctx = egui::Context::default();
    wireframe::fonts::install(&ctx);
    ctx.run_ui(Default::default(), |_| {
        let m = EguiMeasurer {
            context: &ctx,
            regular: wireframe::fonts::family(false, false),
            bold: wireframe::fonts::family(true, false),
        };
        let style = TextStyle {
            size: 24.,
            bold: false,
            italic: false,
        };
        let oversized = RichText::plain(&"a".repeat(wireframe::text_engine::MAX_TEXT_BYTES + 1));
        let failed = m.rich_galley(&oversized, style, None, true);
        assert!(failed.error.unwrap().contains("64 KB"));
        assert!(failed.runs.is_empty());
        assert!(!failed.size.is_finite());
        let invalid_size = m.rich_galley(
            &RichText::plain("text"),
            TextStyle {
                size: 513.,
                ..style
            },
            None,
            true,
        );
        assert!(invalid_size.error.is_some());
        assert!(invalid_size.runs.is_empty());
    })
    .drop_without_applying_deltas();
}
