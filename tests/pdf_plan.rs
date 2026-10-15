// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{
    document::{Alternate, Content, Project},
    model::{Bounds, Kind},
    pdf::{self, Destination, Options, Orientation, Paper, Selection},
    render::Drawing,
    workspace::Editor,
};
fn fixture() -> Project {
    let mut e = Editor::default();
    e.add(Kind::Rectangle, 100., 100.);
    e.selected_mut(|c| c.bounds = Bounds::new(100., 100., 200., 100.));
    e.current_mut().name = "Small linked".into();
    e.current_mut().notes = "First note.\nSecond note.".into();
    e.add_page();
    e.add(Kind::Rectangle, 100., 100.);
    e.selected_mut(|c| c.bounds = Bounds::new(100., 100., 2000., 1000.));
    e.current_mut().name = "Large sheet".into();
    e.current_mut().notes = "Large page note.".into();
    let target = e.page;
    let first = e.document.pages[0].id;
    e.switch_page(first);
    e.selection = e.document.pages[0]
        .content
        .nodes
        .iter()
        .map(|n| n.id)
        .collect();
    e.selected_mut(|c| c.link = Some(target));
    e.document
}
#[test]
fn content_and_paper_layout_keep_artwork_and_decoration_scales_separate() {
    let p = fixture();
    let before = p.encode().unwrap();
    let mut options = Options {
        margins: false,
        ..Default::default()
    };
    let plain = pdf::plan(&p, &options).unwrap();
    assert_eq!(plain[0].size, [200., 100.]);
    assert_eq!(plain[1].size, [2000., 1000.]);
    options.margins = true;
    options.names = true;
    options.numbers = true;
    options.notes = true;
    let decorated = pdf::plan(&p, &options).unwrap();
    assert_eq!(decorated[0].size, [220., 182.]);
    assert_eq!(decorated[1].size, [2020., 1068.]);
    assert_eq!(decorated[0].origin, [10., 36.]);
    options.paper = Some(Paper::A4);
    let paper = pdf::plan(&p, &options).unwrap();
    assert!((paper[0].size[0] - 841.8898).abs() < 0.001);
    assert_eq!(paper[0].scale, 1.);
    assert!((paper[1].scale - 0.4109449).abs() < 0.00001);
    for page in &paper {
        assert!(
            page.decoration
                .as_ref()
                .unwrap()
                .items
                .iter()
                .filter_map(|i| if let Drawing::Glyph(g) = &i.drawing {
                    Some(g.size)
                } else {
                    None
                })
                .all(|s| s == 12. || s == 13.)
        );
    }
    assert_eq!(before, p.encode().unwrap());
    assert!(pdf::encode("Layout", &paper).unwrap().starts_with(b"%PDF"));
}
#[test]
fn selection_order_alternates_missing_targets_and_hints_follow_exported_pages() {
    let mut p = fixture();
    let owner = p.pages[1].id;
    let alt = p.allocate_id().unwrap();
    p.pages[1].alternates.push(Alternate {
        id: alt,
        name: "Draft".into(),
        content: Content::default(),
    });
    let first = p.pages[0].id;
    let options = Options {
        selection: Selection::Set([alt, first].into()),
        hints: true,
        ..Default::default()
    };
    let pages = pdf::plan(&p, &options).unwrap();
    assert_eq!(pages.len(), 2);
    assert_eq!(pages[0].scene.title, "Small linked");
    assert_eq!(pages[1].scene.title, "Large sheet — Draft");
    assert!(matches!(
        pages[0].links[0].destination,
        Destination::Page(1)
    ));
    assert!(
        pages[0]
            .decoration
            .as_ref()
            .unwrap()
            .items
            .iter()
            .any(|i| (i.opacity - 0.15).abs() < 0.001)
    );
    let only = pdf::plan(
        &p,
        &Options {
            selection: Selection::Current(first),
            hints: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(only[0].links.is_empty());
    assert!(only[0].decoration.as_ref().unwrap().items.is_empty());
    let all = pdf::plan(
        &p,
        &Options {
            include_alternates: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(all.len(), 3);
    assert!(matches!(all[0].links[0].destination, Destination::Page(1)));
    assert!(
        pdf::plan(
            &p,
            &Options {
                selection: Selection::Current(owner + 9999),
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(
        pdf::plan(
            &p,
            &Options {
                selection: Selection::Set(Default::default()),
                ..Default::default()
            }
        )
        .is_err()
    );
}
#[test]
fn paper_orientation_empty_pages_and_note_overflow_have_explicit_results() {
    let p = fixture();
    for paper in Paper::ALL {
        for orientation in [
            Orientation::Portrait,
            Orientation::Landscape,
            Orientation::BestFit,
        ] {
            let pages = pdf::plan(
                &p,
                &Options {
                    paper: Some(paper),
                    orientation,
                    ..Default::default()
                },
            )
            .unwrap();
            assert!(pages.iter().all(|p| p.scale > 0. && p.scale <= 1.));
            assert_eq!(
                pages[0].size[0] > pages[0].size[1],
                orientation != Orientation::Portrait
            );
        }
    }
    let mut p = Project::default();
    let pages = pdf::plan(&p, &Options::default()).unwrap();
    assert!(pages[0].size.iter().all(|v| v.is_finite() && *v > 0.));
    p.pages[0].notes = "Never silently drop notes\n".repeat(100);
    let options = Options {
        paper: Some(Paper::A5),
        notes: true,
        ..Default::default()
    };
    assert!(pdf::plan(&p, &options).unwrap_err().contains("do not fit"));
    let pages = pdf::plan(
        &p,
        &Options {
            paper: None,
            ..options
        },
    )
    .unwrap();
    assert!(pages[0].size[1] > 1000.);
}
#[test]
fn background_export_owns_its_snapshot_and_preserves_destination_on_failure() {
    let d = tempfile::tempdir().unwrap();
    let path = d.path().join("export.pdf");
    std::fs::write(&path, b"prior destination").unwrap();
    let mut p = fixture();
    let failed = pdf::ExportJob::start(
        p.clone(),
        Options {
            selection: Selection::Set(Default::default()),
            ..Default::default()
        },
        path.clone(),
    )
    .unwrap();
    assert!(failed.finish().is_err());
    assert_eq!(std::fs::read(&path).unwrap(), b"prior destination");
    let job = pdf::ExportJob::start(p.clone(), Options::default(), path.clone()).unwrap();
    p.pages.clear();
    assert_eq!(job.finish().unwrap(), path);
    assert!(std::fs::read(path).unwrap().starts_with(b"%PDF"));
    assert_eq!(std::fs::read_dir(d.path()).unwrap().count(), 1);
}

#[test]
fn split_annotations_keep_one_paper_hint_per_original_link() {
    use wireframe::document::{Link, Node, Point, Size, Value};
    let mut project = fixture();
    let id = project.allocate_id().unwrap();
    let mut overlay = Node::control(
        id,
        "label",
        Point { x: 120., y: 120. },
        Size::fixed(100., 21.),
    );
    overlay
        .properties
        .insert("text".into(), Value::Text("External link".into()));
    overlay.properties.insert(
        "link".into(),
        Value::Link(Link::Url("https://example.test/export".into())),
    );
    project.pages[0].content.nodes.push(overlay);
    let pages = pdf::plan(
        &project,
        &Options {
            paper: Some(Paper::A4),
            hints: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(pages[0].links.len() > 2);
    assert_eq!(pages[0].decoration.as_ref().unwrap().items.len(), 2);
    let main = pages[0]
        .scene
        .links
        .iter()
        .filter(|l| l.target.starts_with("page:"));
    assert!(main.clone().count() > 1);
    assert!(
        main.clone()
            .all(|l| l.hint == main.clone().next().unwrap().hint)
    );
}
