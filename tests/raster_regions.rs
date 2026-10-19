// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use base64::Engine as _;
use resvg::{tiny_skia as sk, usvg};

fn image_uri() -> String {
    let mut image = sk::Pixmap::new(17, 13).unwrap();
    for y in 0..13 {
        for x in 0..17 {
            image.pixels_mut()[(y * 17 + x) as usize] = sk::ColorU8::from_rgba(
                (x * 13) as u8,
                (y * 17) as u8,
                ((x + y) * 7) as u8,
                ((x * 17 + y * 11) % 256) as u8,
            )
            .premultiply();
        }
    }
    format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(image.encode_png().unwrap())
    )
}

fn text_and_images(group: &usvg::Group) -> (usize, usize) {
    group.children().iter().fold((0, 0), |mut n, node| {
        match node {
            usvg::Node::Group(g) => {
                let child = text_and_images(g);
                n.0 += child.0;
                n.1 += child.1;
            }
            usvg::Node::Text(_) => n.0 += 1,
            usvg::Node::Image(_) => n.1 += 1,
            _ => {}
        }
        n
    })
}

#[test]
fn nested_clips_images_and_text_match_full_frame() {
    let mut options = usvg::Options::default();
    options.fontdb_mut().load_font_data(
        include_bytes!("../assets/fonts/comic-neue/ComicNeue-Regular.ttf").to_vec(),
    );
    for density in [1, 2] {
        for white in [false, true] {
            let background = if white {
                "<rect width='1100' height='500' fill='white'/>"
            } else {
                ""
            };
            let svg = format!(
                "<svg xmlns='http://www.w3.org/2000/svg' width='{}' height='{}' viewBox='0 0 1100 500'>
                <defs>
                  <clipPath id='a'><rect x='50.25' y='20.5' width='1000.5' height='440.25' rx='73.5'/></clipPath>
                  <ellipse id='e' cx='550.5' cy='250.25' rx='500.25' ry='210.5'/>
                  <clipPath id='b' clip-path='url(#a)'><use href='#e' clip-path='url(#c)'/></clipPath>
                  <clipPath id='c'><path d='M30 10H1080V480H30Z M500 250H600V340H500Z' clip-rule='evenodd'/></clipPath>
                  <linearGradient id='g'><stop stop-color='#fa7040'/><stop offset='1' stop-color='#235ed8' stop-opacity='.5'/></linearGradient>
                </defs>{background}
                <g opacity='.7'><g clip-path='url(#b)'><g opacity='.65' transform='rotate(3 550 250)'>
                  <rect x='20.25' y='10.5' width='1060' height='480' rx='100' fill='url(#g)' stroke='#173557' stroke-width='3.25'/>
                  <image x='120.25' y='100.75' width='900.5' height='320.25' preserveAspectRatio='none' href='{}'/>
                  <path d='M20 250C400 10 700 500 1080 200' fill='none' stroke='#21482a' stroke-width='.65' stroke-linecap='round' stroke-dasharray='11.5 7.25'/>
                  <text x='160' y='190' font-family='Comic Neue' font-size='31'>Regional image — café &amp; layout</text>
                </g></g></g></svg>",
                1100 * density,
                500 * density,
                image_uri()
            );
            let tree = usvg::Tree::from_str(&svg, &options).unwrap();
            assert_eq!(text_and_images(tree.root()), (1, 1));
            let size = tree.size().to_int_size();
            let mut full = sk::Pixmap::new(size.width(), size.height()).unwrap();
            resvg::render(&tree, sk::Transform::identity(), &mut full.as_mut());
            assert!(full.pixels().iter().any(|p| p.alpha() > 0 && p.red() < 200));
            let mut max_delta = 0;
            for y in (0..size.height()).step_by(128) {
                for x in (0..size.width()).step_by(512) {
                    let width = 512.min(size.width() - x);
                    let height = 128.min(size.height() - y);
                    let tile = resvg::render_region(
                        &tree,
                        sk::Transform::identity(),
                        sk::IntRect::from_xywh(x as i32, y as i32, width, height).unwrap(),
                        || Ok(()),
                    )
                    .unwrap();
                    for row in 0..height {
                        for col in 0..width {
                            let a = full.pixel(x + col, y + row).unwrap().demultiply();
                            let b = tile.pixel(col, row).unwrap().demultiply();
                            for d in [
                                a.red().abs_diff(b.red()),
                                a.green().abs_diff(b.green()),
                                a.blue().abs_diff(b.blue()),
                                a.alpha().abs_diff(b.alpha()),
                            ] {
                                max_delta = max_delta.max(d);
                            }
                        }
                    }
                }
            }
            assert!(max_delta <= 1, "{density}x white={white}: {max_delta}");
        }
    }
}

#[test]
fn large_logical_canvas_keeps_a_small_region_and_observes_cancellation() {
    let svg = "<svg xmlns='http://www.w3.org/2000/svg' width='20000' height='20000'><rect width='20000' height='20000' fill='white'/><defs><clipPath id='c'><rect x='19890' y='19890' width='100' height='100' rx='12'/></clipPath></defs><g opacity='.8'><g clip-path='url(#c)'><g opacity='.5'><rect x='19900' y='19900' width='80' height='80' rx='20' fill='#1e64dc'/></g></g></g></svg>";
    let tree = usvg::Tree::from_str(svg, &usvg::Options::default()).unwrap();
    let region = sk::IntRect::from_xywh(19872, 19872, 128, 128).unwrap();
    let mut checkpoints = 0;
    let tile = resvg::render_region(&tree, sk::Transform::identity(), region, || {
        checkpoints += 1;
        Ok(())
    })
    .unwrap();
    assert_eq!(tile.data().len(), 128 * 128 * 4);
    assert_eq!(tile.pixel(0, 0).unwrap().alpha(), 255);
    let centre = tile.pixel(68, 68).unwrap().demultiply();
    assert_eq!(
        [centre.red(), centre.green(), centre.blue()],
        [165, 193, 241]
    );
    for stop in [1, 5, checkpoints] {
        let mut calls = 0;
        let error = resvg::render_region(&tree, sk::Transform::identity(), region, || {
            calls += 1;
            if calls == stop {
                Err("Cancelled".into())
            } else {
                Ok(())
            }
        })
        .unwrap_err();
        assert_eq!(error, "Cancelled");
        assert_eq!(calls, stop);
    }
}
