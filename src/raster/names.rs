// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Portable components and deterministic names, shared by folders and ZIP entries.
use std::collections::HashSet;
use unicode_normalization::UnicodeNormalization;

const MAX_STEM_BYTES: usize = 180;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Png,
    Zip,
}
impl Format {
    fn extension(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Zip => "zip",
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileName {
    stem: String,
    format: Format,
}
impl FileName {
    pub fn new(label: &str, format: Format) -> Self {
        Self {
            stem: component(label),
            format,
        }
    }
    pub fn text(&self) -> String {
        format!("{}.{}", self.stem, self.format.extension())
    }
    pub fn key(&self) -> String {
        collision_key(&self.text())
    }
    pub fn numbered(&self, number: u64) -> Self {
        let suffix = format!("_{number}");
        let base = truncate(&self.stem, MAX_STEM_BYTES - suffix.len());
        Self {
            stem: format!("{base}{suffix}"),
            format: self.format,
        }
    }
}
fn truncate(value: &str, bytes: usize) -> &str {
    let mut end = value.len().min(bytes);
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    &value[..end]
}
/// Use a conservative canonical/case key; publication still relies on the
/// destination filesystem's exclusive operation, not this comparison alone.
pub fn collision_key(value: &str) -> String {
    value.nfd().flat_map(char::to_uppercase).nfd().collect()
}
pub fn component(label: &str) -> String {
    let sanitized: String = label
        .nfc()
        .map(|ch| {
            if ch.is_control()
                || matches!(
                    ch,
                    '/' | '\\' | '?' | '%' | '*' | ':' | '|' | '"' | '<' | '>'
                )
            {
                '_'
            } else {
                ch
            }
        })
        .collect();
    let mut stem = truncate(&sanitized, MAX_STEM_BYTES)
        .trim_end_matches([' ', '.'])
        .to_owned();
    if stem.trim().is_empty() {
        stem = "Untitled".into();
    }
    // Device names remain reserved when followed by an extension.
    let root = stem
        .split('.')
        .next()
        .unwrap_or("")
        .trim_end()
        .to_uppercase();
    let numbered_device = |prefix: &str| {
        root.strip_prefix(prefix).is_some_and(|number| {
            matches!(
                number,
                "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
            )
        })
    };
    if matches!(
        root.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
    ) || numbered_device("COM")
        || numbered_device("LPT")
    {
        stem.insert(0, '_');
        stem = truncate(&stem, MAX_STEM_BYTES).to_owned();
    }
    stem
}
/// Reserve all original sanitized names before assigning duplicate suffixes,
/// so an automatically numbered duplicate cannot consume another page's name.
pub fn unique(labels: &[String], format: Format) -> Result<Vec<FileName>, String> {
    let names: Vec<_> = labels
        .iter()
        .map(|label| FileName::new(label, format))
        .collect();
    let reserved: HashSet<_> = names.iter().map(FileName::key).collect();
    let mut used = HashSet::new();
    let mut output = Vec::with_capacity(names.len());
    for name in names {
        if used.insert(name.key()) {
            output.push(name);
            continue;
        }
        let mut number = 2u64;
        loop {
            let candidate = name.numbered(number);
            let key = candidate.key();
            if !reserved.contains(&key) && used.insert(key) {
                output.push(candidate);
                break;
            }
            number = number
                .checked_add(1)
                .ok_or("Cannot assign a unique export filename")?;
        }
    }
    Ok(output)
}
/// Both components are generated here; no caller-supplied path separators or
/// parent components can reach an archive entry.
pub fn zip_entry(project: &str, name: &FileName) -> String {
    format!("{}/{}", component(project), name.text())
}
