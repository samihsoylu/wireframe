// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComponentUsage {
    pub surface: Id,
    pub path: NodePath,
}
impl Editor {
    pub fn in_source_workspace(&self) -> bool {
        self.library.is_some() || self.definition.is_some()
    }
    /// An owning placement is editable canonical content, not a local instance.
    pub fn owning_source(&self, node: Id) -> Option<Id> {
        if self.definition.is_some() || !self.context.is_empty() {
            return None;
        }
        let library = self.library?;
        self.document
            .libraries
            .iter()
            .find(|l| l.id == library && !l.trashed)?
            .sources
            .iter()
            .find(|s| s.node == node)
            .map(|s| s.definition)
    }
    pub fn edit_library(&mut self, id: Id) -> Result<(), String> {
        if !self.document.live_libraries().any(|l| l.id == id) {
            return Err("Component library no longer exists".into());
        }
        self.commit();
        self.library = Some(id);
        self.definition = None;
        self.context.clear();
        self.selection.clear();
        Ok(())
    }
    /// Enter through the source navigator, retaining the owning library for Back.
    pub fn edit_library_source(&mut self, id: Id) -> Result<(), String> {
        let library = self
            .document
            .live_component_source(id)
            .map(|(library, _)| library.id)
            .ok_or("Component source no longer exists")?;
        self.edit_library(library)?;
        self.edit_definition(id)
    }
    fn library_name(name: &str) -> Result<String, String> {
        let name = name.trim();
        if name.is_empty() || name.len() > 10_000 || name.chars().any(char::is_control) {
            return Err("Enter a name of 1–10000 bytes without control characters".into());
        }
        Ok(name.into())
    }
    pub fn create_library(&mut self, name: &str) -> Result<Id, String> {
        let name = Self::library_name(name)?;
        self.component_change("New component library", |e| {
            let id = e.document.allocate_id()?;
            e.document.libraries.push(ComponentLibrary {
                id,
                trashed: false,
                name,
                notes: String::new(),
                content: Content::default(),
                sources: vec![],
                imports: vec![],
            });
            e.library = Some(id);
            e.definition = None;
            e.context.clear();
            e.selection.clear();
            Ok(id)
        })
    }
    pub fn rename_library(&mut self, id: Id, name: &str) -> Result<(), String> {
        let name = Self::library_name(name)?;
        self.component_change("Rename component library", |e| {
            e.document
                .libraries
                .iter_mut()
                .find(|l| l.id == id && !l.trashed)
                .ok_or("Component library no longer exists")?
                .name = name;
            Ok(())
        })
    }
    pub fn create_empty_source(
        &mut self,
        library: Id,
        name: &str,
        origin: Point,
    ) -> Result<Id, String> {
        let name = Self::library_name(name)?;
        self.component_change("New component source", |e| {
            let definition = e.document.allocate_id()?;
            let node = e.document.allocate_id()?;
            let target = e
                .document
                .libraries
                .iter_mut()
                .find(|l| l.id == library && !l.trashed)
                .ok_or("Component library no longer exists")?;
            let mut placement = Node::group(node, origin, vec![]);
            placement.content = NodeContent::Instance {
                definition,
                overrides: vec![],
            };
            target.content.nodes.push(placement);
            if !target.content.paint_order.is_empty() {
                target.content.paint_order.push(vec![node]);
            }
            target.sources.push(ComponentSource {
                definition,
                node,
                notes: String::new(),
            });
            e.document.definitions.push(Definition {
                id: definition,
                name,
                content: Content::default(),
            });
            e.library = Some(library);
            e.definition = Some(definition);
            e.context.clear();
            e.selection.clear();
            Ok(definition)
        })
    }
    /// Commit notes to their original resource even if navigation has departed.
    pub fn edit_source_notes(&mut self, surface: Id, notes: &str) -> Result<(), String> {
        if notes.len() > 1_000_000 {
            return Err("Notes exceed the one million byte limit".into());
        }
        self.component_change("Edit notes", |e| {
            if let Some(library) = e
                .document
                .libraries
                .iter_mut()
                .find(|l| l.id == surface && !l.trashed)
            {
                library.notes = notes.into();
            } else {
                let source = e
                    .document
                    .libraries
                    .iter_mut()
                    .filter(|l| !l.trashed)
                    .flat_map(|l| &mut l.sources)
                    .find(|s| s.definition == surface)
                    .ok_or("Component source no longer exists")?;
                source.notes = notes.into();
            }
            Ok(())
        })
    }
    /// Direct references on navigable surfaces. Nested source references belong
    /// to the containing definition; ownership placements themselves are not uses.
    pub fn component_usages(&self, definition: Id) -> Vec<ComponentUsage> {
        fn visit(
            nodes: &[Node],
            definition: Id,
            surface: Id,
            parent: &mut NodePath,
            out: &mut Vec<ComponentUsage>,
        ) {
            for node in nodes {
                parent.push(node.id);
                match &node.content {
                    NodeContent::Group { children } => {
                        visit(children, definition, surface, parent, out)
                    }
                    NodeContent::Instance {
                        definition: target, ..
                    } if *target == definition => {
                        out.push(ComponentUsage {
                            surface,
                            path: parent.clone(),
                        });
                    }
                    _ => {}
                }
                parent.pop();
            }
        }
        let mut uses = vec![];
        for page in self.document.live_pages() {
            visit(
                &page.content.nodes,
                definition,
                page.id,
                &mut vec![],
                &mut uses,
            );
            for alternate in &page.alternates {
                visit(
                    &alternate.content.nodes,
                    definition,
                    alternate.id,
                    &mut vec![],
                    &mut uses,
                );
            }
        }
        for source in self.document.live_definitions() {
            visit(
                &source.content.nodes,
                definition,
                source.id,
                &mut vec![],
                &mut uses,
            );
        }
        for library in self.document.live_libraries() {
            for node in &library.content.nodes {
                if !library.sources.iter().any(|s| s.node == node.id) {
                    visit(
                        std::slice::from_ref(node),
                        definition,
                        library.id,
                        &mut vec![],
                        &mut uses,
                    );
                }
            }
        }
        uses
    }
    pub fn visit_component_usage(
        &mut self,
        definition: Id,
        usage: &ComponentUsage,
    ) -> Result<(), String> {
        if !self.component_usages(definition).contains(usage) {
            return Err("Component usage no longer exists".into());
        }
        let page = self.document.live_pages().find_map(|page| {
            if page.id == usage.surface {
                Some((page.id, None))
            } else {
                page.alternates
                    .iter()
                    .find(|a| a.id == usage.surface)
                    .map(|alternate| (page.id, Some(alternate.id)))
            }
        });
        if let Some((page, alternate)) = page {
            self.switch_version(page, alternate)
        } else if self
            .document
            .definitions
            .iter()
            .any(|d| d.id == usage.surface)
        {
            self.edit_library_source(usage.surface)
        } else {
            self.edit_library(usage.surface)
        }
    }
}
