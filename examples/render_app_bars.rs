// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Generate an editable navigation gallery with distinct icon destinations.
use std::{fs, path::PathBuf};
use wireframe::{
    document::{IconSource, Link, Value},
    model::Kind,
    workspace::Editor,
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .unwrap_or_else(|| "navigation-gallery".into()),
    );
    fs::create_dir_all(&out)?;
    let mut e = Editor::default();
    e.document.title = "Project navigation".into();
    e.document.pages[0].name = "Navigation".into();
    e.document.settings.sketch = false;
    let page = e.page;
    let mut destinations = vec![];
    for name in ["Workspace", "Favorites", "More actions"] {
        e.add_page();
        destinations.push(e.page);
        e.document.pages.last_mut().unwrap().name = name.into();
        e.add(Kind::MenuBar, 50., 50.);
        let id = *e.selection.iter().next().unwrap();
        e.set_control_property(id, "text", Value::Text(format!("Return, {name}")))?;
        e.set_control_property(
            id,
            "row_links",
            Value::Links(vec![Some(Link::Page(page)), None]),
        )?;
    }
    e.switch_page(page);
    for (i, source) in [
        "Projects",
        "Favorites",
        "Workspace",
        "Title",
        "A longer workspace heading",
        "Title\\rcontinued",
        "Quiet workspace",
        "Small preview",
        "Duplicates",
    ]
    .iter()
    .enumerate()
    {
        e.add(
            Kind::AppBar,
            50. + (i % 3) as f32 * 430.,
            50. + (i / 3) as f32 * 180.,
        );
        let id = *e.selection.iter().next().unwrap();
        e.set_control_property(id, "text", Value::Text((*source).into()))?;
        let names: &[&str] = if i == 8 {
            &["star-regular", "star-regular", "magnifying-glass-solid"]
        } else if matches!(i, 1..=3) {
            &["star-regular", "heart-solid"]
        } else {
            &[]
        };
        e.set_control_property(
            id,
            "icons",
            Value::Icons(
                names
                    .iter()
                    .map(|s| Some(IconSource::Catalog((*s).into())))
                    .collect(),
            ),
        )?;
        if matches!(i, 1 | 3) {
            e.set_control_property(id, "nav_icon", Value::Bool(false))?;
        }
        if i == 2 {
            e.set_control_property(id, "filter_icon", Value::Bool(false))?;
            e.set_control_property(id, "fill", Value::Color([36, 55, 76, 255]))?;
        }
        if i == 6 {
            for key in ["nav_icon", "filter_icon", "menu_icon"] {
                e.set_control_property(id, key, Value::Bool(false))?;
            }
        }
        let mut links = vec![
            Some(Link::Page(destinations[0])),
            Some(Link::Page(destinations[1])),
        ];
        links.extend(names.iter().map(|_| Some(Link::Page(destinations[1]))));
        links.push(Some(Link::Page(destinations[2])));
        e.set_control_property(id, "row_links", Value::Links(links))?;
        if matches!(i, 3 | 7) {
            e.begin("Compact preview");
            e.selected_mut(|c| {
                c.bounds.w = if i == 3 { 140. } else { 80. };
                c.bounds.h = 44.;
            });
            e.commit();
        }
    }
    fs::write(
        out.join("Project-navigation.wireframe"),
        e.document.encode()?,
    )?;
    for (name, sketch) in [("clean", false), ("sketch", true)] {
        e.document.settings.sketch = sketch;
        let scene = e.rendered_scene(page)?;
        fs::write(out.join(format!("{name}.png")), scene.png()?)?;
        fs::write(out.join(format!("{name}.svg")), scene.svg())?;
    }
    let image = image::RgbaImage::from_fn(32, 32, |x, y| {
        if (4..=27).contains(&x) && (4..=27).contains(&y) && (x < 11 || y > 20) {
            image::Rgba([0, 0, 0, 255])
        } else {
            image::Rgba([0, 0, 0, 0])
        }
    });
    image.save(out.join("Corner.png"))?;
    Ok(())
}
