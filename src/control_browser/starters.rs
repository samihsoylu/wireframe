// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Original editable starter projects built from the product's own controls.
use super::*;
use crate::{
    catalog,
    document::{
        ComponentLibrary, ComponentSource, Content, Definition, Link, Node, NodeContent, Point,
        Size, Value,
    },
};

pub(super) fn entries() -> Vec<Entry> {
    [
        (
            "landing",
            "Landing page",
            "Web",
            "A landing page with two linked destinations and a contact form.",
        ),
        (
            "contact",
            "Contact forms",
            "Forms",
            "Desktop and phone contact pages with labeled fields.",
        ),
        (
            "order",
            "Tablet order",
            "Mobile",
            "An order summary with contact details and confirmation.",
        ),
        (
            "components",
            "Form components",
            "Components",
            "Reusable action buttons and labeled text fields.",
        ),
    ]
    .into_iter()
    .enumerate()
    .map(|(i, (id, title, category, description))| Entry {
        id: format!("wireframe-starter:{id}:1"),
        title: title.into(),
        category: category.into(),
        description: description.into(),
        source: Source::Starter(i),
    })
    .collect()
}
fn control(
    p: &mut Project,
    page: usize,
    kind: &str,
    rect: [f32; 4],
    text: &str,
) -> Result<Id, String> {
    let id = p.allocate_id()?;
    let mut n = catalog::get(kind)
        .ok_or_else(|| format!("Unknown starter control: {kind}"))?
        .create(id, Point::new(rect[0], rect[1]));
    if let NodeContent::Control { size, .. } = &mut n.content {
        *size = Size::fixed(rect[2], rect[3]);
    }
    n.properties.insert("text".into(), Value::Text(text.into()));
    p.pages[page].content.nodes.push(n);
    Ok(id)
}
fn page(p: &mut Project, name: &str) -> Result<usize, String> {
    let mut page = p.pages[0].clone();
    page.id = p.allocate_id()?;
    page.name = name.into();
    page.content = Content::default();
    page.notes = String::new();
    page.alternates.clear();
    p.pages.push(page);
    Ok(p.pages.len() - 1)
}
fn contact(p: &mut Project, index: usize, mobile: bool) -> Result<(), String> {
    let width = if mobile { 320. } else { 640. };
    control(p, index, "heading", [24., 24., width, 42.], "Get in touch")?;
    control(
        p,
        index,
        "paragraph",
        [24., 76., width, 54.],
        "Tell us what you have in mind. We will reply by email.",
    )?;
    for (y, label, value) in [
        (155., "Your name", "Name"),
        (245., "Email address", "you@example.com"),
    ] {
        control(p, index, "label", [24., y, width, 24.], label)?;
        control(
            p,
            index,
            "text_input",
            [24., y + 30., width.min(430.), 36.],
            value,
        )?;
    }
    control(p, index, "label", [24., 335., width, 24.], "Message")?;
    control(
        p,
        index,
        "text_area",
        [24., 365., width.min(430.), 120.],
        "How can we help?",
    )?;
    control(p, index, "button", [24., 510., 150., 36.], "Send message")?;
    Ok(())
}
pub(super) fn project(index: usize) -> Result<Project, String> {
    let info = entries()
        .get(index)
        .cloned()
        .ok_or("Starter project is unavailable")?;
    let mut p = Project {
        title: info.title,
        ..Project::default()
    };
    match index {
        0 => {
            p.pages[0].name = "Landing".into();
            control(
                &mut p,
                0,
                "heading",
                [40., 40., 650., 60.],
                "Make room for your next idea",
            )?;
            control(
                &mut p,
                0,
                "paragraph",
                [40., 120., 600., 72.],
                "A simple workspace for exploring ideas together. Start with a sketch and make it yours.",
            )?;
            let a = control(
                &mut p,
                0,
                "button",
                [40., 225., 170., 40.],
                "Explore features",
            )?;
            let b = control(&mut p, 0, "button", [235., 225., 170., 40.], "Contact us")?;
            let features = page(&mut p, "Features")?;
            let contact_page = page(&mut p, "Contact")?;
            let targets = [p.pages[features].id, p.pages[contact_page].id];
            for (id, target) in [a, b].into_iter().zip(targets) {
                p.pages[0]
                    .content
                    .nodes
                    .iter_mut()
                    .find(|n| n.id == id)
                    .unwrap()
                    .properties
                    .insert("link".into(), Value::Link(Link::Page(target)));
            }
            control(
                &mut p,
                features,
                "heading",
                [40., 40., 650., 48.],
                "From rough ideas to a shared plan",
            )?;
            for (y, title) in [
                (120., "Sketch the important parts"),
                (220., "Compare different approaches"),
                (320., "Share a clear next step"),
            ] {
                control(&mut p, features, "heading", [40., y, 650., 36.], title)?;
            }
            contact(&mut p, contact_page, false)?;
        }
        1 => {
            p.pages[0].name = "Desktop contact".into();
            contact(&mut p, 0, false)?;
            let i = page(&mut p, "Phone contact")?;
            contact(&mut p, i, true)?;
        }
        2 => {
            p.pages[0].name = "Tablet order".into();
            control(&mut p, 0, "heading", [32., 28., 660., 48.], "Your order")?;
            control(
                &mut p,
                0,
                "paragraph",
                [32., 100., 620., 80.],
                "Notebook                     2 × €12\nPencil set                    1 × €8\nTotal                              €32",
            )?;
            for (y, label) in [
                (220., "Full name"),
                (310., "Email"),
                (400., "Delivery address"),
            ] {
                control(&mut p, 0, "label", [32., y, 600., 25.], label)?;
                control(&mut p, 0, "text_input", [32., y + 32., 600., 38.], "")?;
            }
            control(
                &mut p,
                0,
                "checkbox",
                [32., 505., 600., 32.],
                "Send me an order confirmation",
            )?;
            control(&mut p, 0, "button", [32., 575., 200., 42.], "Place order")?;
        }
        3 => {
            p.pages[0].name = "Examples".into();
            let library_id = p.allocate_id()?;
            let mut library = ComponentLibrary {
                id: library_id,
                name: "Form components".into(),
                notes: "Original reusable form controls.".into(),
                trashed: false,
                content: Content::default(),
                sources: vec![],
                imports: vec![],
            };
            for (i, (kind, name, text, w)) in [
                ("button", "Primary action", "Continue", 140.),
                ("text_input", "Text field", "Enter a value", 250.),
            ]
            .into_iter()
            .enumerate()
            {
                let def = p.allocate_id()?;
                let child = p.allocate_id()?;
                let root = p.allocate_id()?;
                let mut node = catalog::get(kind).unwrap().create(child, Point::default());
                if let NodeContent::Control { size, .. } = &mut node.content {
                    *size = Size::fixed(w, 36.);
                }
                node.properties
                    .insert("text".into(), Value::Text(text.into()));
                p.definitions.push(Definition {
                    id: def,
                    name: name.into(),
                    content: Content {
                        nodes: vec![node],
                        paint_order: vec![],
                    },
                });
                let mut instance = Node::group(root, Point::new(24., 24. + i as f32 * 80.), vec![]);
                instance.content = NodeContent::Instance {
                    definition: def,
                    overrides: vec![],
                };
                library.content.nodes.push(instance);
                library.sources.push(ComponentSource {
                    definition: def,
                    node: root,
                    notes: String::new(),
                });
            }
            p.default_library = Some(library_id);
            p.libraries.push(library);
        }
        _ => unreachable!(),
    }
    p.validate()?;
    Ok(p)
}
