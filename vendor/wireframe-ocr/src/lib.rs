// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Bounded, offline OCR. No project/history API or screenshot/model files needed.
use image::{ImageFormat, ImageReader};
use std::{
    ffi::{CStr, c_char, c_int, c_void},
    io::Cursor,
    ptr::{self, NonNull},
    sync::atomic::{AtomicBool, Ordering},
};

const MODEL: &[u8] = include_bytes!("../models/eng.traineddata");
pub const MAX_BYTES: usize = 32 * 1024 * 1024;
pub const MAX_PIXELS: u64 = 16 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recognition {
    /// Reading order and line breaks supplied by Tesseract; no synthetic text.
    pub text: String,
    /// Engine estimate, 0–100. It is not a guarantee of accuracy.
    pub confidence: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Cancelled,
    ImageTooLarge,
    InvalidImage,
    Engine,
    Recognition,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Cancelled => "Text extraction cancelled",
            Self::ImageTooLarge => "Choose a screenshot smaller than 32 MB and 16 megapixels",
            Self::InvalidImage => "Choose a PNG, JPEG or GIF screenshot that opens correctly",
            Self::Engine => "Couldn't start text recognition. Close this preview and try again.",
            Self::Recognition => {
                "Couldn't finish text recognition. Try a smaller, clearer screenshot."
            }
        })
    }
}
impl std::error::Error for Error {}

fn check_cancel(cancel: &AtomicBool) -> Result<(), Error> {
    if cancel.load(Ordering::Relaxed) {
        Err(Error::Cancelled)
    } else {
        Ok(())
    }
}

fn reader(bytes: &[u8]) -> Result<ImageReader<Cursor<&[u8]>>, Error> {
    if bytes.len() > MAX_BYTES {
        return Err(Error::ImageTooLarge);
    }
    let format = image::guess_format(bytes).map_err(|_| Error::InvalidImage)?;
    if !matches!(
        format,
        ImageFormat::Png | ImageFormat::Jpeg | ImageFormat::Gif
    ) {
        return Err(Error::InvalidImage);
    }
    let mut reader = ImageReader::with_format(Cursor::new(bytes), format);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(32768);
    limits.max_image_height = Some(32768);
    limits.max_alloc = Some(MAX_PIXELS * 4);
    reader.limits(limits);
    Ok(reader)
}

/// Recognize printed English text in a PNG, JPEG or first GIF frame.
///
/// Call from a worker thread. The model is embedded; no screenshot/model files are needed and
/// no screenshot bytes leave this process. Cancellation is checked around decode,
/// initialization and through Tesseract's recognition monitor. The monitor uses a
/// cooperative 30-second deadline. Page segmentation can run before its checks;
/// this is not a total wall-clock timeout. Keep UI worker ownership until return.
pub fn recognize(bytes: &[u8], cancel: &AtomicBool) -> Result<Recognition, Error> {
    check_cancel(cancel)?;
    let (width, height) = reader(bytes)?
        .into_dimensions()
        .map_err(|_| Error::InvalidImage)?;
    if width == 0 || height == 0 || u64::from(width) * u64::from(height) > MAX_PIXELS {
        return Err(Error::ImageTooLarge);
    }
    let rgba = reader(bytes)?
        .decode()
        .map_err(|_| Error::InvalidImage)?
        .to_rgba8();
    check_cancel(cancel)?;
    // Flatten transparency onto white. Hidden RGB under transparent pixels must
    // not become recognized text, and alpha must not be interpreted as ink.
    let mut rgb = Vec::with_capacity(width as usize * height as usize * 3);
    for pixel in rgba.pixels() {
        let alpha = u16::from(pixel[3]);
        for channel in &pixel.0[..3] {
            rgb.push(((u16::from(*channel) * alpha + 255 * (255 - alpha) + 127) / 255) as u8);
        }
    }
    drop(rgba);
    recognize_rgb(&rgb, width, height, cancel)
}

fn recognize_rgb(
    rgb: &[u8],
    width: u32,
    height: u32,
    cancel: &AtomicBool,
) -> Result<Recognition, Error> {
    check_cancel(cancel)?;
    // SAFETY: the non-null handle is owned here and destroyed by Engine. Every
    // API call uses this one thread and all buffers outlive their native use.
    let engine = Engine(NonNull::new(unsafe { TessBaseAPICreate() }).ok_or(Error::Engine)?);
    unsafe {
        // Init5 consumes an in-memory traineddata file; OEM_LSTM_ONLY = 1.
        if TessBaseAPIInit5(
            engine.0.as_ptr(),
            MODEL.as_ptr().cast(),
            MODEL.len() as c_int,
            c"eng".as_ptr(),
            1,
            ptr::null_mut(),
            0,
            ptr::null_mut(),
            ptr::null_mut(),
            0,
            0,
        ) != 0
        {
            return Err(Error::Engine);
        }
        check_cancel(cancel)?;
        // Automatic layout preserves reading order and recognizes text inside UI
        // buttons. No OSD or script model lookup is needed (PSM_AUTO = 3).
        TessBaseAPISetPageSegMode(engine.0.as_ptr(), 3);
        TessBaseAPISetVariable(
            engine.0.as_ptr(),
            c"user_defined_dpi".as_ptr(),
            c"150".as_ptr(),
        );
        TessBaseAPISetImage(
            engine.0.as_ptr(),
            rgb.as_ptr(),
            width as c_int,
            height as c_int,
            3,
            (width * 3) as c_int,
        );
        let monitor = Monitor(NonNull::new(TessMonitorCreate()).ok_or(Error::Engine)?);
        TessMonitorSetCancelFunc(monitor.0.as_ptr(), Some(is_cancelled));
        TessMonitorSetCancelThis(
            monitor.0.as_ptr(),
            (cancel as *const AtomicBool).cast_mut().cast(),
        );
        TessMonitorSetDeadlineMSecs(monitor.0.as_ptr(), 30_000);
        let status = TessBaseAPIRecognize(engine.0.as_ptr(), monitor.0.as_ptr());
        check_cancel(cancel)?;
        if status != 0 {
            return Err(Error::Recognition);
        }
        let text = Text(
            NonNull::new(TessBaseAPIGetUTF8Text(engine.0.as_ptr())).ok_or(Error::Recognition)?,
        );
        let recognized = CStr::from_ptr(text.0.as_ptr())
            .to_str()
            .map_err(|_| Error::Recognition)?;
        let result = Recognition {
            text: recognized.trim().to_owned(),
            confidence: TessBaseAPIMeanTextConf(engine.0.as_ptr()).clamp(0, 100) as u8,
        };
        check_cancel(cancel)?;
        Ok(result)
    }
}

// Native pointer types are deliberately opaque. No native object is shared or
// sent to another thread. The callback only borrows the caller's AtomicBool.
struct Engine(NonNull<c_void>);
impl Drop for Engine {
    fn drop(&mut self) {
        unsafe {
            TessBaseAPIDelete(self.0.as_ptr());
        }
    }
}
struct Monitor(NonNull<c_void>);
impl Drop for Monitor {
    fn drop(&mut self) {
        unsafe {
            TessMonitorDelete(self.0.as_ptr());
        }
    }
}
struct Text(NonNull<c_char>);
impl Drop for Text {
    fn drop(&mut self) {
        unsafe {
            TessDeleteText(self.0.as_ptr());
        }
    }
}
unsafe extern "C" fn is_cancelled(cancel: *mut c_void, _words: c_int) -> bool {
    // SAFETY: recognize_rgb supplies a valid AtomicBool until Recognize returns;
    // no callback is retained or invoked after the monitor has been dropped.
    unsafe { (&*cancel.cast::<AtomicBool>()).load(Ordering::Relaxed) }
}

unsafe extern "C" {
    fn TessBaseAPICreate() -> *mut c_void;
    fn TessBaseAPIDelete(handle: *mut c_void);
    fn TessBaseAPIInit5(
        handle: *mut c_void,
        data: *const c_char,
        data_size: c_int,
        language: *const c_char,
        mode: c_int,
        configs: *mut *mut c_char,
        configs_size: c_int,
        vars: *mut *mut c_char,
        values: *mut *mut c_char,
        vars_size: usize,
        only_non_debug: c_int,
    ) -> c_int;
    fn TessBaseAPISetPageSegMode(handle: *mut c_void, mode: c_int);
    fn TessBaseAPISetVariable(
        handle: *mut c_void,
        name: *const c_char,
        value: *const c_char,
    ) -> c_int;
    fn TessBaseAPISetImage(
        handle: *mut c_void,
        image: *const u8,
        width: c_int,
        height: c_int,
        bpp: c_int,
        stride: c_int,
    );
    fn TessBaseAPIRecognize(handle: *mut c_void, monitor: *mut c_void) -> c_int;
    fn TessBaseAPIGetUTF8Text(handle: *mut c_void) -> *mut c_char;
    fn TessBaseAPIMeanTextConf(handle: *mut c_void) -> c_int;
    fn TessDeleteText(text: *mut c_char);
    fn TessMonitorCreate() -> *mut c_void;
    fn TessMonitorDelete(monitor: *mut c_void);
    fn TessMonitorSetCancelFunc(
        monitor: *mut c_void,
        callback: Option<unsafe extern "C" fn(*mut c_void, c_int) -> bool>,
    );
    fn TessMonitorSetCancelThis(monitor: *mut c_void, cancel: *mut c_void);
    fn TessMonitorSetDeadlineMSecs(monitor: *mut c_void, deadline: c_int);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cancellation_precedes_invalid_data() {
        assert_eq!(
            recognize(b"", &AtomicBool::new(true)),
            Err(Error::Cancelled)
        );
    }
    #[test]
    fn invalid_and_oversized_data_are_rejected() {
        let cancel = AtomicBool::new(false);
        assert_eq!(
            recognize(b"not an image", &cancel),
            Err(Error::InvalidImage)
        );
        assert_eq!(
            recognize(&vec![0; MAX_BYTES + 1], &cancel),
            Err(Error::ImageTooLarge)
        );
    }
    #[test]
    fn oversized_dimensions_are_rejected_before_decode() {
        let bytes = include_bytes!("../tests/fixtures/oversized-header.png");
        assert_eq!(
            recognize(bytes, &AtomicBool::new(false)),
            Err(Error::ImageTooLarge)
        );
    }
}
