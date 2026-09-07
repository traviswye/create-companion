//! Which application is in front (bundle identifier from NSWorkspace) and,
//! when a profile asks, the title of its focused window through the
//! Accessibility API (needs the Accessibility permission).

use companion_core::profile::AppIdentity;
use objc2_app_kit::NSWorkspace;
use objc2_core_foundation::{CFRetained, CFString};
use std::ffi::c_void;
use std::ptr::NonNull;

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXUIElementCreateApplication(pid: libc::pid_t) -> *mut c_void;
    fn AXUIElementCopyAttributeValue(
        element: *const c_void,
        attribute: *const CFString,
        value: *mut *const c_void,
    ) -> i32;
    fn CFRelease(cf: *const c_void);
}

fn frontmost() -> Option<(String, libc::pid_t)> {
    let ws = NSWorkspace::sharedWorkspace();
    let app = ws.frontmostApplication()?;
    let bundle = app.bundleIdentifier()?.to_string();
    Some((bundle, app.processIdentifier()))
}

/// The frontmost application's bundle identifier.
pub fn current() -> Option<AppIdentity> {
    frontmost().map(|(bundle, _)| AppIdentity::MacBundle(bundle))
}

/// The title of the frontmost application's focused window, via AX.
/// `None` without the Accessibility permission or when the app has no
/// focused window.
pub fn current_title() -> Option<String> {
    let (_, pid) = frontmost()?;
    // SAFETY: plain C calls on values we own; every CF object we receive is
    // released below.
    unsafe {
        let app = AXUIElementCreateApplication(pid);
        if app.is_null() {
            return None;
        }
        let focused_attr = CFString::from_static_str("AXFocusedWindow");
        let title_attr = CFString::from_static_str("AXTitle");
        let mut window: *const c_void = std::ptr::null();
        let err = AXUIElementCopyAttributeValue(app, &*focused_attr, &mut window);
        let title = if err == 0 && !window.is_null() {
            let mut value: *const c_void = std::ptr::null();
            let err = AXUIElementCopyAttributeValue(window, &*title_attr, &mut value);
            let t = if err == 0 && !value.is_null() {
                let s: CFRetained<CFString> =
                    CFRetained::from_raw(NonNull::new_unchecked(value as *mut CFString));
                Some(s.to_string())
            } else {
                None
            };
            CFRelease(window);
            t
        } else {
            None
        };
        CFRelease(app);
        title
    }
}
