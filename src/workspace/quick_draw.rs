// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;

impl Editor {
    pub fn can_quick_draw(&self) -> bool {
        self.require_structure_edit().is_ok()
    }
    /// Commit one detached gesture, converting world coordinates to group space.
    pub fn insert_quick_draw(&mut self, mut node: Node) -> Result<Id, String> {
        self.require_structure_edit()?;
        let NodeContent::Control { kind, size } = &node.content else {
            return Err("Draw a control".into());
        };
        if !["arrow", "image", "rectangle", "text_block", "text_line"].contains(&kind.as_str()) {
            return Err("Unsupported drawing tool".into());
        }
        if ![node.origin.x,node.origin.y].into_iter().all(|v| v.is_finite() && v.abs() <= 100_000.)
            || ![size.width,size.height].into_iter().all(|v| matches!(v,Dimension::Fixed(x) if x.is_finite() && (1. ..=100_000.).contains(&x))) {
            return Err("Drawing exceeds the supported canvas".into());
        }
        catalog::get(kind)
            .unwrap()
            .effective(&node, &self.property_context())
            .map_err(|e| e.to_string())?;
        let mut result = Err("Unable to allocate a control".into());
        self.edit("Draw control", |e| {
            let Ok(id) = e.document.allocate_id() else {
                return;
            };
            let origin = e.context_origin();
            node.id = id;
            node.origin.x -= origin.x;
            node.origin.y -= origin.y;
            e.nodes_mut().push(node);
            e.selection = HashSet::from([id]);
            e.append_order(id);
            result = Ok(id);
        });
        result
    }
    /// Layout with the live UI fonts; never construct an Editor or touch history
    /// on each pointer sample just to display a temporary control.
    pub fn quick_draw_scene(
        &self,
        node: Node,
        fonts: &crate::layout::EguiMeasurer<'_>,
    ) -> Result<crate::render::Scene, String> {
        let engine = crate::layout::Engine {
            project: &self.document,
            measurer: fonts,
            context: self.property_context(),
        };
        let layout = engine.layout_named(
            &Content {
                nodes: vec![node],
                paint_order: vec![],
            },
            "",
        )?;
        let layers: Vec<_> = layout
            .leaves()
            .into_iter()
            .map(|(n, _)| Layer {
                control: view::control(n),
                frame: Bounds::new(n.bounds.x, n.bounds.y, n.bounds.width, n.bounds.height),
                clip: None,
                opacity: 0.5,
                local_markup: n.local_markup,
                group_links: vec![],
                path: n.path.clone(),
                properties: n.properties.clone(),
                clock: None,
                date: None,
            })
            .collect();
        crate::render::Scene::build(
            "",
            &layers,
            self.document.settings.sketch,
            fonts,
            &self.document.assets,
        )
    }
}
