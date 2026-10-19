// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use std::{
    hash::{BuildHasher, Hasher},
    io::Read,
};

pub(super) struct Identity {
    pub author: Author,
    pub name: String,
    path: Option<PathBuf>,
    pub error: Option<String>,
}
impl Default for Identity {
    fn default() -> Self {
        let random = || {
            std::collections::hash_map::RandomState::new()
                .build_hasher()
                .finish()
        };
        let author = Author {
            id: format!("local-{:016x}{:016x}", random(), random()),
            name: "Reviewer".into(),
        };
        Self {
            name: author.name.clone(),
            author,
            path: None,
            error: None,
        }
    }
}
impl Identity {
    pub fn load(&mut self, root: &std::path::Path) {
        self.path = Some(root.join("reviewer.json"));
        let result = (|| {
            let file = match std::fs::File::open(self.path.as_ref().unwrap()) {
                Ok(file) => file,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
                Err(e) => return Err(e.to_string()),
            };
            let mut data = vec![];
            file.take(4097)
                .read_to_end(&mut data)
                .map_err(|e| e.to_string())?;
            if data.len() > 4096 {
                return Err("Reviewer settings are too large".into());
            }
            let author: Author = serde_json::from_slice(&data).map_err(|e| e.to_string())?;
            Self::validate(&author)?;
            self.name = author.name.clone();
            self.author = author;
            Ok(())
        })();
        self.error = result.err();
    }
    fn validate(author: &Author) -> Result<(), String> {
        let mut discussion = Discussion::default();
        discussion
            .post(
                author.clone(),
                None,
                wireframe::review::Body {
                    text: "validate".into(),
                    callouts: vec![],
                },
                0,
            )
            .map(|_| ())
    }
    pub fn save(&mut self) -> Result<Author, String> {
        if let Some(error) = &self.error {
            return Err(format!(
                "Reviewer settings could not be read and have been preserved: {error}"
            ));
        }
        let author = Author {
            id: self.author.id.clone(),
            name: self.name.trim().into(),
        };
        Self::validate(&author)?;
        if let Some(path) = &self.path {
            storage::write_atomic(
                path,
                &serde_json::to_vec_pretty(&author).map_err(|e| e.to_string())?,
            )?;
        }
        self.author = author.clone();
        Ok(author)
    }
}
