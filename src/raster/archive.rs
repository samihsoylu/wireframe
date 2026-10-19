// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! PNG entries streamed directly into a temporary ZIP, one scene at a time.
use super::{Geometry, plan::Prepared};
use chrono::{Datelike, Timelike};
use std::io::{Seek, Write};

pub fn write_zip(
    plan: &Prepared,
    output: impl Write + Seek,
    mut checkpoint: impl FnMut(usize, u32, u32) -> Result<(), String>,
) -> Result<Vec<Geometry>, String> {
    let mut archive = zip::ZipWriter::new(output);
    let now = chrono::Local::now();
    let timestamp = zip::DateTime::from_date_and_time(
        now.year().clamp(1980, 2107) as u16,
        now.month() as u8,
        now.day() as u8,
        now.hour() as u8,
        now.minute() as u8,
        now.second().min(59) as u8,
    )
    .unwrap_or_default();
    // PNG is already compressed. Reserve ZIP64 sizes because entry length is
    // unknown while streaming; do not buffer a PNG to discover its size first.
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Stored)
        .large_file(true)
        .last_modified_time(timestamp)
        .unix_permissions(0o644);
    let mut geometry = Vec::with_capacity(plan.entries().len());
    for index in 0..plan.entries().len() {
        checkpoint(index, 0, 0)?;
        let scene = plan.scene(index)?;
        archive
            .start_file(plan.archive_entry(index)?, options)
            .map_err(|e| format!("Cannot start ZIP image: {e}"))?;
        geometry.push(super::write_png(
            &scene,
            plan.options(),
            &mut archive,
            |row, total| checkpoint(index, row, total),
        )?);
    }
    archive
        .finish()
        .map_err(|e| format!("Cannot finish ZIP archive: {e}"))?;
    Ok(geometry)
}
