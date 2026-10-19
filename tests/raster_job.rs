// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use std::{
    fs,
    path::Path,
    time::{Duration, Instant},
};
use wireframe::{
    document::*,
    export_plan::Selection,
    raster::{
        job::{Choice, Decision, Destination, ExportJob, Outcome, Phase},
        plan::Request,
    },
    workspace::Editor,
};
fn fixture() -> Editor {
    let mut p = Project {
        title: "Project".into(),
        next_id: 20,
        ..Default::default()
    };
    p.pages = (1..=3)
        .map(|id| Page {
            trashed: false,
            id,
            name: format!("Page {id}"),
            notes: String::new(),
            parent: None,
            alternates: vec![],
            content: Content {
                nodes: vec![Node::control(
                    10 + id,
                    "rectangle",
                    Point::default(),
                    Size::fixed(30. + id as f32, 20.),
                )],
                paint_order: vec![],
            },
        })
        .collect();
    Editor::new(p)
}
fn wait_for(job: &ExportJob, predicate: impl Fn(&Phase) -> bool) -> wireframe::raster::job::Status {
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        let status = job.status();
        if predicate(&status.phase) {
            return status;
        }
        assert!(
            !job.is_finished(),
            "job ended before expected phase: {status:?}"
        );
        assert!(Instant::now() < deadline, "export timed out: {status:?}");
        std::thread::sleep(Duration::from_millis(5));
    }
}
fn done(job: &ExportJob) {
    let deadline = Instant::now() + Duration::from_secs(15);
    while !job.is_finished() {
        assert!(
            Instant::now() < deadline,
            "export timed out: {:?}",
            job.status()
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}
fn conflict(job: &ExportJob) -> (u64, std::path::PathBuf) {
    let status = wait_for(job, |phase| matches!(phase, Phase::Conflict { .. }));
    let Phase::Conflict { id, path } = status.phase else {
        unreachable!()
    };
    (id, path)
}
fn originals(dir: &Path) {
    for id in 1..=3 {
        fs::write(dir.join(format!("Page {id}.png")), format!("original {id}")).unwrap();
    }
}
fn assert_originals(dir: &Path) {
    for id in 1..=3 {
        assert_eq!(
            fs::read(dir.join(format!("Page {id}.png"))).unwrap(),
            format!("original {id}").as_bytes()
        );
    }
}
#[test]
fn single_file_routes_to_png_and_many_pages_route_to_zip_from_owned_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let mut editor = fixture();
    let png = dir.path().join("single.png");
    let request = Request {
        selection: Selection::Current(1),
        ..Request::new(1)
    };
    let job = ExportJob::start(
        editor.export_snapshot(),
        request,
        Destination::File(png.clone()),
    )
    .unwrap();
    editor.document.pages[0].content.nodes.clear();
    done(&job);
    let report = job.finish();
    assert_eq!(report.outcome, Outcome::Completed);
    assert_eq!(
        (
            report.exported_pages,
            report.files.len(),
            report.remaining_pages()
        ),
        (1, 1, 0)
    );
    let image = image::open(png).unwrap();
    assert_eq!((image.width(), image.height()), (62, 40));
    let archive = dir.path().join("chosen.zip");
    let job = ExportJob::start(
        fixture().export_snapshot(),
        Request::new(1),
        Destination::File(archive.clone()),
    )
    .unwrap();
    done(&job);
    let report = job.finish();
    assert_eq!(report.outcome, Outcome::Completed);
    assert_eq!((report.exported_pages, report.files.len()), (3, 1));
    assert_eq!(
        zip::ZipArchive::new(fs::File::open(archive).unwrap())
            .unwrap()
            .len(),
        3
    );
}
#[test]
fn skip_then_keep_both_for_remaining_conflicts_keeps_originals_and_counts() {
    let dir = tempfile::tempdir().unwrap();
    originals(dir.path());
    let job = ExportJob::start(
        fixture().export_snapshot(),
        Request::new(1),
        Destination::Folder(dir.path().to_owned()),
    )
    .unwrap();
    let (first, path) = conflict(&job);
    assert!(path.ends_with("Page 1.png"));
    assert!(job.decide(
        first,
        Decision {
            choice: Choice::Skip,
            apply_to_all: false
        }
    ));
    let (second, path) = loop {
        let pair = conflict(&job);
        if pair.0 != first {
            break pair;
        }
        std::thread::yield_now();
    };
    assert!(path.ends_with("Page 2.png"));
    assert!(!job.decide(
        first,
        Decision {
            choice: Choice::Replace,
            apply_to_all: true
        }
    ));
    assert!(job.decide(
        second,
        Decision {
            choice: Choice::KeepBoth,
            apply_to_all: true
        }
    ));
    done(&job);
    let report = job.finish();
    assert_eq!(report.outcome, Outcome::Completed);
    assert_eq!(
        (
            report.exported_pages,
            report.skipped_pages,
            report.remaining_pages()
        ),
        (2, 1, 0)
    );
    assert_originals(dir.path());
    assert!(dir.path().join("Page 2_1.png").exists());
    assert!(dir.path().join("Page 3_1.png").exists());
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 5);
}
#[test]
fn cancel_while_waiting_preserves_completed_files_and_reports_remaining_pages() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("Page 2.png"), b"keep me").unwrap();
    let job = ExportJob::start(
        fixture().export_snapshot(),
        Request::new(1),
        Destination::Folder(dir.path().to_owned()),
    )
    .unwrap();
    let (id, path) = conflict(&job);
    assert!(path.ends_with("Page 2.png"));
    assert_eq!(job.status().exported_pages, 1);
    job.cancel();
    assert!(!job.decide(
        id,
        Decision {
            choice: Choice::Replace,
            apply_to_all: true
        }
    ));
    done(&job);
    let report = job.finish();
    assert_eq!(report.outcome, Outcome::Cancelled);
    assert_eq!(
        (
            report.exported_pages,
            report.skipped_pages,
            report.remaining_pages()
        ),
        (1, 0, 2)
    );
    assert!(image::open(dir.path().join("Page 1.png")).is_ok());
    assert_eq!(fs::read(dir.path().join("Page 2.png")).unwrap(), b"keep me");
    assert!(!dir.path().join("Page 3.png").exists());
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 2);
}
#[test]
fn archive_skip_counts_pages_but_does_not_claim_an_exported_file() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("project.zip");
    fs::write(&target, b"original archive").unwrap();
    let job = ExportJob::start(
        fixture().export_snapshot(),
        Request::new(1),
        Destination::File(target.clone()),
    )
    .unwrap();
    let (id, _) = conflict(&job);
    assert_eq!(job.status().exported_pages, 0);
    assert!(job.decide(
        id,
        Decision {
            choice: Choice::Skip,
            apply_to_all: true
        }
    ));
    done(&job);
    let report = job.finish();
    assert_eq!(report.outcome, Outcome::Completed);
    assert_eq!(
        (
            report.exported_pages,
            report.skipped_pages,
            report.files.len(),
            report.remaining_pages()
        ),
        (0, 3, 0, 0)
    );
    assert_eq!(fs::read(target).unwrap(), b"original archive");
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
}
#[test]
fn failure_after_one_publication_retains_its_count_and_later_pages_are_untouched() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("Page 2.png")).unwrap();
    let job = ExportJob::start(
        fixture().export_snapshot(),
        Request::new(1),
        Destination::Folder(dir.path().to_owned()),
    )
    .unwrap();
    let (id, _) = conflict(&job);
    assert!(job.decide(
        id,
        Decision {
            choice: Choice::Replace,
            apply_to_all: false
        }
    ));
    done(&job);
    let report = job.finish();
    assert!(matches!(&report.outcome,Outcome::Failed(error) if error.contains("folder")));
    assert_eq!((report.exported_pages, report.remaining_pages()), (1, 2));
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 2);
}
#[test]
fn conflict_cancel_choice_wakes_worker_and_keeps_all_originals() {
    let dir = tempfile::tempdir().unwrap();
    originals(dir.path());
    let job = ExportJob::start(
        fixture().export_snapshot(),
        Request::new(1),
        Destination::Folder(dir.path().to_owned()),
    )
    .unwrap();
    let (id, _) = conflict(&job);
    assert!(job.decide(
        id,
        Decision {
            choice: Choice::Cancel,
            apply_to_all: false
        }
    ));
    done(&job);
    let report = job.finish();
    assert_eq!(report.outcome, Outcome::Cancelled);
    assert_eq!((report.exported_pages, report.remaining_pages()), (0, 3));
    assert_originals(dir.path());
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 3);
}

#[test]
fn replace_and_skip_apply_to_all_without_reusing_a_dialog_reply() {
    for choice in [Choice::Replace, Choice::Skip] {
        let dir = tempfile::tempdir().unwrap();
        originals(dir.path());
        let job = ExportJob::start(
            fixture().export_snapshot(),
            Request::new(1),
            Destination::Folder(dir.path().to_owned()),
        )
        .unwrap();
        let (id, _) = conflict(&job);
        let decision = Decision {
            choice,
            apply_to_all: true,
        };
        assert!(job.decide(id, decision));
        assert!(!job.decide(id, decision));
        done(&job);
        let report = job.finish();
        assert_eq!(report.outcome, Outcome::Completed);
        if choice == Choice::Replace {
            assert_eq!((report.exported_pages, report.skipped_pages), (3, 0));
            for id in 1..=3 {
                assert!(image::open(dir.path().join(format!("Page {id}.png"))).is_ok());
            }
        } else {
            assert_eq!((report.exported_pages, report.skipped_pages), (0, 3));
            assert_originals(dir.path());
        }
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 3);
    }
}
#[test]
fn dropping_a_job_releases_its_conflict_wait_and_discards_pending_output() {
    let dir = tempfile::tempdir().unwrap();
    originals(dir.path());
    let job = ExportJob::start(
        fixture().export_snapshot(),
        Request::new(1),
        Destination::Folder(dir.path().to_owned()),
    )
    .unwrap();
    conflict(&job);
    drop(job);
    let deadline = Instant::now() + Duration::from_secs(15);
    while fs::read_dir(dir.path()).unwrap().count() != 3 {
        assert!(
            Instant::now() < deadline,
            "abandoned export did not discard its temporary file"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_originals(dir.path());
}

#[test]
fn dimension_reduction_is_reported_for_the_published_page() {
    let mut editor = fixture();
    editor.document.pages.truncate(1);
    let NodeContent::Control { size, .. } = &mut editor.document.pages[0].content.nodes[0].content
    else {
        unreachable!()
    };
    *size = Size::fixed(10001., 1.);
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("wide.png");
    let job = ExportJob::start(
        editor.export_snapshot(),
        Request::new(1),
        Destination::File(target.clone()),
    )
    .unwrap();
    done(&job);
    let report = job.finish();
    assert_eq!(report.outcome, Outcome::Completed);
    assert_eq!(report.reduced_pages.len(), 1);
    assert_eq!(report.reduced_pages[0].name, "Page 1");
    assert_eq!(report.reduced_pages[0].width, 20000);
    assert_eq!(image::open(target).unwrap().width(), 20000);
}
