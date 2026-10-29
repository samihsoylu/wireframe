// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use serde_json::json;
use std::{io::Write, net::TcpListener, thread};

fn page(index: u32, name: &str) -> serde_json::Value {
    json!({"index":index,"title":format!("File:{name}"),"imageinfo":[{
        "url":"https://upload.wikimedia.org/wikipedia/commons/a/ab/Picture.jpg",
        "descriptionurl":"https://commons.wikimedia.org/wiki/File:Picture.jpg",
        "mime":"image/jpeg","width":120,"height":80,
        "extmetadata":{
            "Artist":{"value":"<a href=\"https://example.org\">Jane &amp; Jo</a>"},
            "LicenseShortName":{"value":"CC BY-SA 4.0"},
            "LicenseUrl":{"value":"//creativecommons.org/licenses/by-sa/4.0/"}
        }
    }]})
}
#[test]
fn ordered_results_preserve_credits_and_reject_unusable_or_untrusted_media() {
    let good = page(2, "Second.jpg");
    let mut svg = page(3, "Vector.svg");
    svg["imageinfo"][0]["mime"] = json!("image/svg+xml");
    let mut hostile = page(4, "Hostile.jpg");
    hostile["imageinfo"][0]["url"] = json!("https://upload.wikimedia.org.evil.example/picture.jpg");
    let mut missing = page(5, "No licence.jpg");
    missing["imageinfo"][0]["extmetadata"] = json!({});
    let results = parse(&serde_json::to_vec(&json!({"query":{"pages":[good,svg,hostile,missing,page(1,"First.jpg")]},"continue":{"gsroffset":8}})).unwrap()).unwrap();
    assert_eq!(results.hits.len(), 2);
    assert_eq!(results.hits[0].attribution.title, "First.jpg");
    assert_eq!(results.hits[0].attribution.creator, "Jane & Jo");
    assert_eq!(
        results.hits[0].attribution.license_url,
        "https://creativecommons.org/licenses/by-sa/4.0/"
    );
    assert!(
        results.hits[0]
            .attribution
            .credit()
            .contains("https://commons.wikimedia.org/wiki/File:Picture.jpg")
    );
    assert_eq!(results.next, Some(8));
    assert!(
        parse(br#"{"error":{"code":"maxlag"}}"#)
            .unwrap_err()
            .contains("temporarily unavailable")
    );
    assert!(parse(b"<html>bad gateway</html>").is_err());
    assert!(parse(br#"{"batchcomplete":true}"#).unwrap().hits.is_empty());
}
fn server(status: &str, body: &[u8]) -> (String, thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = format!("http://{}/w/api.php", listener.local_addr().unwrap());
    let reply = format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )
    .into_bytes();
    let body = body.to_vec();
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut request = vec![];
        while !request.ends_with(b"\r\n\r\n") {
            let mut b = [0];
            stream.read_exact(&mut b).unwrap();
            request.push(b[0]);
        }
        let _ = stream.write_all(&reply);
        let _ = stream.write_all(&body);
        String::from_utf8(request).unwrap()
    });
    (address, handle)
}
#[test]
fn search_sends_only_explicit_encoded_query_and_public_client_identity() {
    let body = serde_json::to_vec(&json!({"query":{"pages":[page(1,"Canal.jpg")]}})).unwrap();
    let (endpoint, worker) = server("200 OK", &body);
    let result = search_at(
        &endpoint,
        " café & canal ",
        Some(8),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(result.hits.len(), 1);
    let request = worker.join().unwrap();
    let path = request
        .lines()
        .next()
        .unwrap()
        .split_whitespace()
        .nth(1)
        .unwrap();
    let url = Url::parse(&format!("http://localhost{path}")).unwrap();
    let params: std::collections::HashMap<_, _> = url.query_pairs().collect();
    assert_eq!(params["gsrsearch"], "café & canal filetype:bitmap");
    assert_eq!(params["gsroffset"], "8");
    assert_eq!(params["gsrnamespace"], "6");
    assert!(request.contains(USER_AGENT));
    assert!(!request.to_lowercase().contains("cookie:"));
    assert!(!request.to_lowercase().contains("authorization:"));
}
#[test]
fn failed_cancelled_oversized_and_invalid_searches_do_not_produce_results() {
    for (status, body, expected) in [
        ("429 Too Many Requests", b"".to_vec(), "busy"),
        ("302 Found", b"".to_vec(), "HTTP 302"),
        ("200 OK", vec![b' '; MAX_RESPONSE + 1], "too large"),
    ] {
        let (endpoint, worker) = server(status, &body);
        let error = search_at(&endpoint, "test", None, &AtomicBool::new(false)).unwrap_err();
        worker.join().unwrap();
        assert!(error.contains(expected), "{error}");
    }
    for query in ["", "\n", "bad\u{1}query", &"x".repeat(201)] {
        assert!(request_url(API, query, None).is_err());
    }
    assert_eq!(
        search_at("http://127.0.0.1:1/", "test", None, &AtomicBool::new(true)).unwrap_err(),
        "Image search cancelled"
    );
}

#[test]
fn metadata_text_decodes_names_without_rendering_html_or_unbounded_content() {
    assert_eq!(
        plain_text("<a href='javascript:bad'>Zo&#235; &amp; Jos&#xE9;</a>"),
        "Zoë & José"
    );
    assert_eq!(plain_text("&amp;lt; &unknown;"), "&lt; &unknown;");
    assert_eq!(plain_text(&"x".repeat(40_000)).len(), 4000);
    assert!(thumbnail("http://127.0.0.1:1/private", &AtomicBool::new(false)).is_err());
}

#[test]
#[ignore = "explicit public-provider smoke check; requires Wikimedia network"]
fn commons_provider_search_thumbnail_and_full_preview_smoke() {
    let cancel = AtomicBool::new(false);
    let results = search("intitle:\"Example.jpg\"", None, &cancel).unwrap();
    assert!(!results.hits.is_empty());
    let hit = results
        .hits
        .iter()
        .find(|hit| hit.thumbnail.is_some())
        .expect("provider supplies a supported thumbnail");
    let thumb = thumbnail(hit.thumbnail.as_ref().unwrap(), &cancel).unwrap();
    assert!(thumb.pixels.size[0] > 0 && thumb.pixels.size[1] > 0);
    let preview = crate::images::web::fetch(&hit.address, &cancel).unwrap();
    assert!(!preview.image.bytes.is_empty());
    assert!(preview.original.pixels.size[0] > 0 && preview.original.pixels.size[1] > 0);
    assert!(!hit.attribution.license.is_empty());
    assert!(
        hit.attribution
            .source_url
            .starts_with("https://commons.wikimedia.org/")
    );
    println!(
        "Provider returned {} supported results; thumbnail/full preview decoded; source/licence present",
        results.hits.len()
    );
}
