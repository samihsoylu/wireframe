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
        assert!(parsed.complexity_limited);
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
        assert_eq!(metrics.width, painted.galley.size().x);
        assert_eq!(metrics.height, painted.galley.size().y);
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
