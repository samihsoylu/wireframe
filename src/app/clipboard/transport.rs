// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::Clipboard;
use std::io::{self, Write};

// Bound the serialized JSON, including embedded assets, consistently with files.
// The version marker is outside this bound on both publication and reading.
pub(in crate::app) const MAX_JSON_BYTES: usize = wireframe::document::MAX_FILE_BYTES;

struct LimitedJson {
    bytes: Vec<u8>,
    remaining: usize,
}
impl Write for LimitedJson {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > self.remaining {
            return Err(io::Error::other("Clipboard content is too large"));
        }
        self.bytes.extend_from_slice(bytes);
        self.remaining -= bytes.len();
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub(super) fn clipboard_text(copy: &Clipboard) -> Result<String, String> {
    encode(copy, MAX_JSON_BYTES)
}
fn encode(copy: &Clipboard, limit: usize) -> Result<String, String> {
    let mut writer = LimitedJson {
        bytes: if copy.assets.iter().any(|asset| asset.attribution.is_some())
            || copy.resources.as_ref().is_some_and(|resources| {
                resources
                    .assets
                    .iter()
                    .any(|asset| asset.attribution.is_some())
            })
            || copy
                .pages
                .as_ref()
                .is_some_and(|pages| pages.project().format_version >= 16)
        {
            b"wireframe-nodes-v3:".to_vec()
        } else {
            b"wireframe-nodes-v2:".to_vec()
        },
        remaining: limit,
    };
    serde_json::to_writer(&mut writer, copy).map_err(|e| e.to_string())?;
    String::from_utf8(writer.bytes).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_json_boundary_and_utf8_match_previous_wire_format() {
        let copy = Clipboard {
            origin: Some("世界".into()),
            ..Default::default()
        };
        let json = serde_json::to_string(&copy).unwrap();
        assert_eq!(
            encode(&copy, json.len()).unwrap(),
            format!("wireframe-nodes-v2:{json}")
        );
        assert!(
            encode(&copy, json.len() - 1)
                .unwrap_err()
                .contains("too large")
        );
        let mut writer = LimitedJson {
            bytes: vec![],
            remaining: 3,
        };
        writer.write_all(b"ab").unwrap();
        assert!(writer.write_all(b"cd").is_err());
        assert_eq!(writer.bytes, b"ab");
        assert_eq!(writer.remaining, 1);
    }
}

#[cfg(test)]
mod credit_tests {
    use super::*;
    use wireframe::document::{Asset, ImageAttribution};
    fn asset() -> Asset {
        Asset {
            id: 2,
            trashed: false,
            name: "Artwork".into(),
            media_type: "image/png".into(),
            bytes: vec![1],
            attribution: Some(ImageAttribution {
                title: "Artwork".into(),
                creator: "Creator".into(),
                license: "CC BY 4.0".into(),
                license_url: "https://creativecommons.org/licenses/by/4.0/".into(),
                source_url: "https://commons.wikimedia.org/wiki/File:Artwork.jpg".into(),
            }),
        }
    }
    #[test]
    fn credited_control_and_library_payloads_use_a_marker_old_readers_do_not_accept() {
        for library in [false, true] {
            let mut copy = Clipboard::default();
            if library {
                copy.resources = Some(wireframe::workspace::LibraryClipboard {
                    project: "origin".into(),
                    requested: vec![1],
                    libraries: vec![],
                    definitions: vec![],
                    assets: vec![asset()],
                    removed_assets: Default::default(),
                });
            } else {
                copy.assets.push(asset());
            }
            let text = clipboard_text(&copy).unwrap();
            assert!(text.starts_with("wireframe-nodes-v3:"));
            assert!(text.strip_prefix("wireframe-nodes-v2:").is_none());
            let restored: Clipboard =
                serde_json::from_str(text.strip_prefix("wireframe-nodes-v3:").unwrap()).unwrap();
            let assets = restored
                .resources
                .as_ref()
                .map_or(&restored.assets, |r| &r.assets);
            assert_eq!(assets[0].attribution, asset().attribution);
        }
    }
}
