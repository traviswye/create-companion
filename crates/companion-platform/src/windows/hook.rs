//! Low-level keyboard hook (`WH_KEYBOARD_LL`) plus the foreground watch, both
//! living on one dedicated message-pump thread.
//!
//! Design (docs/PLAN.md §4.1): the callback does the bare minimum. It checks a
//! static reserved-key table, skips events we injected ourselves, works out
//! which modifiers belong to the key, pushes a [`RawTransportEvent`] onto a
//! channel and returns `1` so the key never reaches the foreground
//! application. Everything else happens on the engine's worker thread.
//!
//! Modifier namespaces by timing: a module that sends `Shift+F13` presses
//! Shift about a millisecond before F13. A Shift the user is holding has been
//! down far longer. So a modifier counts as part of the transport code only if
//! it went down within [`NAMESPACE_WINDOW_MS`] before the F-key; older ones
//! are reported as `held` and ignored for decoding. The hook tracks physical
//! modifier presses itself rather than asking Windows, so modifiers the engine
//! injects for its own actions never leak into the decision.
//!
//! Only a single hook instance can exist per process.

use crate::windows::foreground;
use crate::{InputHook, PlatformError, RawTransportEvent};
use companion_core::transport::{FunctionKey, Modifiers, TransportCode, TransportTable};
use crossbeam_channel::Sender;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{mpsc, OnceLock};
use std::thread::JoinHandle;
use std::time::Instant;
use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, PostThreadMessageW, SetWindowsHookExW,
    TranslateMessage, UnhookWindowsHookEx, HC_ACTION, KBDLLHOOKSTRUCT, LLKHF_INJECTED, LLKHF_UP,
    MSG, WH_KEYBOARD_LL, WM_QUIT,
};

static SENDER: OnceLock<Sender<RawTransportEvent>> = OnceLock::new();
static RESERVED: [AtomicBool; 12] = [const { AtomicBool::new(false) }; 12];
static ALLOW_INJECTED: AtomicBool = AtomicBool::new(false);
/// Learn mode: report every F13-F24 press (reserved or not) so the UI can
/// discover what a module sends. Unreserved keys are not swallowed.
static LEARN: AtomicBool = AtomicBool::new(false);
static HOOK_THREAD_ID: AtomicU32 = AtomicU32::new(0);

/// How long before an F-key a modifier may have gone down and still count as
/// the module's namespace (milliseconds). Config: `engine.namespace_window_ms`.
static NAMESPACE_WINDOW_MS: AtomicU64 = AtomicU64::new(100);
/// When each physical modifier went down, in ms since [`epoch`]; 0 = up.
/// Slots: ctrl, shift, alt, win.
static MOD_DOWN_AT: [AtomicU64; 4] = [const { AtomicU64::new(0) }; 4];
static EPOCH: OnceLock<Instant> = OnceLock::new();

fn now_ms() -> u64 {
    let epoch = EPOCH.get_or_init(Instant::now);
    // +1 so a press in the very first millisecond is never mistaken for "up".
    epoch.elapsed().as_millis() as u64 + 1
}

/// Which modifier slot a virtual key belongs to.
fn modifier_slot(vk: u16) -> Option<usize> {
    match vk {
        0x11 | 0xA2 | 0xA3 => Some(0), // Ctrl
        0x10 | 0xA0 | 0xA1 => Some(1), // Shift
        0x12 | 0xA4 | 0xA5 => Some(2), // Alt
        0x5B | 0x5C => Some(3),        // Win
        _ => None,
    }
}

/// Split the modifiers currently down into the module's (fresh) and the
/// user's (held), judged by how long ago each went down.
fn classify_modifiers(now: u64) -> (Modifiers, Modifiers) {
    let window = NAMESPACE_WINDOW_MS.load(Ordering::Relaxed);
    let mut fresh = [false; 4];
    let mut held = [false; 4];
    for (i, slot) in MOD_DOWN_AT.iter().enumerate() {
        let t = slot.load(Ordering::Relaxed);
        if t == 0 {
            continue;
        }
        if now.saturating_sub(t) <= window {
            fresh[i] = true;
        } else {
            held[i] = true;
        }
    }
    let make = |m: [bool; 4]| Modifiers {
        ctrl: m[0],
        shift: m[1],
        alt: m[2],
        meta: m[3],
        fn_key: false,
    };
    (make(fresh), make(held))
}

/// Set the namespace window (see [`NAMESPACE_WINDOW_MS`]).
pub fn set_namespace_window_ms(ms: u32) {
    NAMESPACE_WINDOW_MS.store(u64::from(ms.max(1)), Ordering::Relaxed);
}

/// Publish which function keys the hook should swallow. Safe to call while
/// the hook is running (config hot reload).
pub fn set_reserved(table: &TransportTable) {
    for (i, slot) in RESERVED.iter().enumerate() {
        let key = FunctionKey::from_windows_vk(0x7C + i as u16).expect("index within F13..F24");
        slot.store(table.is_reserved(key), Ordering::Relaxed);
    }
}

/// Testing aid: also treat injected (SendInput / keybd_event) F-keys as
/// transport. Off by default so our own synthetic output can never loop back.
pub fn set_allow_injected(allow: bool) {
    ALLOW_INJECTED.store(allow, Ordering::Relaxed);
}

/// Arm or disarm learn mode (see [`LEARN`]).
pub fn set_learn(on: bool) {
    LEARN.store(on, Ordering::Relaxed);
}

unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        // SAFETY: for WH_KEYBOARD_LL with HC_ACTION, lparam points to a KBDLLHOOKSTRUCT.
        let info = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
        let vk = info.vkCode as u16;
        let injected = (info.flags & LLKHF_INJECTED).0 != 0;
        // Track physical modifier presses so F-keys can be classified below.
        if let Some(slot) = modifier_slot(vk) {
            if !injected {
                let up = (info.flags & LLKHF_UP).0 != 0;
                if up {
                    MOD_DOWN_AT[slot].store(0, Ordering::Relaxed);
                } else if MOD_DOWN_AT[slot].load(Ordering::Relaxed) == 0 {
                    MOD_DOWN_AT[slot].store(now_ms(), Ordering::Relaxed);
                }
            }
        }
        if (0x7C..=0x87).contains(&vk) {
            let reserved = RESERVED[(vk - 0x7C) as usize].load(Ordering::Relaxed);
            let learning = LEARN.load(Ordering::Relaxed);
            // Every F13-F24 event is reported so the UI can say "the keyboard sent
            // Shift+F16 but no input uses it"; only reserved keys are swallowed.
            let _ = learning;
            if !injected || ALLOW_INJECTED.load(Ordering::Relaxed) {
                if let (Some(tx), Some(key)) = (SENDER.get(), FunctionKey::from_windows_vk(vk)) {
                    let pressed = (info.flags & LLKHF_UP).0 == 0;
                    let (mods, held) = classify_modifiers(now_ms());
                    let ev = RawTransportEvent {
                        code: TransportCode { key, mods },
                        held,
                        pressed,
                        at: Instant::now(),
                        reserved,
                    };
                    let _ = tx.try_send(ev);
                }
                if reserved {
                    // Swallow: the transport key must not reach the foreground app.
                    return LRESULT(1);
                }
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
