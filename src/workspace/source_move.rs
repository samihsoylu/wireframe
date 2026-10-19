// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;

impl Editor {
    /// Transfer owning placements without cloning definitions or rewriting uses.
    /// Definition identities are project-wide; placement identities are scoped
    /// to their library and receive a fresh identity at the destination.
    pub fn move_sources_to_library(&mut self, sources: &[Id], target: Id) -> Result<(), String> {
        let target_index = self
            .document
            .libraries
            .iter()
            .position(|l| l.id == target && !l.trashed)
            .ok_or("Destination library no longer exists")?;
        let mut seen = HashSet::new();
        let mut moves = Vec::new();
        for &definition in sources {
            if !seen.insert(definition) {
                return Err("A source occurs more than once in the selection".into());
            }
            let (library, source) = self
                .document
                .live_component_source(definition)
                .ok_or("Component source no longer exists")?;
            if library.id == target {
                continue;
            }
            let placement = library
                .content
                .nodes
                .iter()
                .find(|n| n.id == source.node)
                .ok_or("Source placement no longer exists")?;
            moves.push((library.id, source.clone(), placement.clone()));
        }
        if moves.is_empty() {
            return Ok(());
        }
        self.component_change("Move components to library", |editor| {
            let first = moves[0].1.definition;
            for (owner, mut source, mut placement) in moves {
                let library = editor
                    .document
                    .libraries
                    .iter_mut()
                    .find(|l| l.id == owner)
                    .ok_or("Source library no longer exists")?;
                library
                    .sources
                    .retain(|s| s.definition != source.definition);
                library.content.nodes.retain(|n| n.id != source.node);
                library
                    .content
                    .paint_order
                    .retain(|path| path.first() != Some(&source.node));
                placement.id = editor.document.allocate_id()?;
                source.node = placement.id;
                let destination = &mut editor.document.libraries[target_index];
                if !destination.content.paint_order.is_empty() {
                    destination.content.paint_order.push(vec![placement.id]);
                }
                destination.content.nodes.push(placement);
                destination.sources.push(source);
            }
            editor.library = Some(target);
            editor.definition = Some(first);
            editor.context.clear();
            editor.selection.clear();
            Ok(())
        })
    }
}
