//! A plain Win32 message loop for the main thread (the tray icon needs one),
//! plus a [`Waker`] other threads use to nudge it.

use windows::Win32::Foundation::{LPARAM, WPARAM};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetMessageW, PostThreadMessageW, TranslateMessage, MSG, WM_APP, WM_QUIT,
};

/// Posts to the thread that created it. `wake` delivers `WM_APP`, which
/// [`run`] treats as "state changed, call the handler".
#[derive(Debug, Clone, Copy)]
pub struct Waker {
    tid: u32,
}

impl Waker {
    pub fn for_current_thread() -> Self {
        // SAFETY: no preconditions.
        Self {
            tid: unsafe { GetCurrentThreadId() },
        }
    }

    pub fn wake(&self) {
        // SAFETY: posting to a thread we own; failure (no queue yet) is harmless.
        unsafe {
            let _ = PostThreadMessageW(self.tid, WM_APP, WPARAM(0), LPARAM(0));
        }
    }

    pub fn quit(&self) {
        // SAFETY: as above.
        unsafe {
            let _ = PostThreadMessageW(self.tid, WM_QUIT, WPARAM(0), LPARAM(0));
        }
    }
}

/// Pump messages until `WM_QUIT`. `on_message` runs after every dispatched
/// message, so callers can drain tray/menu event queues there.
pub fn run(mut on_message: impl FnMut()) {
    let mut msg = MSG::default();
    // SAFETY: standard message loop.
    unsafe {
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
            on_message();
        }
    }
}
