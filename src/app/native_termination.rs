// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Route AppKit termination through the same draft/save/export gate as window close.
//! AppKit's modal termination loop cannot drive winit's default-mode UI callbacks,
//! so reject the initial request and resume it asynchronously after approval.
use eframe::egui;
use objc2::{
    ffi,
    runtime::{AnyClass, AnyObject, Imp, Sel},
    sel,
};
use objc2_app_kit::NSApplication;
use objc2_foundation::{MainThreadMarker, NSRunLoop};
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};

struct State {
    ctx: egui::Context,
    requested: Cell<bool>,
    pending: Cell<bool>,
    allowed: Cell<bool>,
}
impl State {
    fn request(&self) {
        if !self.pending.replace(true) {
            self.requested.set(true);
            self.ctx.request_repaint();
        }
    }
}
thread_local! {
    static OWNER: RefCell<Weak<State>> = const { RefCell::new(Weak::new()) };
}

extern "C-unwind" fn should_terminate(_: &AnyObject, _: Sel, _: &NSApplication) -> usize {
    let state = OWNER.with(|owner| owner.borrow().upgrade());
    if let Some(state) = state
        && !state.allowed.get()
    {
        state.request();
        return 0; // NSTerminateCancel: let the ordinary event loop show the save UI.
    }
    1 // NSTerminateNow after approval, or after the gate's owner has gone away.
}

pub(super) struct NativeTermination {
    state: Rc<State>,
    native: bool,
}
impl NativeTermination {
    pub(super) fn new(ctx: &egui::Context) -> Option<Self> {
        let mtm = MainThreadMarker::new()?;
        let application = NSApplication::sharedApplication(mtm);
        let delegate = application.delegate()?;
        let object: &AnyObject = (*delegate).as_ref();
        let class = object.class();
        // Extend only the known event-loop delegate. Never replace another
        // integration's termination handler or change the delegate's identity.
        if class.name() != c"WinitApplicationDelegate" {
            return None;
        }
        // SAFETY: The callback has the Objective-C ABI for NSUInteger(id, SEL, id).
        // All supported macOS targets are 64-bit; Q@:@ is its method encoding.
        let implementation: Imp = unsafe {
            std::mem::transmute::<extern "C-unwind" fn(&AnyObject, Sel, &NSApplication) -> usize, Imp>(
                should_terminate,
            )
        };
        if let Some(method) = class.instance_method(sel!(applicationShouldTerminate:)) {
            if !std::ptr::fn_addr_eq(method.implementation(), implementation) {
                return None;
            }
        } else {
            // SAFETY: Main-thread-only initialization, matching callback/encoding,
            // and the selector is absent. No existing implementation is replaced.
            let added = unsafe {
                ffi::class_addMethod(
                    class as *const AnyClass as *mut AnyClass,
                    sel!(applicationShouldTerminate:),
                    implementation,
                    c"Q@:@".as_ptr(),
                )
            };
            if !added.as_bool() {
                return None;
            }
        }
        if OWNER.with(|owner| owner.borrow().upgrade().is_some()) {
            return None;
        }
        let result = Self::with_state(ctx, true);
        OWNER.with(|owner| *owner.borrow_mut() = Rc::downgrade(&result.state));
        Some(result)
    }
    fn with_state(ctx: &egui::Context, native: bool) -> Self {
        Self {
            state: Rc::new(State {
                ctx: ctx.clone(),
                requested: Cell::new(false),
                pending: Cell::new(false),
                allowed: Cell::new(false),
            }),
            native,
        }
    }
    pub(super) fn take_request(&self) -> bool {
        self.state.requested.replace(false)
    }
    pub(super) fn cancel(&self) {
        self.state.pending.set(false);
        self.state.requested.set(false);
    }
    pub(super) fn approve(&self) -> bool {
        if !self.state.pending.replace(false) {
            return false;
        }
        self.state.allowed.set(true);
        if self.native {
            let resume = block2::RcBlock::new(|| {
                if let Some(mtm) = MainThreadMarker::new() {
                    NSApplication::sharedApplication(mtm).terminate(None);
                }
            });
            // SAFETY: This capture-free block is sendable. The main run loop
            // executes it after the editor's current borrowed UI callback returns.
            unsafe { NSRunLoop::mainRunLoop().performBlock(&resume) };
        }
        true
    }
    #[cfg(test)]
    pub(super) fn simulated(ctx: &egui::Context) -> Self {
        Self::with_state(ctx, false)
    }
    #[cfg(test)]
    pub(super) fn request_for_test(&self) {
        self.state.request();
    }
    #[cfg(test)]
    pub(super) fn approved_for_test(&self) -> bool {
        self.state.allowed.get()
    }
}
