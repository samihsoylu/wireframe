// SPDX-License-Identifier: GPL-3.0-only
//! These tests invoke the real statically linked engine and embedded model.
use std::{io::Cursor, sync::atomic::AtomicBool};
use wireframe_ocr::recognize;

const SCREENSHOT: &[u8] = include_bytes!("fixtures/screenshot.png");
const EXPECTED: [&str; 6] = [
    "Project overview",
    "Welcome to Wireframe",
    "Your ideas, clearly arranged.",
    "Draft screens: 12",
    "Ready for review",
    "Create project",
];

fn assert_screenshot_text(bytes: &[u8]) {
    let result = recognize(bytes, &AtomicBool::new(false)).expect("real inference succeeds");
    println!("Recognized ({}%):\n{}", result.confidence, result.text);
    let mut end = 0;
    for expected in EXPECTED {
        let position = result.text[end..]
            .find(expected)
            .unwrap_or_else(|| panic!("missing or out-of-order {expected:?}: {:?}", result.text));
        end += position + expected.len();
    }
    assert!(result.confidence >= 70, "fixture should be readable");
}

#[test]
fn recognizes_a_real_ui_screenshot_in_reading_order() {
    assert_screenshot_text(SCREENSHOT);
}

#[test]
fn recognizes_jpeg_and_first_gif_frame() {
    let screenshot = image::load_from_memory(SCREENSHOT).unwrap();
    for format in [image::ImageFormat::Jpeg, image::ImageFormat::Gif] {
        let mut encoded = Cursor::new(Vec::new());
        screenshot.write_to(&mut encoded, format).unwrap();
        assert_screenshot_text(&encoded.into_inner());
    }
}

#[test]
fn blank_and_fully_transparent_images_do_not_invent_text() {
    let screenshot = image::load_from_memory(SCREENSHOT).unwrap().to_rgba8();
    let transparent = image::RgbaImage::from_fn(screenshot.width(), screenshot.height(), |x, y| {
        let pixel = screenshot.get_pixel(x, y);
        image::Rgba([pixel[0], pixel[1], pixel[2], 0])
    });
    for image in [
        image::DynamicImage::new_rgb8(800, 500),
        image::DynamicImage::ImageRgba8(transparent),
    ] {
        let mut encoded = Cursor::new(Vec::new());
        image
            .write_to(&mut encoded, image::ImageFormat::Png)
            .unwrap();
        let result = recognize(&encoded.into_inner(), &AtomicBool::new(false)).unwrap();
        assert!(
            result.text.is_empty(),
            "blank image produced {:?}",
            result.text
        );
    }
}

#[test]
fn concurrent_recognitions_have_separate_engine_state() {
    std::thread::scope(|scope| {
        let first = scope.spawn(|| assert_screenshot_text(SCREENSHOT));
        let second = scope.spawn(|| assert_screenshot_text(SCREENSHOT));
        first.join().unwrap();
        second.join().unwrap();
    });
}

#[test]
fn cancellation_during_work_returns_no_partial_text() {
    let cancel = AtomicBool::new(false);
    std::thread::scope(|scope| {
        let worker = scope.spawn(|| recognize(SCREENSHOT, &cancel));
        std::thread::sleep(std::time::Duration::from_millis(1));
        cancel.store(true, std::sync::atomic::Ordering::Relaxed);
        assert_eq!(worker.join().unwrap(), Err(wireframe_ocr::Error::Cancelled));
    });
}
