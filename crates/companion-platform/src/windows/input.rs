//! Action execution on Windows via `SendInput`.

use crate::{ActionSink, PlatformError};
use companion_core::action::{Action, KeyChord, MediaKey, ScrollDirection};
use companion_core::keys::{Key, ParsedChord};
use companion_core::transport::Modifiers;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT, KEYBD_EVENT_FLAGS,
    KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP, MOUSEEVENTF_HWHEEL, MOUSEEVENTF_WHEEL, MOUSEINPUT,
    VIRTUAL_KEY, VK_BACK, VK_CONTROL, VK_DELETE, VK_DOWN, VK_END, VK_ESCAPE, VK_HOME, VK_INSERT,
    VK_LEFT, VK_LWIN, VK_MEDIA_NEXT_TRACK, VK_MEDIA_PLAY_PAUSE, VK_MEDIA_PREV_TRACK, VK_MENU,
    VK_NEXT, VK_OEM_1, VK_OEM_2, VK_OEM_3, VK_OEM_4, VK_OEM_5, VK_OEM_6, VK_OEM_7, VK_OEM_COMMA,
    VK_OEM_MINUS, VK_OEM_PERIOD, VK_OEM_PLUS, VK_PRIOR, VK_RETURN, VK_RIGHT, VK_SHIFT, VK_SPACE,
    VK_TAB, VK_UP, VK_VOLUME_DOWN, VK_VOLUME_MUTE, VK_VOLUME_UP,
};
use windows::Win32::UI::WindowsAndMessaging::WHEEL_DELTA;

fn vk_for(key: Key) -> (VIRTUAL_KEY, bool) {
    // (virtual key, is extended key)
    match key {
        Key::Letter(c) => (VIRTUAL_KEY(c as u16), false),
        Key::Digit(d) => (VIRTUAL_KEY(b'0' as u16 + d as u16), false),
        Key::Function(n) => (VIRTUAL_KEY(0x70 + n as u16 - 1), false),
        Key::Tab => (VK_TAB, false),
        Key::Enter => (VK_RETURN, false),
        Key::Escape => (VK_ESCAPE, false),
        Key::Space => (VK_SPACE, false),
        Key::Backspace => (VK_BACK, false),
        Key::Delete => (VK_DELETE, true),
        Key::Insert => (VK_INSERT, true),
        Key::Home => (VK_HOME, true),
        Key::End => (VK_END, true),
        Key::PageUp => (VK_PRIOR, true),
        Key::PageDown => (VK_NEXT, true),
        Key::Left => (VK_LEFT, true),
        Key::Right => (VK_RIGHT, true),
        Key::Up => (VK_UP, true),
        Key::Down => (VK_DOWN, true),
        Key::Minus => (VK_OEM_MINUS, false),
        Key::Equals => (VK_OEM_PLUS, false),
        Key::LeftBracket => (VK_OEM_4, false),
        Key::RightBracket => (VK_OEM_6, false),
        Key::Backslash => (VK_OEM_5, false),
        Key::Semicolon => (VK_OEM_1, false),
        Key::Apostrophe => (VK_OEM_7, false),
        Key::Comma => (VK_OEM_COMMA, false),
        Key::Period => (VK_OEM_PERIOD, false),
        Key::Slash => (VK_OEM_2, false),
        Key::Grave => (VK_OEM_3, false),
    }
}

fn key_input(vk: VIRTUAL_KEY, extended: bool, up: bool) -> INPUT {
    let mut flags = KEYBD_EVENT_FLAGS(0);
    if extended {
        flags |= KEYEVENTF_EXTENDEDKEY;
    }
    if up {
        flags |= KEYEVENTF_KEYUP;
    }
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

fn wheel_input(horizontal: bool, delta: i32) -> INPUT {
    INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx: 0,
                dy: 0,
                mouseData: delta as u32,
                dwFlags: if horizontal {
                    MOUSEEVENTF_HWHEEL
                } else {
                    MOUSEEVENTF_WHEEL
                },
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

fn send(inputs: &[INPUT]) -> Result<(), PlatformError> {
    // SAFETY: inputs is a valid slice; cbSize matches the struct.
    let sent = unsafe { SendInput(inputs, std::mem::size_of::<INPUT>() as i32) };
    if sent as usize != inputs.len() {
        return Err(PlatformError::Os(format!(
            "SendInput sent {sent} of {} events",
            inputs.len()
        )));
    }
    Ok(())
}

fn parse(chord: &KeyChord) -> Result<ParsedChord, PlatformError> {
    let parsed: ParsedChord = chord
        .0
        .parse()
        .map_err(|e| PlatformError::Os(format!("bad chord `{}`: {e}", chord.0)))?;
    if parsed.mods.fn_key {
        // Fn is a macOS modifier; Windows has nothing to send it to.
        return Err(PlatformError::Unsupported);
    }
    Ok(parsed)
}

fn media_vk(key: MediaKey) -> Option<VIRTUAL_KEY> {
    Some(match key {
        MediaKey::VolumeUp => VK_VOLUME_UP,
        MediaKey::VolumeDown => VK_VOLUME_DOWN,
        MediaKey::Mute => VK_VOLUME_MUTE,
        MediaKey::PlayPause => VK_MEDIA_PLAY_PAUSE,
        MediaKey::NextTrack => VK_MEDIA_NEXT_TRACK,
        MediaKey::PreviousTrack => VK_MEDIA_PREV_TRACK,
        // No virtual key on Windows; needs the WMI/monitor API later.
        MediaKey::BrightnessUp | MediaKey::BrightnessDown => return None,
    })
}

/// Virtual keys for a modifier set, in press order.
fn mod_vks(m: Modifiers) -> Vec<VIRTUAL_KEY> {
    let mut v = Vec::with_capacity(4);
    if m.ctrl {
        v.push(VK_CONTROL);
    }
    if m.shift {
        v.push(VK_SHIFT);
    }
    if m.alt {
        v.push(VK_MENU);
    }
    if m.meta {
        v.push(VK_LWIN);
    }
    v
}

/// Modifiers in `a` that are not in `b`.
fn minus(a: Modifiers, b: Modifiers) -> Modifiers {
    Modifiers {
        ctrl: a.ctrl && !b.ctrl,
        shift: a.shift && !b.shift,
        alt: a.alt && !b.alt,
        meta: a.meta && !b.meta,
        fn_key: false,
    }
}

#[derive(Debug, Default)]
pub struct SendInputSink {
    /// Modifiers a holding `Keys` action left down.
    held: Modifiers,
}

impl SendInputSink {
    /// One chord on top of whatever is held. A holding chord swaps the held
    /// set for its own modifiers and leaves them down; a plain chord presses
    /// only the modifiers not already held and releases just those.
    fn chord(
        &mut self,
        parsed: &ParsedChord,
        repeat: u32,
        hold: bool,
    ) -> Result<(), PlatformError> {
        let mut v: Vec<INPUT> = Vec::new();
        if hold {
            // Held modifiers this chord does not use go up first (Alt+Shift+Tab -> Alt+Tab).
            for vk in mod_vks(minus(self.held, parsed.mods)) {
                v.push(key_input(vk, false, true));
            }
        }
        let fresh = minus(parsed.mods, self.held);
        let fresh_vks = mod_vks(fresh);
        for vk in &fresh_vks {
            v.push(key_input(*vk, false, false));
        }
        let (vk, ext) = vk_for(parsed.key);
        for _ in 0..repeat {
            v.push(key_input(vk, ext, false));
            v.push(key_input(vk, ext, true));
        }
        if hold {
            self.held = Modifiers {
                fn_key: false,
                ..parsed.mods
            };
        } else {
            for vk in fresh_vks.iter().rev() {
                v.push(key_input(*vk, false, true));
            }
        }
        send(&v)
    }
}

impl ActionSink for SendInputSink {
    fn execute(&mut self, action: &Action, repeat: u32) -> Result<(), PlatformError> {
        let repeat = repeat.max(1);
        match action {
            Action::Noop => Ok(()),
            Action::Release => self.release_held(),
            Action::Keys { chord, hold_ms } => {
                let parsed = parse(chord)?;
                self.chord(&parsed, repeat, hold_ms.is_some())
            }
            Action::Sequence { chords } => {
                for c in chords {
                    self.chord(&parse(c)?, 1, false)?;
                }
                Ok(())
            }
            Action::Media { key } => {
                let vk = media_vk(*key).ok_or(PlatformError::Unsupported)?;
                let mut v = Vec::with_capacity(2 * repeat as usize);
                for _ in 0..repeat {
                    v.push(key_input(vk, false, false));
                    v.push(key_input(vk, false, true));
                }
                send(&v)
            }
            Action::Scroll { direction, lines } => {
                let magnitude = (WHEEL_DELTA as i32) * (*lines as i32) * repeat as i32;
                let (horizontal, delta) = match direction {
                    ScrollDirection::Up => (false, magnitude),
                    ScrollDirection::Down => (false, -magnitude),
                    ScrollDirection::Right => (true, magnitude),
                    ScrollDirection::Left => (true, -magnitude),
                };
                send(&[wheel_input(horizontal, delta)])
            }
            Action::Launch { program, args } => std::process::Command::new(program)
                .args(args)
                .spawn()
                .map(|_| ())
                .map_err(|e| PlatformError::Os(format!("launch `{program}`: {e}"))),
            Action::Command { command } => std::process::Command::new("cmd")
                .args(["/C", command])
                .spawn()
                .map(|_| ())
                .map_err(|e| PlatformError::Os(format!("command `{command}`: {e}"))),
        }
    }

    fn release_modifiers(&mut self, mods: Modifiers) -> Result<(), PlatformError> {
        // Never let go of a modifier a holding chord is keeping down on purpose.
        let vks = mod_vks(minus(mods, self.held));
        if vks.is_empty() {
            return Ok(());
        }
        let ups: Vec<INPUT> = vks.iter().map(|vk| key_input(*vk, false, true)).collect();
        send(&ups)
    }

    fn release_held(&mut self) -> Result<(), PlatformError> {
        let vks = mod_vks(self.held);
        self.held = Modifiers::NONE;
        if vks.is_empty() {
            return Ok(());
        }
        let ups: Vec<INPUT> = vks
            .iter()
            .rev()
            .map(|vk| key_input(*vk, false, true))
            .collect();
        send(&ups)
    }
}
