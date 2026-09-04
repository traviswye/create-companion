//! Low-level keyboard hook (`WH_KEYBOARD_LL`) plus the foreground watch, both
//! living on one dedicated message-pump thread.
//!
//! Design (PLAN.md §4.1): the callback does the bare minimum. It checks a
//! static reserved-key table, skips events we injected ourselves, reads the
//! modifier state, pushes a [`RawTransportEvent`] onto a channel and returns
//! `1` so the key never reaches the foreground application. Everything else
//! happens on the engine's worker thread.
//!
//! Only a single hook instance can exist per process.

use crate::windows::foreground;
use crate::{InputHook, PlatformError, RawTransportEvent};
use companion_core::transport::{FunctionKey, Modifiers, TransportCode, TransportTable};
use crossbeam_channel::Sender;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{mpsc, OnceLock};
use std::thread::JoinHandle;
use std::time::Instant;
use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, VK_CONTROL, VK_MENU, VK_SHIFT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, PostThreadMessageW, SetWindowsHookExW,
    TranslateMessage, UnhookWindowsHookEx, HC_ACTION, KBDLLHOOKSTRUCT, LLKHF_INJECTED, LLKHF_UP,
    MSG, WH_KEYBOARD_LL, WM_QUIT,
};

static SENDER: OnceLock<Sender<RawTransportEvent>> = OnceLock::new();
static RESERVED: [AtomicBool; 12] = [const { AtomicBool::new(false) }; 12];
static ALLOW_INJECTED: AtomicBool = AtomicBool::new(false);
static HOOK_THREAD_ID: AtomicU32 = AtomicU32::new(0);

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

fn current_modifiers() -> Modifiers {
    // SAFETY: GetAsyncKeyState has no preconditions.
    let down = |vk: u16| unsafe { (GetAsyncKeyState(vk as i32) as u16 & 0x8000) != 0 };
    let ctrl = down(VK_CONTROL.0);
    let shift = down(VK_SHIFT.0);
    let alt = down(VK_MENU.0);
    match (ctrl, shift, alt) {
        (false, false, false) => Modifiers::None,
        (false, true, false) => Modifiers::Shift,
        (true, false, false) => Modifiers::Ctrl,
        (false, false, true) => Modifiers::Alt,
        (true, true, false) => Modifiers::CtrlShift,
        // Unrecognised combination: report as plain and let the decoder
        // decide (it will log an unmapped transport code).
        _ => Modifiers::None,
    }
}

unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        // SAFETY: for WH_KEYBOARD_LL with HC_ACTION, lparam points to a KBDLLHOOKSTRUCT.
        let info = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
        let vk = info.vkCode as u16;
        if (0x7C..=0x87).contains(&vk) && RESERVED[(vk - 0x7C) as usize].load(Ordering::Relaxed) {
            let injected = (info.flags & LLKHF_INJECTED).0 != 0;
            if !injected || ALLOW_INJECTED.load(Ordering::Relaxed) {
                if let (Some(tx), Some(key)) = (SENDER.get(), FunctionKey::from_windows_vk(vk)) {
                    let pressed = (info.flags & LLKHF_UP).0 == 0;
                    let ev = RawTransportEvent {
                        code: TransportCode {
                            key,
                            mods: current_modifiers(),
                        },
                        pressed,
                        at: Instant::now(),
                    };
                    let _ = tx.try_send(ev);
                }
                // Swallow: the transport key must not reach the foreground app.
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
            .name("naya-win32-hooks".into())
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
