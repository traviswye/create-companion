//! Low-level keyboard hook (`WH_KEYBOARD_LL`) plus the foreground watch, both
//! living on one dedicated message-pump thread.
//!
//! Design (docs/PLAN.md §4.1): the callback does the bare minimum. It checks a
//! static reserved-key table, skips events we injected ourselves, works out
//! which modifiers belong to the key, pushes a [`RawTransportEvent`] onto a
//! channel and returns `1` so the key never reaches the foreground
//! application. Everything else happens on the engine's worker thread.
//!
//! Reserved keys, learn mode and the modifier-timing rule live in
//! `crate::hookcore`, shared with the macOS event tap.
//!
//! Only a single hook instance can exist per process.

use crate::windows::foreground;
use crate::{InputHook, PlatformError, RawTransportEvent};
use companion_core::transport::FunctionKey;
use crossbeam_channel::Sender;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{mpsc, OnceLock};
use std::thread::JoinHandle;
use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, PostThreadMessageW, SetWindowsHookExW,
    TranslateMessage, UnhookWindowsHookEx, HC_ACTION, KBDLLHOOKSTRUCT, LLKHF_INJECTED, LLKHF_UP,
    MSG, WH_KEYBOARD_LL, WM_QUIT,
};

static SENDER: OnceLock<Sender<RawTransportEvent>> = OnceLock::new();
static HOOK_THREAD_ID: AtomicU32 = AtomicU32::new(0);

use crate::hookcore::{self, ModSlot};
pub use crate::hookcore::{set_allow_injected, set_learn, set_namespace_window_ms, set_reserved};

/// Which modifier slot a virtual key belongs to.
fn modifier_slot(vk: u16) -> Option<ModSlot> {
    match vk {
        0x11 | 0xA2 | 0xA3 => Some(ModSlot::Ctrl),
        0x10 | 0xA0 | 0xA1 => Some(ModSlot::Shift),
        0x12 | 0xA4 | 0xA5 => Some(ModSlot::Alt),
        0x5B | 0x5C => Some(ModSlot::Meta),
        _ => None,
    }
}

unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        // SAFETY: for WH_KEYBOARD_LL with HC_ACTION, lparam points to a KBDLLHOOKSTRUCT.
        let info = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
        let vk = info.vkCode as u16;
        let injected = (info.flags & LLKHF_INJECTED).0 != 0;
        let up = (info.flags & LLKHF_UP).0 != 0;
        // Track physical modifier presses so F-keys can be classified.
        if let Some(slot) = modifier_slot(vk) {
            if !injected {
                hookcore::note_modifier(slot, !up);
            }
        }
        if let Some(key) = FunctionKey::from_windows_vk(vk) {
            let d = hookcore::decide(key, !up, injected);
            if let (Some(ev), Some(tx)) = (d.event, SENDER.get()) {
                let _ = tx.try_send(ev);
            }
            if d.swallow {
                // The transport key must not reach the foreground app.
                return LRESULT(1);
            }
        }
    }
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

pub struct KeyboardHook {
    tx: Option<Sender<RawTransportEvent>>,
    thread: Option<JoinHandle<()>>,
}

impl KeyboardHook {
    /// `tx` receives every swallowed transport event. Use a bounded channel;
    /// the hook drops events rather than block if the consumer stalls.
    pub fn new(tx: Sender<RawTransportEvent>) -> Self {
        Self {
            tx: Some(tx),
            thread: None,
        }
    }
}

impl InputHook for KeyboardHook {
    fn start(&mut self) -> Result<(), PlatformError> {
        let tx = self
            .tx
            .take()
            .ok_or_else(|| PlatformError::Os("hook already started".into()))?;
        SENDER
            .set(tx)
            .map_err(|_| PlatformError::Os("only one keyboard hook per process".into()))?;

        let (ready_tx, ready_rx) = mpsc::channel::<Result<(), String>>();
        let thread = std::thread::Builder::new()
            .name("cc-win32-hooks".into())
            .spawn(move || {
                // SAFETY: standard hook installation + message pump on this thread.
                unsafe {
                    let hook = match SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_proc), None, 0) {
                        Ok(h) => h,
                        Err(e) => {
                            let _ = ready_tx.send(Err(e.to_string()));
                            return;
                        }
                    };
                    let fg_watch = foreground::install_watch();
                    HOOK_THREAD_ID.store(GetCurrentThreadId(), Ordering::Relaxed);
                    let _ = ready_tx.send(Ok(()));

                    let mut msg = MSG::default();
                    while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                        let _ = TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                    }
                    if let Some(h) = fg_watch {
                        foreground::uninstall_watch(h);
                    }
                    let _ = UnhookWindowsHookEx(hook);
                }
            })
            .map_err(|e| PlatformError::Os(e.to_string()))?;

        match ready_rx.recv() {
            Ok(Ok(())) => {
                self.thread = Some(thread);
                Ok(())
            }
            Ok(Err(e)) => Err(PlatformError::Os(format!("SetWindowsHookEx failed: {e}"))),
            Err(_) => Err(PlatformError::Os("hook thread died during start".into())),
        }
    }

    fn stop(&mut self) {
        let tid = HOOK_THREAD_ID.swap(0, Ordering::Relaxed);
        if tid != 0 {
            // SAFETY: posting WM_QUIT to our own hook thread.
            unsafe {
                let _ = PostThreadMessageW(tid, WM_QUIT, WPARAM(0), LPARAM(0));
            }
        }
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

impl Drop for KeyboardHook {
    fn drop(&mut self) {
        self.stop();
    }
}
