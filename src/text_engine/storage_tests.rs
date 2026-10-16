use super::*;
fn tables() -> (Vec<(u32, Vec<u8>)>, usize) {
    let font = skrifa::FontRef::new(crate::fonts::REGULAR).unwrap();
    let tables: Vec<_> = font
        .table_directory
        .table_records()
        .iter()
        .map(|r| {
            let tag = r.tag();
            (
                u32::from_be_bytes(tag.to_be_bytes()),
                font.table_data(tag).unwrap().as_bytes().to_vec(),
            )
        })
        .collect();
    let size = 12
        + 16 * tables.len()
        + tables
            .iter()
            .map(|(_, b)| (b.len() + 3) & !3)
            .sum::<usize>();
    (tables, size)
}
#[test]
fn serialization_consumes_tables_once_and_matches_original_bytes_concurrently() {
    let (tables, size) = tables();
    let expected = sfnt::pack(tables.clone(), size).unwrap();
    let face = Arc::new(FontFace {
        data: FontData::tables(tables, size),
        index: 0,
        variations: vec![],
    });
    let reserved = face.storage_bytes();
    let FontData::Tables { tables, packed, .. } = &face.data else {
        unreachable!()
    };
    assert!(tables.lock().unwrap().is_some());
    assert!(packed.get().is_none());
    let pointers = std::thread::scope(|scope| {
        let threads: Vec<_> = (0..8)
            .map(|_| {
                scope.spawn(|| {
                    let data = face.data().unwrap();
                    assert_eq!(data, expected);
                    data.as_ptr() as usize
                })
            })
            .collect();
        threads
            .into_iter()
            .map(|t| t.join().unwrap())
            .collect::<Vec<_>>()
    });
    assert!(pointers.iter().all(|p| *p == pointers[0]));
    assert!(tables.lock().unwrap().is_none());
    assert!(packed.get().is_some());
    assert_eq!(face.storage_bytes(), reserved);
    assert!(reserved >= expected.len());
    assert!(reserved < expected.len() * 2);
}
#[test]
fn serialization_failure_is_stable_and_releases_input_tables() {
    let face = FontFace {
        data: FontData::tables(vec![(0, vec![0; 1024])], 2048),
        index: 0,
        variations: vec![],
    };
    let a = face.data().unwrap_err();
    let b = face.data().unwrap_err();
    assert_eq!(a, b);
    let FontData::Tables { tables, packed, .. } = &face.data else {
        unreachable!()
    };
    assert!(tables.lock().unwrap().is_none());
    assert!(packed.get().unwrap().is_err());
}
pub(super) fn text(source: &str, face: Arc<FontFace>) -> ShapedText {
    ShapedText {
        source: source.into(),
        width: 10.,
        runs: vec![Run {
            source: 0..source.len(),
            font_name: "Fixture".into(),
            font_source: FontSource::Embedded(face),
            font_size: 17.,
            right_to_left: false,
            glyphs: vec![Glyph {
                id: 1,
                source: 0..source.len(),
                position: [0., 0.],
                advance: [10., 0.],
                ink: Arc::new(Ink::Outline("M0 0 L10 0 L10 -10 Z".into())),
            }],
        }],
    }
}
#[test]
fn shared_faces_are_counted_once_and_distinct_faces_remain_charged() {
    let face = Arc::new(FontFace {
        data: FontData::Bytes(Arc::from(vec![0; 1024])),
        index: 0,
        variations: vec![],
    });
    let mut a = text("a", face.clone());
    let b = text("b", face);
    a.runs.push(a.runs[0].clone());
    let mut seen = Default::default();
    let first = a.font_storage_bytes(&mut seen);
    assert!(first >= 1024);
    assert_eq!(b.font_storage_bytes(&mut seen), 0);
    assert_eq!(a.storage_bytes(), a.local_storage_bytes() + first);
    let distinct = text(
        "c",
        Arc::new(FontFace {
            data: FontData::Bytes(Arc::from(vec![0; 1024])),
            index: 0,
            variations: vec![],
        }),
    );
    assert_eq!(distinct.font_storage_bytes(&mut seen), first);
}
