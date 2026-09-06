//! CGEvent tap: sees every key and modifier event in the login session,
//! swallows reserved F-keys, and reports them to the engine. Runs on its own
//! thread with its own CFRunLoop, like the Windows hook thread.
//!
//! Needs the Input Monitoring permission (`permissions::input_monitoring`);
//! without it `CGEventTapCreate` returns null and `start` fails.

use crate::hookcore::{self, ModSlot};
use crate::{InputHook, PlatformError, RawTransportEvent};
use companion_core::transport::FunctionKey;
use crossbeam_channel::Sender;
use objc2_core_foundation::{kCFRunLoopCommonModes, CFMachPort, CFRunLoop};
use objc2_core_graphics::{
    CGEvent, CGEventField, CGEventFlags, CGEventTapLocation, CGEventTapOptions,
    CGEventTapPlacement, CGEventTapProxy, CGEventType,
};
use std::ffi::c_void;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, OnceLock};
use std::thread::JoinHandle;

pub use crate::hookcore::{set_allow_injected, set_learn, set_namespace_window_ms, set_reserved};

/// Value we stamp into `EventSourceUserData` on every event we post, so the
/// tap can tell its own output from the keyboard.
pub const INJECTED_MARK: i64 = 0x4343_0001;

static SENDER: OnceLock<Sender<RawTransportEvent>> = OnceLock::new();
/// The tap's mach port, so a tap the OS disabled (timeout) can be re-enabled
/// from the callback.
static TAP_PORT: AtomicUsize = AtomicUsize::new(0);

const CG_EVENT_TAP_DISABLED_BY_TIMEOUT: u32 = 0xFFFF_FFFE;
const CG_EVENT_TAP_DISABLED_BY_USER_INPUT: u32 = 0xFFFF_FFFF;

// Carbon virtual key codes for the modifier keys.
const KVK_COMMAND: i64 = 55;
const KVK_SHIFT: i64 = 56;
const KVK_OPTION: i64 = 58;
const KVK_CONTROL: i64 = 59;
const KVK_RIGHT_COMMAND: i64 = 54;
const KVK_RIGHT_SHIFT: i64 = 60;
const KVK_RIGHT_OPTION: i64 = 61;
const KVK_RIGHT_CONTROL: i64 = 62;
const KVK_FUNCTION: i64 = 63;

const FLAG_SHIFT: u64 = 0x0002_0000;
const FLAG_CONTROL: u64 = 0x0004_0000;
const FLAG_ALTERNATE: u64 = 0x0008_0000;
const FLAG_COMMAND: u64 = 0x0010_0000;
const FLAG_SECONDARY_FN: u64 = 0x0080_0000;

fn modifier_slot(keycode: i64) -> Option<(ModSlot, u64)> {
    match keycode {
        KVK_CONTROL | KVK_RIGHT_CONTROL => Some((ModSlot::Ctrl, FLAG_CONTROL)),
        KVK_SHIFT | KVK_RIGHT_SHIFT => Some((ModSlot::Shift, FLAG_SHIFT)),
        KVK_OPTION | KVK_RIGHT_OPTION => Some((ModSlot::Alt, FLAG_ALTERNATE)),
        KVK_COMMAND | KVK_RIGHT_COMMAND => Some((ModSlot::Meta, FLAG_COMMAND)),
        KVK_FUNCTION => Some((ModSlot::Fn, FLAG_SECONDARY_FN)),
        _ => None,
    }
}

unsafe extern "C-unwind" fn tap_callback(
    _proxy: CGEventTapProxy,
    event_type: CGEventType,
    event: NonNull<CGEvent>,
    _info: *mut c_void,
) -> *mut CGEvent {
    let ev: &CGEvent = unsafe { event.as_ref() };
    match event_type.0 {
        CG_EVENT_TAP_DISABLED_BY_TIMEOUT | CG_EVENT_TAP_DISABLED_BY_USER_INPUT => {
            // The OS switched us off (we were too slow, or the user pressed a
            // system key combination). Switch back on and pass the event.
            let port = TAP_PORT.load(Ordering::Relaxed);
            if port != 0 {
                // SAFETY: the pointer came from a live CFMachPort we retain
                // for the tap's whole life.
                let port: &CFMachPort = unsafe { &*(port as *const CFMachPort) };
                CGEvent::tap_enable(port, true);
            }
            return event.as_ptr();
        }
        _ => {}
    }
    let injected =
        CGEvent::integer_value_field(Some(ev), CGEventField::EventSourceUserData) == INJECTED_MARK;
    let keycode = CGEvent::integer_value_field(Some(ev), CGEventField::KeyboardEventKeycode);

    if event_type == CGEventType::FlagsChanged {
        if let Some((slot, flag)) = modifier_slot(keycode) {
            if !injected {
                let flags = CGEvent::flags(Some(ev));
                hookcore::note_modifier(slot, flags.0 & flag != 0);
            }
        }
        return event.as_ptr();
    }
    if event_type != CGEventType::KeyDown && event_type != CGEventType::KeyUp {
        return event.as_ptr();
    }
    let Some(key) = u16::try_from(keycode)
        .ok()
        .and_then(FunctionKey::from_macos_keycode)
    else {
        return event.as_ptr();
    };
    let d = hookcore::decide(key, event_type == CGEventType::KeyDown, injected);
    if let (Some(raw), Some(tx)) = (d.event, SENDER.get()) {
        let _ = tx.try_send(raw);
    }
    if d.swallow {
        std::ptr::null_mut()
    } else {
        event.as_ptr()
    }
}

/// Keeps the `Fn` modifier state honest when it arrives as a flag on a key
/// event rather than as its own FlagsChanged (some keyboards do that).
#[allow(dead_code)]
fn fn_flag_set(flags: CGEventFlags) -> bool {
    flags.0 & FLAG_SECONDARY_FN != 0
}

pub struct EventTapHook {
    tx: Option<Sender<RawTransportEvent>>,
    thread: Option<JoinHandle<()>>,
    /// The tap thread's CFRunLoop, to stop it.
    run_loop: Arc<AtomicUsize>,
}

impl EventTapHook {
    /// `tx` receives every transport event. Use a bounded channel; the tap
    /// drops events rather than block if the consumer stalls.
    pub fn new(tx: Sender<RawTransportEvent>) -> Self {
        Self {
            tx: Some(tx),
            thread: None,
            run_loop: Arc::new(AtomicUsize::new(0)),
        }
    }
}

impl InputHook for EventTapHook {
    fn start(&mut self) -> Result<(), PlatformError> {
        let tx = self
            .tx
            .take()
            .ok_or(PlatformError::Os("hook already started".into()))?;
        let _ = SENDER.set(tx);
        let (ready_tx, ready_rx) = mpsc::channel::<Result<(), PlatformError>>();
        let run_loop = Arc::clone(&self.run_loop);
        let thread = std::thread::Builder::new()
            .name("cc-event-tap".into())
            .spawn(move || {
                let mask: u64 = (1u64 << CGEventType::KeyDown.0)
                    | (1u64 << CGEventType::KeyUp.0)
                    | (1u64 << CGEventType::FlagsChanged.0);
                // SAFETY: the callback is a plain extern fn and reads only
                // statics; user_info is unused.
                let port = unsafe {
                    CGEvent::tap_create(
                        CGEventTapLocation::SessionEventTap,
                        CGEventTapPlacement::HeadInsertEventTap,
                        CGEventTapOptions::Default,
                        mask,
                        Some(tap_callback),
                        std::ptr::null_mut(),
                    )
                };
                let Some(port) = port else {
                    let _ = ready_tx.send(Err(PlatformError::Os(
                        "could not create the event tap: grant Input Monitoring to Create Companion in System Settings > Privacy & Security".into(),
                    )));
                    return;
                };
                let Some(source) = CFMachPort::new_run_loop_source(None, Some(&port), 0) else {
                    let _ = ready_tx.send(Err(PlatformError::Os("no run loop source for the tap".into())));
                    return;
                };
                let Some(rl) = CFRunLoop::current() else {
                    let _ = ready_tx.send(Err(PlatformError::Os("no run loop".into())));
                    return;
                };
                // SAFETY: reading a global constant.
                rl.add_source(Some(&source), unsafe { kCFRunLoopCommonModes });
                TAP_PORT.store(&*port as *const CFMachPort as usize, Ordering::Relaxed);
                CGEvent::tap_enable(&port, true);
                run_loop.store(&*rl as *const CFRunLoop as usize, Ordering::Relaxed);
                let _ = ready_tx.send(Ok(()));
                CFRunLoop::run();
                run_loop.store(0, Ordering::Relaxed);
                TAP_PORT.store(0, Ordering::Relaxed);
                drop(source);
                drop(port);
            })
            .map_err(|e| PlatformError::Os(e.to_string()))?;
        self.thread = Some(thread);
        match ready_rx.recv() {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(_) => Err(PlatformError::Os("event tap thread exited early".into())),
        }
    }

    fn stop(&mut self) {
        let rl = self.run_loop.load(Ordering::Relaxed);
        if rl != 0 {
            // SAFETY: the pointer is the tap thread's live run loop.
            let rl: &CFRunLoop = unsafe { &*(rl as *const CFRunLoop) };
            rl.stop();
        }
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

impl Drop for EventTapHook {
    fn drop(&mut self) {
        self.stop();
    }
}
