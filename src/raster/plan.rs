// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Immutable image export planning; no file dialogs or destination writes.
use super::{
    BoundsRounding, Options,
    names::{self, FileName, Format},
};
use crate::{
    document::{Id, Project},
    export_plan::{self, Entry, Selection},
    render::Scene,
    workspace::{Editor, ExportSnapshot, SceneScope},
};

#[derive(Clone, Debug)]
pub struct Request {
    pub selection: Selection,
    /// The current page when the user requests group or control scope.
    pub current: Id,
    pub scope: SceneScope,
    pub image: Options,
}
impl Request {
    pub fn new(current: Id) -> Self {
        Self {
            selection: Selection::All,
            current,
            scope: SceneScope::Whole,
            image: Options::default(),
        }
    }
}
/// Construct on the export worker. The private editor owns its document and
/// layout caches; later edits in the interactive editor cannot change output.
pub struct Prepared {
    entries: Vec<Entry>,
    names: Vec<FileName>,
    project_title: String,
    editor: Editor,
    scope: SceneScope,
    options: Options,
}
impl Prepared {
    pub fn new(project: &Project, request: &Request) -> Result<Self, String> {
        Self::from_snapshot(ExportSnapshot::new(project.clone()), request)
    }
    pub fn from_snapshot(mut snapshot: ExportSnapshot, request: &Request) -> Result<Self, String> {
        snapshot.document.validate()?;
        let definition = match &request.selection {
            Selection::Current(id) if *id == request.current => snapshot
                .document
                .definitions
                .iter()
                .find(|definition| definition.id == *id),
            _ => None,
        };
        let component = definition.is_some();
        let entries = if let Some(definition) = definition {
            vec![Entry {
                id: definition.id,
                owner: definition.id,
                page_name: definition.name.clone(),
                alternate_name: None,
            }]
        } else {
            export_plan::select(&snapshot.document, &request.selection, true)?
        };
        if request.scope != SceneScope::Whole
            && (entries.len() != 1 || entries[0].id != request.current)
        {
            return Err("Group and selection exports require the current page only".into());
        }
        let names = names::unique(
            &entries.iter().map(Entry::file_stem).collect::<Vec<_>>(),
            Format::Png,
        )?;
        let project_title = snapshot.document.title.clone();
        // Definitions already have stable surface identities. Do not duplicate
        // their nodes into a page or invalidate captured dynamic values.
        if !component {
            snapshot.document.page_imports.clear();
            snapshot.document.pages = entries
                .iter()
                .map(|entry| entry.page(&snapshot.document))
                .collect::<Result<_, _>>()?;
        }
        let mut options = request.image;
        options.rounding = match request.scope {
            SceneScope::Selection(_) | SceneScope::Group(_) => BoundsRounding::Selection,
            SceneScope::Whole => BoundsRounding::Page,
        };
        let prepared = Self {
            entries,
            names,
            project_title,
            editor: snapshot.into_editor(),
            scope: request.scope.clone(),
            options,
        };
        // Validate group/selection paths before any destination can be opened.
        if prepared.scope != SceneScope::Whole {
            prepared.scene(0)?;
        }
        Ok(prepared)
    }
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }
    pub fn file_names(&self) -> &[FileName] {
        &self.names
    }
    pub fn archive_name(&self) -> FileName {
        FileName::new(&self.project_title, Format::Zip)
    }
    pub fn archive_entry(&self, index: usize) -> Result<String, String> {
        let name = self.names.get(index).ok_or("Invalid export page index")?;
        Ok(names::zip_entry(&self.project_title, name))
    }
    pub fn options(&self) -> Options {
        self.options
    }
    pub fn scene(&self, index: usize) -> Result<Scene, String> {
        let entry = self.entries.get(index).ok_or("Invalid export page index")?;
        self.editor.export_scene(entry.id, &self.scope)
    }
}
