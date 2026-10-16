// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Validate the complete IDAT stream with a fixed output buffer. The raster
//! dependency otherwise inflates into a growing Vec before checking image size.
use flate2::{Decompress, FlushDecompress, Status};
struct Stream {
    decoder: Decompress,
    expected: u64,
    ended: bool,
}
impl Stream {
    fn feed(&mut self, mut bytes: &[u8], finish: bool) -> Result<(), String> {
        let mut output = [0u8; 8192];
        loop {
            if self.ended {
                return if bytes.is_empty() {
                    Ok(())
                } else {
                    Err("Trailing compressed glyph data".into())
                };
            }
            if bytes.is_empty() && !finish {
                return Ok(());
            }
            let before_in = self.decoder.total_in();
            let before_out = self.decoder.total_out();
            let status = self
                .decoder
                .decompress(
                    bytes,
                    &mut output,
                    if finish {
                        FlushDecompress::Finish
                    } else {
                        FlushDecompress::None
                    },
                )
                .map_err(|_| "Invalid compressed glyph data".to_string())?;
            if self.decoder.total_out() > self.expected {
                return Err("Color glyph compressed data exceeds decoded budget".into());
            }
            let consumed = usize::try_from(self.decoder.total_in() - before_in)
                .map_err(|_| "Invalid compressed glyph position")?;
            bytes = bytes
                .get(consumed..)
                .ok_or("Invalid compressed glyph position")?;
            let written = self.decoder.total_out() - before_out;
            if status == Status::StreamEnd {
                self.ended = true;
                continue;
            }
            if consumed == 0 && written == 0 {
                return Err("Truncated compressed glyph data".into());
            }
        }
    }
}
pub(super) fn validate(data: &[u8], w: u32, h: u32) -> Result<(), String> {
    if data.len() < 33
        || data[8..12] != 13u32.to_be_bytes()
        || data[26] != 0
        || data[27] != 0
        || data[28] > 1
    {
        return Err("Invalid glyph PNG header".into());
    }
    let depth = data[24];
    let color = data[25];
    let channels = match color {
        0 => 1,
        2 => 3,
        3 => 1,
        4 => 2,
        6 => 4,
        _ => return Err("Invalid glyph PNG color type".into()),
    };
    let valid = match color {
        0 => [1, 2, 4, 8, 16].contains(&depth),
        3 => [1, 2, 4, 8].contains(&depth),
        _ => [8, 16].contains(&depth),
    };
    if !valid {
        return Err("Invalid glyph PNG depth".into());
    }
    let bits = u64::from(depth) * channels;
    let row_bytes = |w: u64, h: u64| {
        if w == 0 || h == 0 {
            0
        } else {
            ((w * bits).div_ceil(8) + 1) * h
        }
    };
    let expected = if data[28] == 0 {
        row_bytes(u64::from(w), u64::from(h))
    } else {
        [
            (0, 0, 8, 8),
            (4, 0, 8, 8),
            (0, 4, 4, 8),
            (2, 0, 4, 4),
            (0, 2, 2, 4),
            (1, 0, 2, 2),
            (0, 1, 1, 2),
        ]
        .iter()
        .map(|&(x, y, dx, dy)| {
            row_bytes(
                u64::from(w).saturating_sub(x).div_ceil(dx),
                u64::from(h).saturating_sub(y).div_ceil(dy),
            )
        })
        .sum()
    };
    let mut stream = Stream {
        decoder: Decompress::new(true),
        expected,
        ended: false,
    };
    let mut offset = 33usize;
    let mut chunks = 0usize;
    let mut ended = false;
    while offset < data.len() {
        chunks += 1;
        if chunks > 4096 {
            return Err("Glyph PNG exceeds chunk budget".into());
        }
        let head = data
            .get(offset..offset + 8)
            .ok_or("Truncated glyph PNG chunk")?;
        let len = u32::from_be_bytes(head[..4].try_into().unwrap()) as usize;
        let end = offset
            .checked_add(12)
            .and_then(|v| v.checked_add(len))
            .ok_or("Invalid glyph PNG length")?;
        let bytes = data
            .get(offset + 8..end - 4)
            .ok_or("Truncated glyph PNG chunk")?;
        if end > data.len() {
            return Err("Truncated glyph PNG checksum".into());
        }
        match &head[4..8] {
            b"IDAT" => stream.feed(bytes, false)?,
            b"IEND" => {
                if len != 0 {
                    return Err("Invalid glyph PNG ending".into());
                }
                ended = true;
                break;
            }
            _ => {}
        }
        offset = end;
    }
    if !ended {
        return Err("Missing glyph PNG ending".into());
    }
    stream.feed(&[], true)?;
    if stream.decoder.total_out() != expected {
        return Err("Glyph PNG decoded length does not match dimensions".into());
    }
    Ok(())
}
