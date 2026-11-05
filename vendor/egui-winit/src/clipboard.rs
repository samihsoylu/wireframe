use raw_window_handle::RawDisplayHandle;

/// Handles interfacing with the OS clipboard.
///
/// If the "clipboard" feature is off, or we cannot connect to the OS clipboard,
/// then a fallback clipboard that just works within the same app is used instead.
pub struct Clipboard {
    #[cfg(all(
        not(any(target_os = "android", target_os = "ios")),
        feature = "arboard",
    ))]
    arboard: Option<arboard::Clipboard>,

    #[cfg(all(
        any(
            target_os = "linux",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd"
        ),
        feature = "smithay-clipboard"
    ))]
    smithay: Option<smithay_clipboard::Clipboard>,

    /// Fallback manual clipboard.
    clipboard: String,
}

fn connect_if_allowed<T>(local_only: bool, factory: impl FnOnce() -> Option<T>) -> Option<T> {
    if local_only { None } else { factory() }
}

impl Clipboard {
    /// Construct a new instance
    pub fn new(raw_display_handle: Option<RawDisplayHandle>) -> Self {
        Self::with_policy(raw_display_handle, false)
    }

    pub fn with_policy(_raw_display_handle: Option<RawDisplayHandle>, local_only: bool) -> Self {
        Self {
            #[cfg(all(
                not(any(target_os = "android", target_os = "ios")),
                feature = "arboard",
            ))]
            arboard: connect_if_allowed(local_only, init_arboard),

            #[cfg(all(
                any(
                    target_os = "linux",
                    target_os = "dragonfly",
                    target_os = "freebsd",
                    target_os = "netbsd",
                    target_os = "openbsd"
                ),
                feature = "smithay-clipboard"
            ))]
            smithay: connect_if_allowed(local_only, || init_smithay_clipboard(_raw_display_handle)),

            clipboard: Default::default(),
        }
    }

    pub fn get(&mut self) -> Option<String> {
        #[cfg(all(
            any(
                target_os = "linux",
                target_os = "dragonfly",
                target_os = "freebsd",
                target_os = "netbsd",
                target_os = "openbsd"
            ),
            feature = "smithay-clipboard"
        ))]
        if let Some(clipboard) = &mut self.smithay {
            match clipboard.load() {
                Ok(text) => return Some(text),
                Err(err) => {
                    log::error!("smithay paste error: {err}");
                }
            }
        }

        #[cfg(all(
            not(any(target_os = "android", target_os = "ios")),
            feature = "arboard",
        ))]
        if let Some(clipboard) = &mut self.arboard {
            return match clipboard.get_text() {
                Ok(text) => Some(text),
                Err(err) => {
                    log::error!("arboard paste error: {err}");
                    None
                }
            };
        }

        Some(self.clipboard.clone())
    }

    pub fn set_text(&mut self, text: String) {
        #[cfg(all(
            any(
                target_os = "linux",
                target_os = "dragonfly",
                target_os = "freebsd",
                target_os = "netbsd",
                target_os = "openbsd"
            ),
            feature = "smithay-clipboard"
        ))]
        if let Some(clipboard) = &mut self.smithay {
            clipboard.store(text);
            return;
        }

        #[cfg(all(
            not(any(target_os = "android", target_os = "ios")),
            feature = "arboard",
        ))]
        if let Some(clipboard) = &mut self.arboard {
            if let Err(err) = clipboard.set_text(text) {
                log::error!("arboard copy/cut error: {err}");
            }
            return;
        }

        self.clipboard = text;
    }

    // Wireframe addition, Copyright 2026 Wireframe contributors.
    // SPDX-License-Identifier: GPL-3.0-only (this method)
    /// A destructive application action requires system publication, not the
    /// in-process fallback. Read-back also detects a failed asynchronous store.
    pub fn set_text_checked(&mut self, text: String) -> Result<(), String> {
        #[cfg(all(
            any(
                target_os = "linux",
                target_os = "dragonfly",
                target_os = "freebsd",
                target_os = "netbsd",
                target_os = "openbsd"
            ),
            feature = "smithay-clipboard"
        ))]
        if let Some(clipboard) = &mut self.smithay {
            clipboard.store(text.clone());
            // Store is queued to the Wayland worker. The compositor's new
            // selection offer can arrive after the first read request. Allow
            // that offer to settle without publishing repeatedly or accepting
            // an old offer as success. Failed verification keeps Cut intact.
            for attempt in 0..8 {
                if clipboard.load().is_ok_and(|actual| actual == text) {
                    return Ok(());
                }
                if attempt < 7 {
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
            }
            return Err("The system clipboard could not be verified".into());
        }
        #[cfg(all(
            not(any(target_os = "android", target_os = "ios")),
            feature = "arboard"
        ))]
        if let Some(clipboard) = &mut self.arboard {
            clipboard
                .set_text(text.clone())
                .map_err(|_| "The system clipboard could not be written".to_owned())?;
            return match clipboard.get_text() {
                Ok(actual) if actual == text => Ok(()),
                _ => Err("The system clipboard could not be verified".into()),
            };
        }
        let _ = text;
        Err("The system clipboard is unavailable".into())
    }

    pub fn set_image(&mut self, image: &egui::ColorImage) {
        #[cfg(all(
            not(any(target_os = "android", target_os = "ios")),
            feature = "arboard",
        ))]
        if let Some(clipboard) = &mut self.arboard {
            if let Err(err) = clipboard.set_image(arboard::ImageData {
                width: image.width(),
                height: image.height(),
                bytes: std::borrow::Cow::Borrowed(bytemuck::cast_slice(&image.pixels)),
            }) {
                log::error!("arboard copy/cut error: {err}");
            }
            log::debug!("Copied image to clipboard");
            return;
        }

        log::error!(
            "Copying images is not supported. Enable the 'clipboard' feature of `egui-winit` to enable it."
        );
        _ = image;
    }
}

#[cfg(all(
    not(any(target_os = "android", target_os = "ios")),
    feature = "arboard",
))]
fn init_arboard() -> Option<arboard::Clipboard> {
    profiling::function_scope!();

    log::trace!("Initializing arboard clipboard…");
    match arboard::Clipboard::new() {
        Ok(clipboard) => Some(clipboard),
        Err(err) => {
            log::warn!("Failed to initialize arboard clipboard: {err}");
            None
        }
    }
}

#[cfg(all(
    any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    ),
    feature = "smithay-clipboard"
))]
fn init_smithay_clipboard(
    raw_display_handle: Option<RawDisplayHandle>,
) -> Option<smithay_clipboard::Clipboard> {
    #![expect(clippy::undocumented_unsafe_blocks)]

    profiling::function_scope!();

    if let Some(RawDisplayHandle::Wayland(display)) = raw_display_handle {
        log::trace!("Initializing smithay clipboard…");
        #[expect(unsafe_code)]
        Some(unsafe { smithay_clipboard::Clipboard::new(display.display.as_ptr()) })
    } else {
        #[cfg(feature = "wayland")]
        log::debug!("Cannot init smithay clipboard without a Wayland display handle");
        #[cfg(not(feature = "wayland"))]
        log::debug!(
            "Cannot init smithay clipboard: the 'wayland' feature of 'egui-winit' is not enabled"
        );
        None
    }
}

// Wireframe addition, Copyright 2026 Wireframe contributors.
// SPDX-License-Identifier: GPL-3.0-only (this test module)
#[cfg(test)]
mod publication_tests {
    use super::*;
    #[test]
    fn checked_publication_never_reports_in_process_fallback_as_system_success() {
        let mut clipboard = Clipboard {
            #[cfg(all(
                not(any(target_os = "android", target_os = "ios")),
                feature = "arboard"
            ))]
            arboard: None,
            #[cfg(all(
                any(
                    target_os = "linux",
                    target_os = "dragonfly",
                    target_os = "freebsd",
                    target_os = "netbsd",
                    target_os = "openbsd"
                ),
                feature = "smithay-clipboard"
            ))]
            smithay: None,
            clipboard: "previous".into(),
        };
        assert!(clipboard.set_text_checked("new".into()).is_err());
        assert_eq!(clipboard.get().as_deref(), Some("previous"));
    }
}

#[cfg(test)]
mod diagnostic_tests {
    #[test]
    fn local_policy_never_invokes_system_factory_and_normal_policy_does() {
        assert!(super::connect_if_allowed::<()>(true, || panic!("OS clipboard factory")).is_none());
        let mut calls = 0;
        assert_eq!(super::connect_if_allowed(false, || { calls += 1; Some(17) }), Some(17));
        assert_eq!(calls, 1);
    }

    use super::*;
    #[test] fn local_text_never_connects_system_and_checked_cut_stays_unavailable() {
        let mut first=Clipboard::with_policy(None,true);let mut second=Clipboard::with_policy(None,true);
        first.set_text("local\r\ntext".into());assert_eq!(first.get().as_deref(),Some("local\r\ntext"));assert_eq!(second.get().as_deref(),Some(""));
        assert!(first.set_text_checked("cut".into()).is_err());assert_eq!(first.get().as_deref(),Some("local\r\ntext"));
        #[cfg(feature="arboard")] assert!(first.arboard.is_none());
        #[cfg(all(target_os="linux",feature="smithay-clipboard"))] assert!(first.smithay.is_none());
    }
}
