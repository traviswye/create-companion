//! Platform-independent key chord parsing. `"Ctrl+Shift+Tab"` becomes a
//! [`ParsedChord`]; the platform layer maps [`Key`] to virtual-key codes.

use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ModifierSet {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    /// Windows key / Command key.
    pub meta: bool,
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
            match tok.to_ascii_uppercase().as_str() {
                "" => continue,
                "CTRL" | "CONTROL" => mods.ctrl = true,
                "SHIFT" => mods.shift = true,
                "ALT" | "OPTION" | "OPT" => mods.alt = true,
                "WIN" | "META" | "CMD" | "COMMAND" | "SUPER" => mods.meta = true,
                _ => {
                    let k = parse_key(tok).ok_or_else(|| ChordError::UnknownKey(tok.to_owned()))?;
                    if key.replace(k).is_some() {
                        return Err(ChordError::MultipleKeys(s.to_owned()));
                    }
                }
            }
        }
        let key = key.ok_or_else(|| ChordError::NoKey(s.to_owned()))?;
        Ok(ParsedChord { mods, key })
    }
}

impl fmt::Display for ParsedChord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.mods.ctrl {
            f.write_str("Ctrl+")?;
        }
        if self.mods.shift {
            f.write_str("Shift+")?;
        }
        if self.mods.alt {
            f.write_str("Alt+")?;
        }
        if self.mods.meta {
            f.write_str("Win+")?;
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
        for s in ["Ctrl+Shift+Tab", "Alt+Left", "Win+D", "F13"] {
            let c: ParsedChord = s.parse().unwrap();
            assert_eq!(c.to_string(), s);
        }
    }
}
