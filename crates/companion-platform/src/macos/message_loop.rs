//! The main-thread event pump the tray needs on macOS, plus a [`Waker`] other
//! threads use to nudge it. The engine runs as an accessory app (no Dock
//! icon), pumping AppKit events itself so the tray menu works without a
//! full `NSApplication::run`.

use objc2::MainThreadMarker;
use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy, NSEventMask};
use objc2_core_foundation::{
    kCFRunLoopCommonModes, CFRunLoop, CFRunLoopSource, CFRunLoopSourceContext,
};
use objc2_foundation::{NSDate, NSDefaultRunLoopMode};
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

static QUIT: AtomicBool = AtomicBool::new(false);
static WAKE_SOURCE: AtomicUsize = AtomicUsize::new(0);
static MAIN_LOOP: AtomicUsize = AtomicUsize::new(0);

unsafe extern "C-unwind" fn perform(_info: *mut c_void) {
    // Nothing to do: signalling the source is enough to return from the
    // pump so `run` calls its handler.
}

/// Signals the main run loop. `wake` returns the pump to the handler;
/// `quit` ends [`run`].
#[derive(Debug, Clone, Copy)]
pub struct Waker;

impl Waker {
    /// Call on the main thread before [`run`]. Also creates the application
    /// object (as an accessory: no Dock icon) so a tray icon can be built.
    pub fn for_current_thread() -> Self {
        if let Some(mtm) = MainThreadMarker::new() {
            let app = NSApplication::sharedApplication(mtm);
            app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
        }
        if WAKE_SOURCE.load(Ordering::Relaxed) == 0 {
            let mut ctx = CFRunLoopSourceContext {
                version: 0,
                info: std::ptr::null_mut(),
                retain: None,
                release: None,
                copyDescription: None,
                equal: None,
                hash: None,
                schedule: None,
                cancel: None,
                perform: Some(perform),
            };
            // SAFETY: context outlives the call (it is copied by CF).
            if let Some(src) = unsafe { CFRunLoopSource::new(None, 0, &mut ctx) } {
                if let Some(rl) = CFRunLoop::current() {
                    // SAFETY: reading a global constant.
                    rl.add_source(Some(&src), unsafe { kCFRunLoopCommonModes });
                    MAIN_LOOP.store(&*rl as *const CFRunLoop as usize, Ordering::Relaxed);
                    // Keep it alive for the life of the process.
                    WAKE_SOURCE.store(
                        objc2_core_foundation::CFRetained::into_raw(src).as_ptr() as usize,
                        Ordering::Relaxed,
                    );
                }
            }
        }
        Self
    }

    pub fn wake(&self) {
        let src = WAKE_SOURCE.load(Ordering::Relaxed);
        let rl = MAIN_LOOP.load(Ordering::Relaxed);
        if src != 0 && rl != 0 {
            // SAFETY: both pointers were leaked on purpose and stay valid.
            unsafe {
                (*(src as *const CFRunLoopSource)).signal();
                (*(rl as *const CFRunLoop)).wake_up();
            }
        }
    }

    pub fn quit(&self) {
        QUIT.store(true, Ordering::Relaxed);
        self.wake();
    }
}

/// Pump AppKit events on the main thread until [`Waker::quit`]. `on_message`
/// runs after every batch of events and at least every 100 ms, so callers
/// can drain tray/menu event queues there.
pub fn run(mut on_message: impl FnMut()) {
    let Some(mtm) = MainThreadMarker::new() else {
        eprintln!("message loop must run on the main thread");
        return;
    };
    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
    app.finishLaunching();
    while !QUIT.load(Ordering::Relaxed) {
        let deadline = NSDate::dateWithTimeIntervalSinceNow(0.1);
        // SAFETY: main thread, valid mask/date/mode.
        while let Some(ev) = unsafe {
            app.nextEventMatchingMask_untilDate_inMode_dequeue(
                NSEventMask(u64::MAX),
                Some(&deadline),
                NSDefaultRunLoopMode,
                true,
            )
        } {
            app.sendEvent(&ev);
            if QUIT.load(Ordering::Relaxed) {
                break;
            }
        }
        on_message();
    }
}
