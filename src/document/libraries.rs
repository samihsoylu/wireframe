// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;

/// A source placement refers to canonical content, not a local instance override.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ComponentSource {
    pub definition: Id,
    pub node: Id,
    pub notes: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ComponentLibrary {
    pub id: Id,
    /// Retained resources occupy identities but are unavailable to normal editing.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub trashed: bool,
    pub name: String,
    pub notes: String,
    pub content: Content,
    pub sources: Vec<ComponentSource>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub imports: Vec<LibraryImport>,
}
/// An explicit foreign identity and its source-definition correspondence.
/// Bindings may outlive deleted sources so a later import can restore references.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LibraryImport {
    pub project: String,
    pub library: Id,
    pub definitions: BTreeMap<Id, Id>,
}
impl Project {
    pub fn live_libraries(&self) -> impl Iterator<Item = &ComponentLibrary> {
        self.libraries.iter().filter(|library| !library.trashed)
    }
    pub fn live_definitions(&self) -> impl Iterator<Item = &Definition> {
        let retained: std::collections::HashSet<_> = self
            .libraries
            .iter()
            .filter(|l| l.trashed)
            .flat_map(|l| l.sources.iter().map(|s| s.definition))
            .collect();
        self.definitions
            .iter()
            .filter(move |d| !retained.contains(&d.id))
    }
    pub fn definition_available(&self, id: Id) -> bool {
        self.definitions
            .iter()
            .any(|definition| definition.id == id)
            && self
                .component_source(id)
                .is_none_or(|(library, _)| !library.trashed)
    }
    pub fn live_component_source(&self, id: Id) -> Option<(&ComponentLibrary, &ComponentSource)> {
        self.component_source(id)
            .filter(|(library, _)| !library.trashed)
    }

    /// Shared root appearance lives on the unique owning source placement.
    pub fn source_appearance(&self, definition: Id) -> (Option<Crop>, bool) {
        self.component_source(definition)
            .and_then(|(library, source)| {
                library.content.nodes.iter().find(|n| n.id == source.node)
            })
            .map_or((None, false), |node| (node.crop, node.markup))
    }
    pub fn component_source(
        &self,
        definition: Id,
    ) -> Option<(&ComponentLibrary, &ComponentSource)> {
        self.libraries.iter().find_map(|library| {
            library
                .sources
                .iter()
                .find(|s| s.definition == definition)
                .map(|source| (library, source))
        })
    }
    /// Assign newly assembled definitions to the default library. Existing source
    /// identities, notes and placements remain unchanged. The change is atomic.
    pub fn organize_components(&mut self) -> Result<(), String> {
        self.validate_libraries(false)?;
        let missing: Vec<_> = self
            .definitions
            .iter()
            .filter(|d| self.component_source(d.id).is_none())
            .map(|d| d.id)
            .collect();
        if missing.is_empty() {
            return Ok(());
        }
        let mut next = self.clone();
        let library_id = if let Some(id) = next
            .default_library
            .filter(|id| next.live_libraries().any(|l| l.id == *id))
        {
            if !next.libraries.iter().any(|l| l.id == id) {
                return Err("Default component library is missing".into());
            }
            id
        } else {
            let id = next.allocate_id()?;
            next.libraries.push(ComponentLibrary {
                id,
                trashed: false,
                name: "Project components".into(),
                notes: String::new(),
                content: Content::default(),
                sources: vec![],
                imports: vec![],
            });
            next.default_library = Some(id);
            id
        };
        let count = next
            .libraries
            .iter()
            .find(|l| l.id == library_id)
            .unwrap()
            .sources
            .len()
            + missing.len();
        // Legacy projects can contain thousands of sources. Keep the synthetic
        // placement grid within the document's coordinate bounds at any count.
        let columns = count.div_ceil(400).max(4);
        let step_x = 300_f32.min(90_000. / (columns - 1) as f32);
        for definition in missing {
            let node = next.allocate_id()?;
            let library = next
                .libraries
                .iter_mut()
                .find(|l| l.id == library_id)
                .unwrap();
            let index = library.sources.len();
            let mut placement = Node::group(
                node,
                Point::new(
                    50. + (index % columns) as f32 * step_x,
                    50. + (index / columns) as f32 * 240.,
                ),
                vec![],
            );
            placement.content = NodeContent::Instance {
                definition,
                overrides: vec![],
            };
            library.content.nodes.push(placement);
            if !library.content.paint_order.is_empty() {
                library.content.paint_order.push(vec![node]);
            }
            library.sources.push(ComponentSource {
                definition,
                node,
                notes: String::new(),
            });
        }
        next.validate()?;
        *self = next;
        Ok(())
    }
}
