// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use std::io::{Cursor, Read, Seek, SeekFrom, Write};
use wireframe::{
    document::*,
    export_plan::Selection,
    raster::{
        self,
        plan::{Prepared, Request},
        publication::PendingOutput,
    },
};
fn plan() -> Prepared {
    let mut project = Project {
        title: "Contact layouts".into(),
        next_id: 20,
        ..Default::default()
    };
    project.pages[0].name = "Desktop".into();
    project.pages[0].content.nodes = vec![Node::control(
        2,
        "rectangle",
        Point::new(-10., 20.),
        Size::fixed(80., 40.),
    )];
    project.pages[0].alternates.push(Alternate {
        version: None,
        notes: String::new(),
        id: 3,
        name: "Compact".into(),
        content: Content {
            nodes: vec![Node::control(
                4,
                "rectangle",
                Point::new(10., 20.),
                Size::fixed(40., 60.),
            )],
            paint_order: vec![],
        },
    });
    let request = Request {
        selection: Selection::Set([3, 1].into()),
        ..Request::new(1)
    };
    Prepared::new(&project, &request).unwrap()
}
#[test]
fn archive_preserves_order_paths_dimensions_and_decodable_pngs() {
    let plan = plan();
    let mut output = Cursor::new(vec![]);
    let mut pages = vec![];
    let geometry = raster::archive::write_zip(&plan, &mut output, |page, _, _| {
        pages.push(page);
        Ok(())
    })
    .unwrap();
    assert_eq!(
        geometry
            .iter()
            .map(|g| (g.width, g.height))
            .collect::<Vec<_>>(),
        [(160, 80), (80, 120)]
    );
    let bytes = output.into_inner();
    let mut archive = zip::ZipArchive::new(Cursor::new(&bytes)).unwrap();
    assert_eq!(archive.len(), 2);
    assert!(pages.windows(2).all(|w| w[0] <= w[1]));
    for (index, (name, size)) in [
        ("Contact layouts/Desktop.png", (160, 80)),
        ("Contact layouts/Desktop (Compact).png", (80, 120)),
    ]
    .into_iter()
    .enumerate()
    {
        let mut file = archive.by_index(index).unwrap();
        assert_eq!(file.name().unwrap(), name);
        assert!(file.enclosed_name().is_some());
        assert_eq!(file.compression(), zip::CompressionMethod::Stored);
        let mut png = vec![];
        file.read_to_end(&mut png).unwrap();
        let image = image::load_from_memory(&png).unwrap().into_rgba8();
        assert_eq!(image.dimensions(), size);
        assert!(image.pixels().all(|p| p.0[3] == 255));
        let mut expected = vec![];
        raster::write_png(
            &plan.scene(index).unwrap(),
            plan.options(),
            &mut expected,
            |_, _| Ok(()),
        )
        .unwrap();
        assert_eq!(png, expected);
    }
    // Optional retained artifact for inspection with another ZIP implementation.
    // create_new deliberately prevents overwriting a previous evidence file.
    if let Some(path) = std::env::var_os("WIREFRAME_ZIP_EVIDENCE") {
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .unwrap()
            .write_all(&bytes)
            .unwrap();
    }
}
#[test]
fn archive_cancellation_preserves_destination_after_an_entry_was_encoded() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("export.zip");
    std::fs::write(&target, b"original").unwrap();
    let result = PendingOutput::prepare(target.clone(), |file| {
        raster::archive::write_zip(&plan(), file, |page, _, _| {
            if page == 1 {
                Err("Cancelled".into())
            } else {
                Ok(())
            }
        })
        .map(|_| ())
    });
    assert_eq!(result.err().unwrap(), "Cancelled");
    assert_eq!(std::fs::read(&target).unwrap(), b"original");
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
}
#[test]
fn archive_write_and_finish_failures_are_reported() {
    struct Fail {
        inner: Cursor<Vec<u8>>,
        remaining: usize,
        fail_seek: bool,
    }
    impl Write for Fail {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if self.remaining == 0 {
                return Err(std::io::Error::other("disk unavailable"));
            }
            let n = bytes.len().min(self.remaining);
            self.remaining -= n;
            self.inner.write(&bytes[..n])
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    impl Seek for Fail {
        fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
            if self.fail_seek && matches!(pos, SeekFrom::Start(_)) {
                Err(std::io::Error::other("cannot finalize archive"))
            } else {
                self.inner.seek(pos)
            }
        }
    }
    let mut complete = Cursor::new(vec![]);
    raster::archive::write_zip(&plan(), &mut complete, |_, _, _| Ok(())).unwrap();
    let final_byte_failure = raster::archive::write_zip(
        &plan(),
        Fail {
            inner: Cursor::new(vec![]),
            remaining: complete.get_ref().len() - 1,
            fail_seek: false,
        },
        |_, _, _| Ok(()),
    );
    assert!(
        final_byte_failure
            .unwrap_err()
            .starts_with("Cannot finish ZIP archive:")
    );
    for (remaining, fail_seek, message) in [
        (90, false, "disk unavailable"),
        (usize::MAX, true, "cannot finalize archive"),
    ] {
        let result = raster::archive::write_zip(
            &plan(),
            Fail {
                inner: Cursor::new(vec![]),
                remaining,
                fail_seek,
            },
            |_, _, _| Ok(()),
        );
        assert!(result.unwrap_err().contains(message));
    }
}
