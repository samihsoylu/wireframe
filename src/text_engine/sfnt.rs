// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Serialize detached font tables for portable exports.
const MAX_TABLES: usize = 256;
pub(super) fn checksum(bytes: &[u8]) -> u32 {
    bytes.chunks(4).fold(0u32, |sum, chunk| {
        let mut word = [0; 4];
        word[..chunk.len()].copy_from_slice(chunk);
        sum.wrapping_add(u32::from_be_bytes(word))
    })
}

/// Original SFNT serialization from detached tables. The head checksum is
/// calculated with adjustment zero; the final whole-font sum is 0xB1B0AFBA.
pub(crate) fn pack(mut tables: Vec<(u32, Vec<u8>)>, limit: usize) -> Result<Vec<u8>, String> {
    if tables.is_empty() || tables.len() > MAX_TABLES {
        return Err("Invalid font table count".into());
    }
    tables.sort_by_key(|(tag, _)| *tag);
    if tables.windows(2).any(|pair| pair[0].0 == pair[1].0) {
        return Err("Duplicate font table".into());
    }
    let head_tag = u32::from_be_bytes(*b"head");
    let head = tables
        .iter_mut()
        .find(|(tag, _)| *tag == head_tag)
        .ok_or("Missing font header")?;
    if head.1.len() < 54 {
        return Err("Truncated font header".into());
    }
    head.1[8..12].fill(0);
    let total = tables
        .iter()
        .try_fold(12 + tables.len() * 16, |size, (_, data)| {
            size.checked_add(data.len().checked_add(3)? & !3)
        })
        .filter(|n| *n <= limit && *n <= u32::MAX as usize)
        .ok_or("Detached font exceeds size limit")?;
    let cff = tables
        .iter()
        .any(|(tag, _)| [*b"CFF ", *b"CFF2"].contains(&tag.to_be_bytes()));
    let mut out = vec![0; 12 + tables.len() * 16];
    out.reserve_exact(total - out.len());
    out[..4].copy_from_slice(if cff { b"OTTO" } else { &[0, 1, 0, 0] });
    let count = tables.len() as u16;
    let selector = count.ilog2() as u16;
    let search = (1u16 << selector) * 16;
    for (offset, value) in [
        (4, count),
        (6, search),
        (8, selector),
        (10, count * 16 - search),
    ] {
        out[offset..offset + 2].copy_from_slice(&value.to_be_bytes());
    }
    let mut head_offset = 0;
    for (index, (tag, data)) in tables.into_iter().enumerate() {
        let offset = out.len();
        if tag == head_tag {
            head_offset = offset;
        }
        let entry = 12 + index * 16;
        for (i, value) in [tag, checksum(&data), offset as u32, data.len() as u32]
            .into_iter()
            .enumerate()
        {
            out[entry + i * 4..entry + i * 4 + 4].copy_from_slice(&value.to_be_bytes());
        }
        out.extend_from_slice(&data);
        out.resize((out.len() + 3) & !3, 0);
    }
    let adjustment = 0xB1B0AFBAu32.wrapping_sub(checksum(&out));
    out[head_offset + 8..head_offset + 12].copy_from_slice(&adjustment.to_be_bytes());
    Ok(out)
}
