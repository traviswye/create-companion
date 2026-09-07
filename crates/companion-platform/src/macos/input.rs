//! Action execution on macOS: keyboard and scroll events through CGEvent,
//! media keys through the AppKit "system defined" event, programs through
//! `open` or a direct spawn, commands through `/bin/sh`.
//!
//! Every event we post carries [`INJECTED_MARK`] in its user-data field so the
//! event tap ignores our own output.

use super::hook::INJECTED_MARK;
use crate::{ActionSink, PlatformError};
use companion_core::action::{Action, KeyChord, MediaKey, ScrollDirection};
use companion_core::keys::{Key, ParsedChord};
use companion_core::transport::Modifiers;
use objc2_app_kit::{NSEvent, NSEventModifierFlags, NSEventType};
use objc2_core_foundation::CFRetained;
use objc2_core_graphics::{
    CGEvent, CGEventField, CGEventFlags, CGEventSource, CGEventSourceStateID, CGEventTapLocation,
    CGScrollEventUnit,
};
use objc2_foundation::NSPoint;

const FLAG_SHIFT: u64 = 0x0002_0000;
const FLAG_CONTROL: u64 = 0x0004_0000;
const FLAG_ALTERNATE: u64 = 0x0008_0000;
const FLAG_COMMAND: u64 = 0x0010_0000;
const FLAG_SECONDARY_FN: u64 = 0x0080_0000;

const KVK_COMMAND: u16 = 55;
const KVK_SHIFT: u16 = 56;
const KVK_OPTION: u16 = 58;
const KVK_CONTROL: u16 = 59;
const KVK_FUNCTION: u16 = 63;

/// Carbon virtual key code for a chord key (ANSI layout).
fn keycode_for(key: Key) -> Option<u16> {
    Some(match key {
        Key::Letter(c) => match c.to_ascii_uppercase() {
            b'A' => 0,
            b'S' => 1,
            b'D' => 2,
            b'F' => 3,
            b'H' => 4,
            b'G' => 5,
            b'Z' => 6,
            b'X' => 7,
            b'C' => 8,
            b'V' => 9,
            b'B' => 11,
            b'Q' => 12,
            b'W' => 13,
            b'E' => 14,
            b'R' => 15,
            b'Y' => 16,
            b'T' => 17,
            b'O' => 31,
            b'U' => 32,
            b'I' => 34,
            b'P' => 35,
            b'L' => 37,
            b'J' => 38,
            b'K' => 40,
            b'N' => 45,
            b'M' => 46,
            _ => return None,
        },
        Key::Digit(d) => match d {
            1 => 18,
            2 => 19,
            3 => 20,
            4 => 21,
            6 => 22,
            5 => 23,
            9 => 25,
            7 => 26,
            8 => 28,
            0 => 29,
            _ => return None,
        },
        Key::Function(n) => match n {
            1 => 122,
            2 => 120,
            3 => 99,
            4 => 118,
            5 => 96,
            6 => 97,
            7 => 98,
            8 => 100,
            9 => 101,
            10 => 109,
            11 => 103,
            12 => 111,
            13 => 105,
            14 => 107,
            15 => 113,
            16 => 106,
            17 => 64,
            18 => 79,
            19 => 80,
            20 => 90,
            _ => return None,
        },
        Key::Tab => 48,
        Key::Enter => 36,
        Key::Escape => 53,
        Key::Space => 49,
        Key::Backspace => 51,
        Key::Delete => 117,
        Key::Insert => 114, // Help key on Apple keyboards
        Key::Home => 115,
        Key::End => 119,
        Key::PageUp => 116,
        Key::PageDown => 121,
        Key::Left => 123,
        Key::Right => 124,
        Key::Up => 126,
        Key::Down => 125,
        Key::Minus => 27,
        Key::Equals => 24,
        Key::LeftBracket => 33,
        Key::RightBracket => 30,
        Key::Backslash => 42,
        Key::Semicolon => 41,
        Key::Apostrophe => 39,
        Key::Comma => 43,
        Key::Period => 47,
        Key::Slash => 44,
        Key::Grave => 50,
    })
}

fn flags_for(m: Modifiers) -> u64 {
    let mut f = 0;
    if m.ctrl {
        f |= FLAG_CONTROL;
    }
    if m.shift {
        f |= FLAG_SHIFT;
    }
    if m.alt {
        f |= FLAG_ALTERNATE;
    }
    if m.meta {
        f |= FLAG_COMMAND;
    }
    if m.fn_key {
        f |= FLAG_SECONDARY_FN;
    }
    f
}

/// Modifier key codes for a set, in press order.
fn mod_keycodes(m: Modifiers) -> Vec<u16> {
    let mut v = Vec::with_capacity(5);
    if m.ctrl {
        v.push(KVK_CONTROL);
    }
    if m.shift {
        v.push(KVK_SHIFT);
    }
    if m.alt {
        v.push(KVK_OPTION);
    }
    if m.meta {
        v.push(KVK_COMMAND);
    }
    if m.fn_key {
        v.push(KVK_FUNCTION);
    }
    v
}

fn parse(chord: &KeyChord) -> Result<ParsedChord, PlatformError> {
    chord
        .0
        .parse()
        .map_err(|e| PlatformError::Os(format!("bad chord `{}`: {e}", chord.0)))
}

/// Executes actions by posting CGEvents at the HID level.
pub struct CGEventSink {
    source: Option<CFRetained<CGEventSource>>,
}

// SAFETY: the event source is a plain CF object that is only ever used from
// the pipeline thread that owns this sink; nothing here is shared.
unsafe impl Send for CGEventSink {}

impl Default for CGEventSink {
    fn default() -> Self {
        Self {
            source: CGEventSource::new(CGEventSourceStateID::HIDSystemState),
        }
    }
}

impl CGEventSink {
    fn post(&self, ev: &CGEvent) {
        CGEvent::set_integer_value_field(
            Some(ev),
            CGEventField::EventSourceUserData,
            INJECTED_MARK,
        );
        CGEvent::post(CGEventTapLocation::HIDEventTap, Some(ev));
    }

    fn key(&self, keycode: u16, down: bool, flags: u64) -> Result<(), PlatformError> {
        let ev = CGEvent::new_keyboard_event(self.source.as_deref(), keycode, down)
            .ok_or_else(|| PlatformError::Os("could not create a keyboard event".into()))?;
        CGEvent::set_flags(Some(&ev), CGEventFlags(flags));
        self.post(&ev);
        Ok(())
    }

    /// Press the chord's modifiers, tap the key `repeat` times, release.
    fn chord(&self, parsed: &ParsedChord, repeat: u32) -> Result<(), PlatformError> {
        let keycode = keycode_for(parsed.key)
            .ok_or_else(|| PlatformError::Os(format!("no macOS key code for {:?}", parsed.key)))?;
        let flags = flags_for(parsed.mods);
        let mods = mod_keycodes(parsed.mods);
        for m in &mods {
            self.key(*m, true, flags)?;
        }
        for _ in 0..repeat.max(1) {
            self.key(keycode, true, flags)?;
            self.key(keycode, false, flags)?;
        }
        for m in mods.iter().rev() {
            self.key(*m, false, 0)?;
        }
        Ok(())
    }

    /// Media keys are "system defined" NSEvents (subtype 8) rather than key codes.
    fn media(&self, key: MediaKey, repeat: u32) -> Result<(), PlatformError> {
        // NX_KEYTYPE_* from IOKit/hidsystem/ev_keymap.h
        let key_type: i64 = match key {
            MediaKey::VolumeUp => 0,
            MediaKey::VolumeDown => 1,
            MediaKey::BrightnessUp => 2,
            MediaKey::BrightnessDown => 3,
            MediaKey::Mute => 7,
            MediaKey::PlayPause => 16,
            MediaKey::NextTrack => 17,
            MediaKey::PreviousTrack => 18,
        };
        for _ in 0..repeat.max(1) {
            for down in [true, false] {
                let state: i64 = if down { 0xA } else { 0xB };
                let data1 = (key_type << 16) | (state << 8);
                let ns = NSEvent::otherEventWithType_location_modifierFlags_timestamp_windowNumber_context_subtype_data1_data2(
                    NSEventType::SystemDefined,
                    NSPoint::new(0.0, 0.0),
                    NSEventModifierFlags(0),
                    0.0,
                    0,
                    None,
                    8,
                    data1 as isize,
                    -1,
                );
                let ns = ns.ok_or_else(|| {
                    PlatformError::Os("could not create a media key event".into())
                })?;
                let cg = ns
                    .CGEvent()
                    .ok_or_else(|| PlatformError::Os("media key event has no CGEvent".into()))?;
                self.post(&cg);
            }
        }
        Ok(())
    }
}

impl ActionSink for CGEventSink {
    fn execute(&mut self, action: &Action, repeat: u32) -> Result<(), PlatformError> {
        let repeat = repeat.max(1);
        match action {
            Action::Noop => Ok(()),
            Action::Release => Ok(()),
            Action::Keys { chord, .. } => self.chord(&parse(chord)?, repeat),
            Action::Sequence { chords } => {
                for c in chords {
                    self.chord(&parse(c)?, 1)?;
                }
                Ok(())
            }
            Action::Media { key } => self.media(*key, repeat),
            Action::Scroll { direction, lines } => {
                let magnitude = (*lines as i32) * repeat as i32;
                let (v, h) = match direction {
                    ScrollDirection::Up => (magnitude, 0),
                    ScrollDirection::Down => (-magnitude, 0),
                    ScrollDirection::Left => (0, magnitude),
                    ScrollDirection::Right => (0, -magnitude),
                };
                let ev = CGEvent::new_scroll_wheel_event2(
                    self.source.as_deref(),
                    CGScrollEventUnit::Line,
                    2,
                    v,
                    h,
                    0,
                )
                .ok_or_else(|| PlatformError::Os("could not create a scroll event".into()))?;
                self.post(&ev);
                Ok(())
            }
            Action::Launch { program, args } => {
                // A bare app name or a .app bundle goes through `open -a`;
                // anything else is spawned directly.
                let looks_like_app = program.ends_with(".app") || !program.contains('/');
                let mut cmd = if looks_like_app {
                    let mut c = std::process::Command::new("open");
                    c.arg("-a").arg(program);
                    if !args.is_empty() {
                        c.arg("--args").args(args);
                    }
                    c
                } else {
                    let mut c = std::process::Command::new(program);
                    c.args(args);
                    c
                };
                cmd.spawn()
                    .map(|_| ())
                    .map_err(|e| PlatformError::Os(format!("launch `{program}`: {e}")))
            }
            Action::Command { command } => std::process::Command::new("/bin/sh")
                .args(["-c", command])
                .spawn()
                .map(|_| ())
                .map_err(|e| PlatformError::Os(format!("command `{command}`: {e}"))),
        }
    }

    fn release_modifiers(&mut self, mods: Modifiers) -> Result<(), PlatformError> {
        for m in mod_keycodes(mods).iter().rev() {
            self.key(*m, false, 0)?;
        }
        Ok(())
    }
}
