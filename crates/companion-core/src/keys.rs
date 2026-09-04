//! Platform-independent key chord parsing. `"Ctrl+Shift+Tab"` becomes a
//! [`ParsedChord`]; the platform layer maps [`Key`] to virtual-key codes.
//!
//! [`ModifierSet`] doubles as the transport namespace a module's key carries
//! (`Cmd+F13` from a Touch flashed for macOS) and as the modifiers of an
//! action chord. `Fn` is a real modifier on macOS (the Globe key) and can be
//! sent to apps there; as a transport namespace it depends on the firmware
//! emitting the Apple vendor-page Fn usage (unverified). Windows has no Fn key.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct ModifierSet {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    /// Windows key / Command key.
    pub meta: bool,
    /// macOS Fn / Globe key. Actions only; never a transport namespace.
    pub fn_key: bool,
}

impl ModifierSet {
    pub const NONE: Self = Self {
        ctrl: false,
        shift: false,
        alt: false,
        meta: false,
        fn_key: false,
    };
    pub const SHIFT: Self = Self {
        shift: true,
        ..Self::NONE
    };
    pub const CTRL: Self = Self {
        ctrl: true,
        ..Self::NONE
    };
    pub const ALT: Self = Self {
        alt: true,
        ..Self::NONE
    };
    pub const CMD: Self = Self {
        meta: true,
        ..Self::NONE
    };
    pub const FN: Self = Self {
        fn_key: true,
        ..Self::NONE
    };
    pub const CTRL_SHIFT: Self = Self {
        ctrl: true,
        shift: true,
        ..Self::NONE
    };

    pub fn is_empty(&self) -> bool {
        *self == Self::NONE
    }

    /// Fn only exists on macOS hosts (and only if the firmware emits the
    /// Apple vendor-page Fn usage). Windows never sees it.
    pub fn requires_macos(&self) -> bool {
        self.fn_key
    }

    /// Canonical token names in display order: Ctrl, Shift, Alt, Cmd, Fn.
    pub fn tokens(&self) -> Vec<&'static str> {
        let mut v = Vec::with_capacity(5);
        if self.ctrl {
            v.push("Ctrl");
        }
        if self.shift {
            v.push("Shift");
        }
        if self.alt {
            v.push("Alt");
        }
        if self.meta {
            v.push("Cmd");
        }
        if self.fn_key {
            v.push("Fn");
        }
        v
    }

    /// Apply one modifier token; returns false if it is not a modifier.
    fn apply_token(&mut self, tok: &str) -> bool {
        match tok.to_ascii_uppercase().as_str() {
            "CTRL" | "CONTROL" => self.ctrl = true,
            "SHIFT" => self.shift = true,
            "ALT" | "OPTION" | "OPT" => self.alt = true,
            "WIN" | "META" | "CMD" | "COMMAND" | "SUPER" => self.meta = true,
            "FN" | "FUNCTION" | "GLOBE" => self.fn_key = true,
            _ => return false,
        }
        true
    }
}

/// Serialized as a lowercase string: `none`, `shift`, `ctrl+shift`, `cmd`,
/// `fn+shift`. Also accepts the legacy `ctrl_shift` spelling.
impl fmt::Display for ModifierSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_empty() {
            return f.write_str("none");
        }
        let lower: Vec<String> = self.tokens().iter().map(|t| t.to_lowercase()).collect();
        f.write_str(&lower.join("+"))
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("unknown modifier set `{0}`")]
pub struct ParseModifiersError(pub String);

impl FromStr for ModifierSet {
    type Err = ParseModifiersError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if s.is_empty() || s.eq_ignore_ascii_case("none") {
            return Ok(Self::NONE);
        }
        let mut set = Self::NONE;
        for tok in s.split(['+', '_', ' ']) {
            let tok = tok.trim();
            if tok.is_empty() {
                continue;
            }
            if !set.apply_token(tok) {
                return Err(ParseModifiersError(s.to_owned()));
            }
        }
        Ok(set)
    }
}

impl Serialize for ModifierSet {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for ModifierSet {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    /// `A`..`Z`, stored uppercase.
    Letter(u8),
    /// `0`..`9`.
    Digit(u8),
    /// `F1`..`F24`.
    Function(u8),
    Tab,
    Enter,
    Escape,
    Space,
    Backspace,
    Delete,
    Insert,
    Home,
    End,
    PageUp,
    PageDown,
    Left,
    Right,
    Up,
    Down,
    Minus,
    Equals,
    LeftBracket,
    RightBracket,
    Backslash,
    Semicolon,
    Apostrophe,
    Comma,
    Period,
    Slash,
    Grave,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParsedChord {
    pub mods: ModifierSet,
    pub key: Key,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ChordError {
    #[error("empty chord")]
    Empty,
    #[error("unknown key `{0}`")]
    UnknownKey(String),
    #[error("chord `{0}` has more than one non-modifier key")]
    MultipleKeys(String),
    #[error("chord `{0}` has only modifiers")]
    NoKey(String),
}

fn parse_key(tok: &str) -> Option<Key> {
    let up = tok.to_ascii_uppercase();
    let b = up.as_bytes();
    if b.len() == 1 {
        return match b[0] {
            c @ b'A'..=b'Z' => Some(Key::Letter(c)),
            c @ b'0'..=b'9' => Some(Key::Digit(c - b'0')),
            b'-' => Some(Key::Minus),
            b'=' => Some(Key::Equals),
            b'[' => Some(Key::LeftBracket),
            b']' => Some(Key::RightBracket),
            b'\\' => Some(Key::Backslash),
            b';' => Some(Key::Semicolon),
            b'\'' => Some(Key::Apostrophe),
            b',' => Some(Key::Comma),
            b'.' => Some(Key::Period),
            b'/' => Some(Key::Slash),
            b'`' => Some(Key::Grave),
            _ => None,
        };
    }
    if let Some(n) = up.strip_prefix('F') {
        if let Ok(n) = n.parse::<u8>() {
            if (1..=24).contains(&n) {
                return Some(Key::Function(n));
            }
        }
    }
    Some(match up.as_str() {
        "TAB" => Key::Tab,
        "ENTER" | "RETURN" => Key::Enter,
        "ESC" | "ESCAPE" => Key::Escape,
        "SPACE" => Key::Space,
        "BACKSPACE" | "BKSP" => Key::Backspace,
        "DELETE" | "DEL" => Key::Delete,
        "INSERT" | "INS" => Key::Insert,
        "HOME" => Key::Home,
        "END" => Key::End,
        "PAGEUP" | "PGUP" => Key::PageUp,
        "PAGEDOWN" | "PGDN" => Key::PageDown,
        "LEFT" => Key::Left,
        "RIGHT" => Key::Right,
        "UP" => Key::Up,
        "DOWN" => Key::Down,
        "MINUS" => Key::Minus,
        "EQUALS" | "EQUAL" | "PLUS" => Key::Equals,
        "COMMA" => Key::Comma,
        "PERIOD" => Key::Period,
        "SLASH" => Key::Slash,
        "BACKSLASH" => Key::Backslash,
        "SEMICOLON" => Key::Semicolon,
        "APOSTROPHE" | "QUOTE" => Key::Apostrophe,
        "GRAVE" | "BACKTICK" => Key::Grave,
        "LEFTBRACKET" | "LBRACKET" => Key::LeftBracket,
        "RIGHTBRACKET" | "RBRACKET" => Key::RightBracket,
        _ => return None,
    })
}

impl FromStr for ParsedChord {
    type Err = ChordError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if s.is_empty() {
            return Err(ChordError::Empty);
        }
        let mut mods = ModifierSet::default();
        let mut key: Option<Key> = None;
        // Split on '+' but allow a literal '+' key as the final token ("Ctrl++").
        let mut tokens: Vec<&str> = s.split('+').collect();
        if tokens.len() >= 2
            && tokens[tokens.len() - 1].is_empty()
            && tokens[tokens.len() - 2].is_empty()
        {
            tokens.truncate(tokens.len() - 2);
            tokens.push("PLUS");
        }
        for tok in tokens {
            let tok = tok.trim();
            if tok.is_empty() {
                continue;
            }
            if mods.apply_token(tok) {
                continue;
            }
            let k = parse_key(tok).ok_or_else(|| ChordError::UnknownKey(tok.to_owned()))?;
            if key.replace(k).is_some() {
                return Err(ChordError::MultipleKeys(s.to_owned()));
            }
        }
        let key = key.ok_or_else(|| ChordError::NoKey(s.to_owned()))?;
        Ok(ParsedChord { mods, key })
    }
}

impl fmt::Display for ParsedChord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for t in self.mods.tokens() {
            // Keep the historical spelling of the meta key on chords.
            f.write_str(if t == "Cmd" { "Win" } else { t })?;
            f.write_str("+")?;
        }
        match self.key {
            Key::Letter(c) => write!(f, "{}", c as char),
            Key::Digit(d) => write!(f, "{d}"),
            Key::Function(n) => write!(f, "F{n}"),
            other => write!(f, "{other:?}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_common_chords() {
        let c: ParsedChord = "Ctrl+Shift+Tab".parse().unwrap();
        assert!(c.mods.ctrl && c.mods.shift && !c.mods.alt);
        assert_eq!(c.key, Key::Tab);

        let c: ParsedChord = "alt+left".parse().unwrap();
        assert!(c.mods.alt);
        assert_eq!(c.key, Key::Left);

        let c: ParsedChord = "b".parse().unwrap();
        assert_eq!(c.key, Key::Letter(b'B'));
        assert_eq!(c.mods, ModifierSet::default());

        let c: ParsedChord = "Ctrl+F24".parse().unwrap();
        assert_eq!(c.key, Key::Function(24));

        let c: ParsedChord = "Ctrl+]".parse().unwrap();
        assert_eq!(c.key, Key::RightBracket);

        let c: ParsedChord = "Fn+C".parse().unwrap();
        assert!(c.mods.fn_key && !c.mods.ctrl);
        assert_eq!(c.key, Key::Letter(b'C'));

        let c: ParsedChord = "Cmd+Option+Esc".parse().unwrap();
        assert!(c.mods.meta && c.mods.alt);
    }

    #[test]
    fn literal_plus_key() {
        let c: ParsedChord = "Ctrl++".parse().unwrap();
        assert!(c.mods.ctrl);
        assert_eq!(c.key, Key::Equals);
    }

    #[test]
    fn rejects_bad_input() {
        assert_eq!("".parse::<ParsedChord>(), Err(ChordError::Empty));
        assert!(matches!(
            "Ctrl+Shift".parse::<ParsedChord>(),
            Err(ChordError::NoKey(_))
        ));
        assert!(matches!(
            "Ctrl+A+B".parse::<ParsedChord>(),
            Err(ChordError::MultipleKeys(_))
        ));
        assert!(matches!(
            "Ctrl+Bogus".parse::<ParsedChord>(),
            Err(ChordError::UnknownKey(_))
        ));
        assert!("F25".parse::<ParsedChord>().is_err());
    }

    #[test]
    fn display_round_trips() {
        for s in ["Ctrl+Shift+Tab", "Alt+Left", "Win+D", "F13", "Fn+C"] {
            let c: ParsedChord = s.parse().unwrap();
            assert_eq!(c.to_string(), s);
        }
    }

    #[test]
    fn modifier_set_string_forms() {
        assert_eq!(ModifierSet::NONE.to_string(), "none");
        assert_eq!(ModifierSet::CTRL_SHIFT.to_string(), "ctrl+shift");
        assert_eq!(
            "ctrl_shift".parse::<ModifierSet>().unwrap(),
            ModifierSet::CTRL_SHIFT
        );
        assert_eq!(
            "Cmd+Shift".parse::<ModifierSet>().unwrap().to_string(),
            "shift+cmd"
        );
        assert_eq!("".parse::<ModifierSet>().unwrap(), ModifierSet::NONE);
        assert_eq!("fn".parse::<ModifierSet>().unwrap(), ModifierSet::FN);
        assert!(ModifierSet::FN.requires_macos());
        assert!(!ModifierSet::CMD.requires_macos());
        assert!("bogus".parse::<ModifierSet>().is_err());
    }
}
