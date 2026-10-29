// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Explicit, bounded Commons searches. No document or image data is uploaded.
use crate::document::ImageAttribution;
use serde::Deserialize;
use std::{
    io::Read,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};
use url::Url;

const API: &str = "https://commons.wikimedia.org/w/api.php";
const MAX_RESPONSE: usize = 1024 * 1024;
const USER_AGENT: &str = concat!(
    "Wireframe/",
    env!("CARGO_PKG_VERSION"),
    " (https://github.com/samihsoylu/wireframe)"
);

#[derive(Clone, Debug)]
pub struct Hit {
    pub address: String,
    pub thumbnail: Option<String>,
    pub width: u32,
    pub height: u32,
    pub attribution: ImageAttribution,
}
#[derive(Debug)]
pub struct Results {
    pub hits: Vec<Hit>,
    pub next: Option<u64>,
}
#[derive(Deserialize)]
struct Response {
    query: Option<Query>,
    #[serde(rename = "continue")]
    continuation: Option<Continuation>,
    error: Option<serde_json::Value>,
}
#[derive(Deserialize)]
struct Continuation {
    gsroffset: Option<u64>,
}
#[derive(Deserialize)]
struct Query {
    #[serde(default)]
    pages: Vec<Page>,
}
#[derive(Deserialize)]
struct Page {
    title: String,
    #[serde(default)]
    index: u32,
    #[serde(default)]
    imageinfo: Vec<Info>,
}
#[derive(Deserialize)]
struct Info {
    url: String,
    thumburl: Option<String>,
    descriptionurl: String,
    mime: String,
    width: u32,
    height: u32,
    #[serde(default)]
    extmetadata: std::collections::BTreeMap<String, Metadata>,
}
#[derive(Deserialize)]
struct Metadata {
    value: String,
}

fn check_cancel(cancel: &AtomicBool) -> Result<(), String> {
    if cancel.load(Ordering::Relaxed) {
        Err("Image search cancelled".into())
    } else {
        Ok(())
    }
}
fn request_url(endpoint: &str, query: &str, offset: Option<u64>) -> Result<Url, String> {
    let query = query.trim();
    if query.is_empty() || query.chars().count() > 200 || query.chars().any(char::is_control) {
        return Err("Enter a search of up to 200 characters".into());
    }
    let mut url = Url::parse(endpoint).map_err(|_| "Cannot open image search")?;
    let bitmap_query = format!("{query} filetype:bitmap");
    url.query_pairs_mut().extend_pairs([
        ("action", "query"),
        ("format", "json"),
        ("formatversion", "2"),
        ("generator", "search"),
        ("gsrnamespace", "6"),
        ("gsrlimit", "8"),
        ("gsrsearch", bitmap_query.as_str()),
        ("iiurlwidth", "240"),
        ("iiurlheight", "160"),
        ("prop", "imageinfo"),
        ("iiprop", "url|size|mime|extmetadata"),
        ("iilimit", "1"),
        ("iiextmetadatalanguage", "en"),
        (
            "iiextmetadatafilter",
            "Artist|LicenseShortName|LicenseUrl|UsageTerms",
        ),
        ("maxlag", "5"),
    ]);
    if let Some(offset) = offset {
        url.query_pairs_mut()
            .append_pair("gsroffset", &offset.to_string());
    }
    Ok(url)
}
pub fn thumbnail(address: &str, cancel: &AtomicBool) -> Result<super::Decoded, String> {
    if !safe_url(address, false) {
        return Err("The search thumbnail has an unsupported address".into());
    }
    super::web::thumbnail(address, cancel)
}
pub fn search(query: &str, offset: Option<u64>, cancel: &AtomicBool) -> Result<Results, String> {
    search_at(API, query, offset, cancel)
}
fn search_at(
    endpoint: &str,
    query: &str,
    offset: Option<u64>,
    cancel: &AtomicBool,
) -> Result<Results, String> {
    check_cancel(cancel)?;
    let url = request_url(endpoint, query, offset)?;
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .max_redirects(0)
        .http_status_as_error(false)
        .timeout_global(Some(Duration::from_secs(20)))
        .build()
        .into();
    let mut response = agent
        .get(url.as_str())
        .header("User-Agent", USER_AGENT)
        .header("Accept", "application/json")
        .call()
        .map_err(|_| "Cannot search for images. Check your connection and try again.")?;
    check_cancel(cancel)?;
    if !response.status().is_success() {
        return Err(if response.status().as_u16() == 429 {
            "Image search is busy. Wait a moment and try again.".into()
        } else {
            format!(
                "Image search returned HTTP {}. Try again later.",
                response.status().as_u16()
            )
        });
    }
    let mut reader = response.body_mut().as_reader();
    let mut bytes = Vec::new();
    let mut chunk = [0; 8192];
    loop {
        check_cancel(cancel)?;
        let count = reader
            .read(&mut chunk)
            .map_err(|_| "The search response was interrupted. Try again.")?;
        if count == 0 {
            break;
        }
        if bytes.len() + count > MAX_RESPONSE {
            return Err("The search response was too large. Try a more specific search.".into());
        }
        bytes.extend_from_slice(&chunk[..count]);
    }
    check_cancel(cancel)?;
    parse(&bytes)
}
fn safe_url(address: &str, source: bool) -> bool {
    address.len() <= 4000
        && Url::parse(address).is_ok_and(|url| {
            url.scheme() == "https"
                && url.username().is_empty()
                && url.password().is_none()
                && url.port().is_none()
                && url.fragment().is_none()
                && if source {
                    url.host_str() == Some("commons.wikimedia.org")
                } else {
                    url.host_str() == Some("upload.wikimedia.org")
                }
        })
}
fn parse(bytes: &[u8]) -> Result<Results, String> {
    let response: Response = serde_json::from_slice(bytes)
        .map_err(|_| "Cannot read the image search response. Try again later.")?;
    if response.error.is_some() {
        return Err("Image search is temporarily unavailable. Try again later.".into());
    }
    let mut pages = response.query.map_or_else(Vec::new, |q| q.pages);
    pages.sort_by_key(|p| p.index);
    let hits = pages
        .into_iter()
        .take(8)
        .filter_map(|page| {
            let info = page.imageinfo.into_iter().next()?;
            if !matches!(info.mime.as_str(), "image/png" | "image/jpeg" | "image/gif")
                || info.width == 0
                || info.height == 0
                || !safe_url(&info.url, false)
                || !safe_url(&info.descriptionurl, true)
            {
                return None;
            }
            let meta = |key: &str| {
                info.extmetadata
                    .get(key)
                    .map(|m| plain_text(&m.value))
                    .unwrap_or_default()
            };
            let mut license = meta("LicenseShortName");
            if license.is_empty() {
                license = meta("UsageTerms");
            }
            if license.is_empty() {
                return None;
            }
            let license_url = meta("LicenseUrl");
            let license_url = if license_url.starts_with("//") {
                format!("https:{license_url}")
            } else {
                license_url
            };
            let license_url = Url::parse(&license_url)
                .ok()
                .filter(|u| {
                    u.as_str().len() <= 4000
                        && u.scheme() == "https"
                        && u.username().is_empty()
                        && u.password().is_none()
                })
                .map(|u| u.to_string())
                .unwrap_or_default();
            Some(Hit {
                thumbnail: info.thumburl.filter(|url| safe_url(url, false)),
                address: info.url,
                width: info.width,
                height: info.height,
                attribution: ImageAttribution {
                    title: plain_text(page.title.strip_prefix("File:").unwrap_or(&page.title)),
                    creator: meta("Artist"),
                    license,
                    license_url,
                    source_url: info.descriptionurl,
                },
            })
        })
        .collect();
    Ok(Results {
        hits,
        next: response.continuation.and_then(|c| c.gsroffset),
    })
}
// Metadata is HTML. Display only bounded text; never render tags or follow their links.
fn plain_text(html: &str) -> String {
    let mut text = String::new();
    let mut tag = false;
    for ch in html.chars().take(32_000) {
        if ch == '<' {
            tag = true;
            text.push(' ');
        } else if ch == '>' {
            tag = false;
        } else if !tag {
            text.push(ch);
        }
    }
    let mut decoded = String::new();
    let mut rest = text.as_str();
    while let Some(index) = rest.find('&') {
        decoded.push_str(&rest[..index]);
        rest = &rest[index + 1..];
        if let Some(end) = rest.find(';').filter(|end| *end <= 16) {
            let entity = &rest[..end];
            let value = match entity {
                "amp" => Some('&'),
                "quot" => Some('"'),
                "apos" => Some('\''),
                "nbsp" => Some(' '),
                "lt" => Some('<'),
                "gt" => Some('>'),
                _ => entity
                    .strip_prefix("#x")
                    .or_else(|| entity.strip_prefix("#X"))
                    .and_then(|n| u32::from_str_radix(n, 16).ok())
                    .or_else(|| entity.strip_prefix('#').and_then(|n| n.parse::<u32>().ok()))
                    .and_then(char::from_u32),
            };
            if let Some(value) = value {
                decoded.push(value);
                rest = &rest[end + 1..];
                continue;
            }
        }
        decoded.push('&');
    }
    decoded.push_str(rest);
    let mut text: String = decoded
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .filter(|c| !c.is_control())
        .collect();
    if text.len() > 4000 {
        let mut end = 4000;
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        text.truncate(end);
    }
    text
}

#[cfg(test)]
mod tests;
