// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Bounded, cancellable downloads. No project state is changed here.
use super::{Decoded, MAX_BYTES, Prepared, decoded_styled};
use std::{
    io::Read,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};
use url::Url;

pub struct Preview {
    pub image: Prepared,
    pub original: Decoded,
    pub sketch: Decoded,
}
fn validate(url: &Url) -> Result<(), String> {
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err("Enter an HTTP or HTTPS image address".into());
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err("Image addresses containing login details are not supported".into());
    }
    Ok(())
}
fn cancelled(cancel: &AtomicBool) -> Result<(), String> {
    if cancel.load(Ordering::Relaxed) {
        Err("Download cancelled".into())
    } else {
        Ok(())
    }
}
pub fn fetch(address: &str, cancel: &AtomicBool) -> Result<Preview, String> {
    fetch_with_timeout(address, cancel, Duration::from_secs(30))
}
fn fetch_with_timeout(
    address: &str,
    cancel: &AtomicBool,
    timeout: Duration,
) -> Result<Preview, String> {
    let (name, bytes) = download(address, cancel, timeout, MAX_BYTES, false)?;
    let image = Prepared::from_bytes(&name, &bytes)?;
    let original = decoded_styled(&image.bytes, 0, false)?;
    cancelled(cancel)?;
    let sketch = decoded_styled(&image.bytes, 0, true)?;
    cancelled(cancel)?;
    Ok(Preview {
        image,
        original,
        sketch,
    })
}
/// Search-result thumbnails have tighter time, byte and decoded-pixel limits.
pub(crate) fn thumbnail(address: &str, cancel: &AtomicBool) -> Result<Decoded, String> {
    let (_, bytes) = download(
        address,
        cancel,
        Duration::from_secs(8),
        2 * 1024 * 1024,
        true,
    )?;
    decode_thumbnail(&bytes, cancel)
}
fn decode_thumbnail(bytes: &[u8], cancel: &AtomicBool) -> Result<Decoded, String> {
    let (width, height) = super::dimensions(bytes)?;
    if u64::from(width) * u64::from(height) > 2 * 1024 * 1024 {
        return Err("Search thumbnail is too large".into());
    }
    cancelled(cancel)?;
    super::decoded(bytes)
}
fn download(
    address: &str,
    cancel: &AtomicBool,
    timeout: Duration,
    limit: usize,
    commons_only: bool,
) -> Result<(String, Vec<u8>), String> {
    let mut url = Url::parse(address.trim()).map_err(|_| "Enter an HTTP or HTTPS image address")?;
    validate(&url)?;
    url.set_fragment(None);
    let start = Instant::now();
    for redirect in 0..=5 {
        cancelled(cancel)?;
        if commons_only
            && (url.scheme() != "https" || url.host_str() != Some("upload.wikimedia.org"))
        {
            return Err("The search thumbnail has an unsupported address".into());
        }
        let remaining = timeout
            .checked_sub(start.elapsed())
            .filter(|d| !d.is_zero())
            .ok_or("The image download timed out")?;
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .max_redirects(0)
            .http_status_as_error(false)
            .timeout_global(Some(remaining))
            .build()
            .into();
        let mut response = agent
            .get(url.as_str())
            .header(
                "User-Agent",
                concat!(
                    "Wireframe/",
                    env!("CARGO_PKG_VERSION"),
                    " (https://github.com/samihsoylu/wireframe)"
                ),
            )
            .header("Accept", "image/png,image/jpeg,image/gif")
            .call()
            .map_err(|_| "Cannot download the image. Check the address and your connection.")?;
        cancelled(cancel)?;
        if response.status().is_redirection() {
            if redirect == 5 {
                return Err("The image address redirects too many times".into());
            }
            let location = response
                .headers()
                .get("location")
                .and_then(|v| v.to_str().ok())
                .ok_or("The image redirect has no valid address")?;
            let mut next = url
                .join(location)
                .map_err(|_| "The image redirect has no valid address")?;
            validate(&next)?;
            if url.scheme() == "https" && next.scheme() != "https" {
                return Err("The image redirects to an insecure connection".into());
            }
            next.set_fragment(None);
            url = next;
            continue;
        }
        if !response.status().is_success() {
            return Err(format!(
                "The image server returned HTTP {}",
                response.status().as_u16()
            ));
        }
        if response
            .headers()
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok())
            .is_some_and(|n| n > limit as u64)
        {
            return Err(format!(
                "This image exceeds the {} MB limit",
                limit / (1024 * 1024)
            ));
        }
        let mut reader = response.body_mut().as_reader();
        let mut bytes = Vec::new();
        let mut chunk = [0u8; 16 * 1024];
        loop {
            cancelled(cancel)?;
            let count = reader
                .read(&mut chunk)
                .map_err(|_| "The image download was interrupted")?;
            if count == 0 {
                break;
            }
            if bytes.len() + count > limit {
                return Err(format!(
                    "This image exceeds the {} MB limit",
                    limit / (1024 * 1024)
                ));
            }
            bytes.extend_from_slice(&chunk[..count]);
        }
        cancelled(cancel)?;
        let name = url
            .path_segments()
            .and_then(|mut p| p.next_back())
            .filter(|s| !s.is_empty())
            .unwrap_or("Web image");
        return Ok((name.to_owned(), bytes));
    }
    unreachable!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Cursor, Write},
        net::TcpListener,
        thread,
    };
    fn png() -> Vec<u8> {
        let mut bytes = Cursor::new(Vec::new());
        image::RgbaImage::from_pixel(12, 8, image::Rgba([180, 10, 30, 128]))
            .write_to(&mut bytes, image::ImageFormat::Png)
            .unwrap();
        bytes.into_inner()
    }
    fn server(responses: Vec<Vec<u8>>) -> (String, thread::JoinHandle<Vec<String>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = format!("http://{}", listener.local_addr().unwrap());
        let worker = thread::spawn(move || {
            let mut requests = Vec::new();
            for response in responses {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(3)))
                    .unwrap();
                let mut request = Vec::new();
                while !request.ends_with(b"\r\n\r\n") {
                    let mut byte = [0];
                    stream.read_exact(&mut byte).unwrap();
                    request.push(byte[0]);
                }
                requests.push(String::from_utf8(request).unwrap());
                let _ = stream.write_all(&response);
            }
            requests
        });
        (address, worker)
    }
    fn response(body: &[u8]) -> Vec<u8> {
        let mut out = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        )
        .into_bytes();
        out.extend_from_slice(body);
        out
    }
    #[test]
    fn thumbnails_have_an_independent_decoded_pixel_cap() {
        assert_eq!(
            decode_thumbnail(&png(), &AtomicBool::new(false))
                .unwrap()
                .pixels
                .size,
            [12, 8]
        );
        let mut bytes = Cursor::new(Vec::new());
        image::RgbaImage::from_pixel(2048, 1025, image::Rgba([255, 255, 255, 255]))
            .write_to(&mut bytes, image::ImageFormat::Png)
            .unwrap();
        assert!(
            decode_thumbnail(&bytes.into_inner(), &AtomicBool::new(false))
                .unwrap_err()
                .contains("thumbnail is too large")
        );
        assert!(decode_thumbnail(&png(), &AtomicBool::new(true)).is_err());
    }
    #[test]
    fn relative_redirect_fetches_bounded_preview_without_sending_fragment_or_cookies() {
        let (url, worker) = server(vec![
            b"HTTP/1.1 302 Found\r\nLocation: /picture.png?version=2\r\nContent-Length: 0\r\n\r\n"
                .to_vec(),
            response(&png()),
        ]);
        let preview = fetch(&(url + "/start#private-fragment"), &AtomicBool::new(false)).unwrap();
        assert_eq!(preview.image.name, "picture.png");
        assert_eq!(preview.original.pixels.size, [12, 8]);
        assert_eq!(preview.sketch.pixels.size, [16, 12]);
        assert_ne!(preview.original.pixels.pixels, preview.sketch.pixels.pixels);
        let requests = worker.join().unwrap();
        assert!(requests[0].starts_with("GET /start HTTP/1.1"));
        assert!(requests[1].starts_with("GET /picture.png?version=2 HTTP/1.1"));
        assert!(
            !requests
                .iter()
                .any(|r| r.to_lowercase().contains("cookie:") || r.contains("private-fragment"))
        );
    }
    #[test]
    fn invalid_scheme_credentials_and_pre_cancel_never_connect() {
        for url in [
            "file:///tmp/picture.png",
            "data:image/png,test",
            "ftp://example.com/image",
            "http://user:password@example.com/image",
            "not a url",
        ] {
            assert!(fetch(url, &AtomicBool::new(false)).is_err());
        }
        assert_eq!(
            fetch("http://127.0.0.1:1/image", &AtomicBool::new(true))
                .err()
                .unwrap(),
            "Download cancelled"
        );
    }
    #[test]
    fn rejects_status_non_image_size_and_truncated_body() {
        let cases = [
            (
                b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n".to_vec(),
                "HTTP 404",
            ),
            (response(b"<html>not an image</html>"), "PNG, JPEG or GIF"),
            (
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n",
                    MAX_BYTES + 1
                )
                .into_bytes(),
                "32 MB",
            ),
            (
                b"HTTP/1.1 200 OK\r\nContent-Length: 100\r\n\r\nshort".to_vec(),
                "interrupted",
            ),
        ];
        for (reply, message) in cases {
            let (url, worker) = server(vec![reply]);
            let error = fetch(&url, &AtomicBool::new(false)).err().unwrap();
            worker.join().unwrap();
            assert!(error.contains(message), "{error}");
        }
    }
    #[test]
    fn redirects_are_validated_and_counted() {
        for location in [
            "file:///tmp/image.png",
            "http://user:secret@example.com/image",
        ] {
            let (url, worker) = server(vec![
                format!("HTTP/1.1 302 Found\r\nLocation: {location}\r\nContent-Length: 0\r\n\r\n")
                    .into_bytes(),
            ]);
            assert!(fetch(&url, &AtomicBool::new(false)).is_err());
            worker.join().unwrap();
        }
        let (url, worker) = server(
            vec![b"HTTP/1.1 302 Found\r\nLocation: /loop\r\nContent-Length: 0\r\n\r\n".to_vec(); 6],
        );
        assert!(
            fetch(&url, &AtomicBool::new(false))
                .err()
                .unwrap()
                .contains("too many")
        );
        assert_eq!(worker.join().unwrap().len(), 6);
    }
    #[test]
    fn body_without_length_is_still_bounded() {
        let mut body = b"HTTP/1.1 200 OK\r\nConnection: close\r\n\r\n".to_vec();
        body.resize(body.len() + MAX_BYTES + 1, 0);
        let (url, worker) = server(vec![body]);
        assert!(
            fetch(&url, &AtomicBool::new(false))
                .err()
                .unwrap()
                .contains("32 MB")
        );
        worker.join().unwrap();
    }
    #[test]
    fn slow_server_is_bounded_by_deadline() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let worker = thread::spawn(move || {
            let (_stream, _) = listener.accept().unwrap();
            thread::sleep(Duration::from_millis(250));
        });
        assert!(
            fetch_with_timeout(&url, &AtomicBool::new(false), Duration::from_millis(40)).is_err()
        );
        worker.join().unwrap();
    }
}
