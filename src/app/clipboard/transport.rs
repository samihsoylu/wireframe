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
        bytes: b"wireframe-nodes-v2:".to_vec(),
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
