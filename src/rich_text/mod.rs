// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Source-preserving rich text. Parsing and drawing use the same styled spans.
pub mod painting;
mod source_lines;
pub mod structured;

#[derive(serde::Serialize, Clone, Debug, Default, PartialEq)]
pub struct Style {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strike: bool,
    pub disabled: bool,
    pub color: Option<String>,
    pub size: Option<u16>,
    pub link: bool,
    pub target: Option<String>,
}
#[derive(serde::Serialize, Clone, Copy, Debug, PartialEq)]
pub enum Break {
    Hard,
    Soft,
}
#[derive(serde::Serialize, Clone, Debug, PartialEq)]
pub enum Inline {
    Span { text: String, style: Style },
    Break(Break),
    Bullet,
}
#[derive(serde::Serialize, Clone, Debug, Default, PartialEq)]
pub struct RichText {
    pub source: String,
    pub items: Vec<Inline>,
    pub complexity_limited: bool,
}
impl RichText {
    pub fn plain(text: &str) -> Self {
        Self {
            source: text.into(),
            items: vec![Inline::Span {
                text: text.into(),
                style: Style::default(),
            }],
            complexity_limited: false,
        }
    }
    pub fn parse(text: &str, bullets: bool) -> Self {
        let mut parser = Parser {
            output: Self {
                source: text.into(),
                ..Self::default()
            },
            budget: text.len().saturating_mul(32).max(1),
            bullets,
        };
        parser.segment(text, Style::default(), 0);
        parser.output
    }
    pub fn display_text(&self, soft_breaks: bool) -> String {
        self.items
            .iter()
            .map(|i| match i {
                Inline::Span { text, .. } => text.as_str(),
                Inline::Break(Break::Hard) => "\n",
                Inline::Break(Break::Soft) => {
                    if soft_breaks {
                        "\n"
                    } else {
                        ""
                    }
                }
                Inline::Bullet => "• ",
            })
            .collect()
    }
}
struct Parser {
    output: RichText,
    budget: usize,
    bullets: bool,
}
impl Parser {
    fn push(&mut self, text: &str, style: &Style) {
        if text.is_empty() {
            return;
        }
        if let Some(Inline::Span {
            text: previous,
            style: old,
        }) = self.output.items.last_mut()
            && old == style
        {
            previous.push_str(text);
        } else {
            self.output.items.push(Inline::Span {
                text: text.into(),
                style: style.clone(),
            });
        }
    }
    fn find(&mut self, text: &str, needle: &str, case_insensitive: bool) -> Option<usize> {
        let mut escaped = false;
        for (i, ch) in text.char_indices() {
            if self.budget == 0 {
                self.output.complexity_limited = true;
                return None;
            }
            self.budget -= 1;
            if escaped {
                escaped = false;
                continue;
            }
            if ch == '\\' {
                escaped = true;
                continue;
            }
            if let Some(candidate) = text.get(i..i + needle.len())
                && (candidate == needle
                    || case_insensitive && candidate.eq_ignore_ascii_case(needle))
            {
                return Some(i);
            }
        }
        None
    }
    fn segment(&mut self, text: &str, style: Style, depth: usize) {
        if depth >= 24 {
            self.output.complexity_limited = true;
            self.push(text, &style);
            return;
        }
        let mut i = 0;
        let mut line_start = true;
        while i < text.len() {
            if self.budget == 0 {
                self.output.complexity_limited = true;
                self.push(&text[i..], &style);
                return;
            }
            self.budget -= 1;
            let rest = &text[i..];
            let ch = rest.chars().next().unwrap();
            if depth == 0
                && line_start
                && self.bullets
                && (rest.starts_with("* ") || rest.starts_with("- "))
            {
                self.output.items.push(Inline::Bullet);
                i += 2;
                line_start = false;
                continue;
            }
            if ch == '\n' || ch == '\r' || ch == '\u{2028}' || ch == '\u{2029}' {
                self.output.items.push(Inline::Break(Break::Hard));
                i += ch.len_utf8();
                line_start = true;
                continue;
            }
            line_start = false;
            if ch == '\\'
                && let Some(next) = rest[1..].chars().next()
            {
                if next == 'n' || next == 'r' {
                    self.output.items.push(Inline::Break(Break::Soft));
                    i += 2;
                    continue;
                }
                if "*_&~-[]{}(),\\".contains(next) {
                    self.push(&rest[1..1 + next.len_utf8()], &style);
                    i += 1 + next.len_utf8();
                    continue;
                }
            }
            if "*_&~-".contains(ch) {
                let after = &rest[ch.len_utf8()..];
                if after
                    .chars()
                    .next()
                    .is_some_and(|c| !c.is_whitespace() && c != ch)
                    && let Some(end) = self.find(after, &ch.to_string(), false)
                {
                    let inner = &after[..end];
                    if !inner.is_empty() && inner.chars().last().is_some_and(|c| !c.is_whitespace())
                    {
                        let mut child = style.clone();
                        match ch {
                            '*' => child.bold = true,
                            '_' => child.italic = true,
                            '&' => child.underline = true,
                            '~' => child.strike = true,
                            '-' => child.disabled = true,
                            _ => {}
                        }
                        self.segment(inner, child, depth + 1);
                        i += end + 2 * ch.len_utf8();
                        continue;
                    }
                }
            }
            if ch == '['
                && let Some(end) = self.find(&rest[1..], "]", false)
            {
                let inner = &rest[1..1 + end];
                if !inner.is_empty() && inner.trim() == inner {
                    let mut length = end + 2;
                    let mut child = style.clone();
                    child.link = true;
                    if rest[length..].starts_with('(')
                        && let Some(close) = self.find(&rest[length + 1..], ")", false)
                    {
                        child.target = Some(rest[length + 1..length + 1 + close].into());
                        length += close + 2;
                    }
                    self.segment(inner, child, depth + 1);
                    i += length;
                    continue;
                }
            }
            if ch == '{'
                && let Some(open_end) = self.find(rest, "}", false)
            {
                let tag = &rest[1..open_end];
                if let Some((name, value)) = tag.split_once(':') {
                    let mut child = style.clone();
                    let value = value.trim();
                    let recognized = if name.eq_ignore_ascii_case("size")
                        && !value.is_empty()
                        && value.bytes().all(|b| b.is_ascii_digit())
                    {
                        if let Ok(size) = value.parse::<u16>() {
                            child.size = Some(size);
                            true
                        } else {
                            false
                        }
                    } else if (name.eq_ignore_ascii_case("color")
                        || name.eq_ignore_ascii_case("colour"))
                        && !value.is_empty()
                    {
                        child.color = Some(value.into());
                        true
                    } else {
                        false
                    };
                    if recognized {
                        let close = format!("{{{name}}}");
                        let after = &rest[open_end + 1..];
                        if let Some(end) = self.find(after, &close, true) {
                            self.segment(&after[..end], child, depth + 1);
                            i += open_end + 1 + end + close.len();
                            continue;
                        }
                    }
                }
            }
            self.push(&rest[..ch.len_utf8()], &style);
            i += ch.len_utf8();
        }
    }
}
