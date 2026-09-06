//! The OS-independent half of the keyboard hook: which F-keys are reserved,
//! learn mode, whether injected events count, and the modifier-timing rule
//! that decides which held modifiers belong to the module and which to the
//! human. Both the Windows low-level hook and the macOS event tap feed their
//! raw key and modifier events through here so they behave identically.
//!
//! Modifier namespaces by timing: a module that sends `Shift+F13` presses
//! Shift about a millisecond before F13. A Shift the user is holding has been
//! down far longer. So a modifier counts as part of the transport code only if
//! it went down within the namespace window before the F-key; older ones are
//! reported as `held` and ignored for decoding. Physical modifier presses are
//! tracked here rather than read back from the OS, so modifiers the engine
//! injects for its own actions never leak into the decision.

use crate::RawTransportEvent;
use companion_core::transport::{FunctionKey, Modifiers, TransportCode, TransportTable};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::Instant;

static RESERVED: [AtomicBool; 12] = [const { AtomicBool::new(false) }; 12];
static ALLOW_INJECTED: AtomicBool = AtomicBool::new(false);
/// Learn mode: report every F13-F24 press (reserved or not) so the UI can
/// discover what a module sends. Unreserved keys are not swallowed.
static LEARN: AtomicBool = AtomicBool::new(false);
/// How long before an F-key a modifier may have gone down and still count as
/// the module's namespace (milliseconds). Config: `engine.namespace_window_ms`.
static NAMESPACE_WINDOW_MS: AtomicU64 = AtomicU64::new(100);
/// When each physical modifier went down, in ms since [`now_ms`]'s epoch;
/// 0 = up. Slots: see [`ModSlot`].
static MOD_DOWN_AT: [AtomicU64; 5] = [const { AtomicU64::new(0) }; 5];
static EPOCH: OnceLock<Instant> = OnceLock::new();

/// A modifier key the hook tracks. `Fn` only exists on macOS keyboards.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModSlot {
    Ctrl = 0,
    Shift = 1,
    Alt = 2,
    Meta = 3,
    Fn = 4,
}

/// Publish which function keys the hook should swallow. Safe to call while
/// the hook is running (config hot reload).
pub fn set_reserved(table: &TransportTable) {
    for (i, slot) in RESERVED.iter().enumerate() {
        let key = FunctionKey::from_windows_vk(0x7C + i as u16).expect("index within F13..F24");
        slot.store(table.is_reserved(key), Ordering::Relaxed);
    }
}

/// Testing aid: also treat injected (synthetic) F-keys as transport. Off by
/// default so our own output can never loop back.
pub fn set_allow_injected(allow: bool) {
    ALLOW_INJECTED.store(allow, Ordering::Relaxed);
}

/// Arm or disarm learn mode (see [`LEARN`]).
pub fn set_learn(on: bool) {
    LEARN.store(on, Ordering::Relaxed);
}

/// Set the namespace window (see [`NAMESPACE_WINDOW_MS`]).
pub fn set_namespace_window_ms(ms: u32) {
    NAMESPACE_WINDOW_MS.store(u64::from(ms.max(1)), Ordering::Relaxed);
}

pub fn allow_injected() -> bool {
    ALLOW_INJECTED.load(Ordering::Relaxed)
}

pub fn learning() -> bool {
    LEARN.load(Ordering::Relaxed)
}

pub fn is_reserved(key: FunctionKey) -> bool {
    RESERVED[(key.windows_vk() - 0x7C) as usize].load(Ordering::Relaxed)
}

pub fn now_ms() -> u64 {
    let epoch = EPOCH.get_or_init(Instant::now);
    // +1 so a press in the very first millisecond is never mistaken for "up".
    epoch.elapsed().as_millis() as u64 + 1
}

/// Record a physical modifier press or release.
pub fn note_modifier(slot: ModSlot, down: bool) {
    let cell = &MOD_DOWN_AT[slot as usize];
    if !down {
        cell.store(0, Ordering::Relaxed);
    } else if cell.load(Ordering::Relaxed) == 0 {
        cell.store(now_ms(), Ordering::Relaxed);
    }
}

/// Split the modifiers currently down into the module's (fresh) and the
/// user's (held), judged by how long ago each went down.
pub fn classify_modifiers(now: u64) -> (Modifiers, Modifiers) {
    let window = NAMESPACE_WINDOW_MS.load(Ordering::Relaxed);
    let mut fresh = [false; 5];
    let mut held = [false; 5];
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
    let make = |m: [bool; 5]| Modifiers {
        ctrl: m[0],
        shift: m[1],
        alt: m[2],
        meta: m[3],
        fn_key: m[4],
    };
    (make(fresh), make(held))
}

/// What the platform hook should do with an F13-F24 key event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Decision {
    /// Forward this to the engine (always, unless it is our own injected key).
    pub event: Option<RawTransportEvent>,
    /// Stop the key from reaching the foreground application.
    pub swallow: bool,
}

/// Classify one F-key event. Every physical F13-F24 event is reported so the
/// UI can say "the keyboard sent Shift+F16 but no input uses it"; only
/// reserved keys are swallowed. Injected keys are ignored unless
/// `set_allow_injected(true)`.
pub fn decide(key: FunctionKey, pressed: bool, injected: bool) -> Decision {
    if injected && !allow_injected() {
        return Decision {
            event: None,
            swallow: false,
        };
    }
    let reserved = is_reserved(key);
    let (mods, held) = classify_modifiers(now_ms());
    Decision {
        event: Some(RawTransportEvent {
            code: TransportCode { key, mods },
            held,
            pressed,
            at: Instant::now(),
            reserved,
        }),
        swallow: reserved,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// The statics are process-wide; run these one at a time.
    static SERIAL: Mutex<()> = Mutex::new(());

    #[test]
    fn fresh_modifier_joins_the_code_and_a_held_one_does_not() {
        let _g = SERIAL.lock().unwrap();
        set_namespace_window_ms(100);
        note_modifier(ModSlot::Shift, true);
        let now = now_ms();
        let (fresh, held) = classify_modifiers(now);
        assert!(fresh.shift && !held.shift);
        // The same press, judged ten seconds later: the human is holding it.
        let (fresh, held) = classify_modifiers(now + 10_000);
        assert!(!fresh.shift && held.shift);
        note_modifier(ModSlot::Shift, false);
        let (fresh, held) = classify_modifiers(now_ms());
        assert!(!fresh.shift && !held.shift);
    }

    #[test]
    fn injected_keys_are_ignored_unless_allowed() {
        let _g = SERIAL.lock().unwrap();
        set_allow_injected(false);
        assert_eq!(decide(FunctionKey::F13, true, true).event, None);
        set_allow_injected(true);
        assert!(decide(FunctionKey::F13, true, true).event.is_some());
        set_allow_injected(false);
    }
}
