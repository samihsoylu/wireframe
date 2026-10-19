// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::SourceLink;

pub(super) fn decode_target(source: &str) -> String {
    let mut result = String::new();
    let mut chars = source.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\'
            && chars
                .peek()
                .is_some_and(|c| matches!(c, '(' | ')' | '\\' | ','))
        {
            result.push(chars.next().unwrap());
        } else {
            result.push(c);
        }
    }
    result
}
/// Replace only the destination suffix. The bracketed label and all surrounding
/// text remain byte-identical, including Unicode, escapes and nested styling.
pub fn replace_source_link_target(
    source: &str,
    link: &SourceLink,
    target: Option<&str>,
) -> Result<String, String> {
    let Some(label_end) = link.label.end.checked_add(1) else {
        return Err("Invalid text link range".into());
    };
    if link.label.start == 0
        || link.range.start.checked_add(1) != Some(link.label.start)
        || source.get(link.range.start..link.label.start) != Some("[")
        || source.get(link.label.end..label_end) != Some("]")
        || link.range.end < label_end
        || !source.is_char_boundary(link.range.end)
    {
        return Err("The text link no longer matches its source".into());
    }
    let mut suffix = String::new();
    if let Some(target) = target.filter(|s| !s.is_empty()) {
        suffix.push('(');
        for c in target.chars() {
            if matches!(c, '(' | ')' | '\\' | ',') {
                suffix.push('\\');
            }
            suffix.push(c);
        }
        suffix.push(')');
    }
    let mut result = source.to_owned();
    result.replace_range(label_end..link.range.end, &suffix);
    Ok(result)
}
