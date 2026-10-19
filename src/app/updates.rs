// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Optional bundle-local Sparkle integration. Bare Cargo builds stay offline.
//! Sparkle owns update UI, preferences, signature validation and installation.
use base64::{Engine as _, engine::general_purpose::STANDARD};
use objc2::{
    DefinedClass, MainThreadOnly, define_class, msg_send,
    rc::{Allocated, Retained},
    runtime::{AnyClass, AnyObject},
    sel,
};
use objc2_app_kit::{NSApplication, NSMenu, NSMenuItem};
use objc2_foundation::{
    MainThreadMarker, NSBundle, NSError, NSNumber, NSObject, NSObjectProtocol, NSString,
};

struct Ivars {
    updater: Retained<AnyObject>,
}
define_class!(
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = Ivars]
    struct UpdatePreferences;
    unsafe impl NSObjectProtocol for UpdatePreferences {}
    impl UpdatePreferences {
        #[unsafe(method(validateMenuItem:))]
        fn validate(&self, item: &NSMenuItem) -> bool {
            item.setState(isize::from(self.automatic()));
            true
        }
        #[unsafe(method(toggleAutomaticChecks:))]
        fn toggle(&self, item: &NSMenuItem) {
            let enabled = !self.automatic();
            // SAFETY: Pinned SPUUpdater BOOL setter, on the main thread.
            unsafe { let _: () = msg_send![&*self.ivars().updater, setAutomaticallyChecksForUpdates: enabled]; }
            item.setState(isize::from(enabled));
        }
    }
);
impl UpdatePreferences {
    fn automatic(&self) -> bool {
        // SAFETY: Pinned SPUUpdater BOOL property, on the main thread.
        unsafe { msg_send![&*self.ivars().updater, automaticallyChecksForUpdates] }
    }
    fn new(mtm: MainThreadMarker, updater: Retained<AnyObject>) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(Ivars { updater });
        // SAFETY: Initialize the allocated NSObject subclass.
        unsafe { msg_send![super(this), init] }
    }
}

pub(super) struct Updates {
    // AppKit does not retain menu targets. Detach before releasing either target.
    menu: Retained<NSMenu>,
    items: Vec<Retained<NSMenuItem>>,
    _preferences: Retained<UpdatePreferences>,
    _controller: Retained<AnyObject>,
    _framework: Retained<NSBundle>,
}
impl Updates {
    pub(super) fn start(termination_ready: bool) -> Result<Option<Self>, String> {
        let host = NSBundle::mainBundle();
        if !info_bool(&host, "WireframeEnableUpdater").unwrap_or(false) {
            return Ok(None);
        }
        if !termination_ready {
            return Err("The application cannot safely close pending edits.".into());
        }
        let mtm = MainThreadMarker::new().ok_or("Updates require the main thread.")?;
        validate_configuration(
            &info_string(&host, "SUFeedURL").unwrap_or_default(),
            &info_string(&host, "SUPublicEDKey").unwrap_or_default(),
        )?;
        if info_bool(&host, "SUAllowsAutomaticUpdates") != Some(false) {
            return Err("This build must require approval before installing updates.".into());
        }
        let menu = NSApplication::sharedApplication(mtm)
            .mainMenu()
            .and_then(|root| root.itemAtIndex(0))
            .and_then(|item| item.submenu())
            .ok_or("The application menu is unavailable.")?;
        let root = std::path::PathBuf::from(host.bundlePath().to_string());
        let framework_path = root.join("Contents/Frameworks/Sparkle.framework");
        let canonical = framework_path
            .canonicalize()
            .map_err(|_| "The update framework is missing.")?;
        let root = root
            .canonicalize()
            .map_err(|_| "The application bundle is unavailable.")?;
        if !canonical.starts_with(root.join("Contents/Frameworks")) {
            return Err("The update framework must be inside the application.".into());
        }
        let framework = NSBundle::bundleWithPath(&NSString::from_str(&canonical.to_string_lossy()))
            .ok_or("The update framework is not a bundle.")?;
        if info_string(&framework, "CFBundleShortVersionString").as_deref() != Some("2.10.0") {
            return Err("The update framework version does not match this build.".into());
        }
        // SAFETY: Load only the bundled, pinned framework on the main thread.
        // Distribution tooling verifies its archive and signs the whole bundle.
        unsafe { framework.loadAndReturnError() }
            .map_err(|e| e.localizedDescription().to_string())?;
        let class = AnyClass::get(c"SPUStandardUpdaterController")
            .ok_or("The update controller is unavailable.")?;
        // SAFETY: Signatures follow Sparkle 2.10.0 headers. NSObject allocation
        // transfers ownership to init; retained results outlive their menu items.
        let controller: Retained<AnyObject> = unsafe {
            let allocated: Allocated<AnyObject> = msg_send![class, alloc];
            let result: Option<Retained<AnyObject>> = msg_send![allocated,
                initWithStartingUpdater: false,
                updaterDelegate: Option::<&AnyObject>::None,
                userDriverDelegate: Option::<&AnyObject>::None];
            result.ok_or("The update controller could not be created.")?
        };
        let updater: Retained<AnyObject> = unsafe { msg_send![&*controller, updater] };
        let mut error: Option<Retained<NSError>> = None;
        let started: bool = unsafe { msg_send![&*updater, startUpdater: &mut error] };
        if !started {
            return Err(error.map_or_else(
                || "The updater could not start.".into(),
                |e| e.localizedDescription().to_string(),
            ));
        }
        let preferences = UpdatePreferences::new(mtm, updater);
        let check = NSMenuItem::new(mtm);
        check.setTitle(&NSString::from_str("Check for Updates…"));
        let automatic = NSMenuItem::new(mtm);
        automatic.setTitle(&NSString::from_str("Automatically Check for Updates"));
        // SAFETY: Both targets implement these actions and are retained by Self.
        unsafe {
            check.setTarget(Some(&controller));
            check.setAction(Some(sel!(checkForUpdates:)));
            automatic.setTarget(Some(&preferences));
            automatic.setAction(Some(sel!(toggleAutomaticChecks:)));
        }
        automatic.setState(isize::from(preferences.automatic()));
        let items = vec![check, automatic, NSMenuItem::separatorItem(mtm)];
        for (index, item) in items.iter().enumerate() {
            menu.insertItem_atIndex(item, index as isize + 1);
        }
        Ok(Some(Self {
            menu,
            items,
            _preferences: preferences,
            _controller: controller,
            _framework: framework,
        }))
    }
}
impl Drop for Updates {
    fn drop(&mut self) {
        for item in &self.items {
            // SAFETY: Clear unretained targets before their owners are dropped.
            unsafe {
                item.setTarget(None);
                item.setAction(None);
            }
            self.menu.removeItem(item);
        }
    }
}
fn info_string(bundle: &NSBundle, key: &str) -> Option<String> {
    bundle
        .objectForInfoDictionaryKey(&NSString::from_str(key))?
        .downcast_ref::<NSString>()
        .map(ToString::to_string)
}
fn info_bool(bundle: &NSBundle, key: &str) -> Option<bool> {
    bundle
        .objectForInfoDictionaryKey(&NSString::from_str(key))?
        .downcast_ref::<NSNumber>()
        .map(|number| number.boolValue())
}
fn validate_configuration(feed: &str, public_key: &str) -> Result<(), String> {
    let url = url::Url::parse(feed).map_err(|_| "The update feed URL is invalid.")?;
    if url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
    {
        return Err(
            "The update feed must be an HTTPS URL without credentials or a fragment.".into(),
        );
    }
    if STANDARD
        .decode(public_key)
        .ok()
        .filter(|bytes| bytes.len() == 32)
        .is_none()
    {
        return Err("The update signing public key is invalid.".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn update_configuration_requires_https_and_a_complete_public_key() {
        let key = STANDARD.encode([7u8; 32]);
        assert!(validate_configuration("https://example.org/appcast.xml", &key).is_ok());
        for feed in [
            "http://example.org/appcast.xml",
            "file:///tmp/appcast.xml",
            "https://user:secret@example.org/feed",
            "https://example.org/feed#fragment",
            "",
        ] {
            assert!(validate_configuration(feed, &key).is_err(), "{feed}");
        }
        for bad in [
            "",
            "not base64",
            &STANDARD.encode([7u8; 31]),
            &STANDARD.encode([7u8; 33]),
        ] {
            assert!(validate_configuration("https://example.org/feed", bad).is_err());
        }
    }
    #[test]
    fn ordinary_cargo_build_does_not_start_an_updater() {
        assert!(matches!(Updates::start(false), Ok(None)));
    }
}
