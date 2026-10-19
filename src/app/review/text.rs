// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::rich_text::{RichText, source_links};

/// Readable, stable aliases for typed destinations. The dictionary survives
/// text-local Undo and removed tokens; only actual unescaped tokens are encoded.
#[derive(Default)]
pub(super) struct Text {
    pub value: String,
    tokens: Vec<(String, String, String)>,
}
impl Text {
    #[cfg(test)]
    pub fn from_source(source: &str) -> Result<Self, String> {
        let mut text = Self::default();
        text.show_source(source)?;
        Ok(text)
    }
    pub fn show_source(&mut self, source: &str) -> Result<(), String> {
        let links = source_links(source)?;
        let mut value = String::new();
        let mut start = 0;
        for link in links {
            if link.range.start < start {
                continue;
            }
            let Some(target) = link
                .target
                .as_deref()
                .filter(|t| t.starts_with("callout:") || t.starts_with("surface:"))
            else {
                continue;
            };
            value.push_str(&source[start..link.range.start]);
            let raw_label = &source[link.label];
            let label = RichText::parse(raw_label, false).display_text(true);
            let token = self.register(&label, target, raw_label);
            value.push_str(&token);
            start = link.range.end;
        }
        value.push_str(&source[start..]);
        self.value = value;
        Ok(())
    }
    pub fn alias(&mut self, label: &str, target: &str) -> String {
        let source_label: String = label
            .chars()
            .flat_map(|c| {
                if "\\[]*_&~{}-".contains(c) {
                    vec!['\\', c]
                } else {
                    vec![c]
                }
            })
            .collect();
        self.register(label, target, &source_label)
    }
    fn register(&mut self, label: &str, target: &str, source_label: &str) -> String {
        let marker = target.starts_with("callout:");
        if let Some((token, _, _)) = self
            .tokens
            .iter()
            .find(|(_, t, label)| t == target && (marker || label == source_label))
        {
            return token.clone();
        }
        let label: String = label
            .chars()
            .map(|c| match c {
                '[' | ']' | '(' | ')' | '\\' | '\n' | '\r' => ' ',
                _ => c,
            })
            .collect();
        let label = label.trim();
        let label = if label.is_empty() { "Link" } else { label };
        let wrap = |s: &str| {
            if marker {
                format!("({s})")
            } else {
                format!("[{s}]")
            }
        };
        let mut token = wrap(label);
        let mut suffix = 1;
        while self.tokens.iter().any(|(old, _, _)| old == &token) {
            token = wrap(&format!("{label}.{suffix}"));
            suffix += 1;
        }
        self.tokens
            .push((token.clone(), target.into(), source_label.into()));
        self.tokens
            .sort_by_key(|(token, _, _)| std::cmp::Reverse(token.len()));
        token
    }
    pub fn encode(&self, caret: usize) -> Result<(String, usize), String> {
        if !self.value.is_char_boundary(caret) {
            return Err("Invalid comment caret".into());
        }
        let links = source_links(&self.value)?;
        let protected: Vec<_> = links
            .into_iter()
            .filter(|l| l.target.is_some())
            .map(|l| l.range)
            .collect();
        let mut output = String::new();
        let mut mapped = 0;
        let mut i = 0;
        while i < self.value.len() {
            let next = if self.value[i..].starts_with("\\[") {
                // An escaped link is literal text, including marker-like text
                // inside its label and destination.
                escaped_link_end(&self.value, i).unwrap_or(i + 2)
            } else if self.value[i..].starts_with('\\') {
                i + 1 + self.value[i + 1..].chars().next().map_or(0, char::len_utf8)
            } else if self.value[i..].starts_with('[')
                && let Some(range) = protected.iter().find(|r| r.start == i)
            {
                range.end
            } else {
                i
            };
            if next > i {
                if caret >= i {
                    mapped = output.len() + caret.min(next) - i;
                }
                output.push_str(&self.value[i..next]);
                i = next;
                continue;
            }
            let boundary = i == 0
                || self.value[..i]
                    .chars()
                    .next_back()
                    .is_none_or(|c| !c.is_alphanumeric());
            let token = (boundary && self.value[i..].starts_with(['(', '[']))
                .then(|| {
                    self.tokens.iter().find(|(token, _, _)| {
                        self.value[i..].starts_with(token)
                            && self.value[i + token.len()..]
                                .chars()
                                .next()
                                .is_none_or(|c| !c.is_alphanumeric())
                    })
                })
                .flatten();
            if let Some((token, target, source_label)) = token {
                if caret >= i {
                    mapped = output.len();
                }
                let label = if target.starts_with("surface:") {
                    source_label
                } else {
                    token
                };
                output.push_str(&format!("[{label}]({target})"));
                i += token.len();
                if caret >= i {
                    mapped = output.len();
                }
            } else {
                let ch = self.value[i..].chars().next().unwrap();
                if caret >= i {
                    mapped = output.len();
                }
                output.push(ch);
                i += ch.len_utf8();
            }
        }
        if caret == self.value.len() {
            mapped = output.len();
        }
        Ok((output, mapped))
    }
}

fn escaped_link_end(text: &str, start: usize) -> Option<usize> {
    let find = |from: usize, needle: char| {
        let mut escaped = false;
        text[from..].char_indices().find_map(|(i, c)| {
            if escaped {
                escaped = false;
                return None;
            }
            if c == '\\' {
                escaped = true;
                return None;
            }
            (c == needle).then_some(from + i + c.len_utf8())
        })
    };
    let end = find(start + 2, ']')?;
    if text[end..].starts_with('(') {
        find(end + 1, ')').or(Some(end))
    } else {
        Some(end)
    }
}
