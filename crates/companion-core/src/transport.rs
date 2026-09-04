//! Transport layer: the F-key (+ modifier namespace) the firmware emits for
//! each gesture. `F24` is *not* "volume up"; `F24` is "Tune clockwise".
//!
//! The [`TransportTable`] is built from config and consulted by the OS hook.
//! Only keys present in the table are swallowed (scope §17: never globally
//! eat F17–F24 unless they are configured as Naya transport).

use crate::event::SemanticEvent;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The function keys usable as transport. HID usage IDs F13–F24 = 0x68–0x73.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[allow(clippy::upper_case_acronyms)]
pub enum FunctionKey {
    F13,
    F14,
    F15,
    F16,
    F17,
    F18,
    F19,
    F20,
    F21,
    F22,
    F23,
    F24,
}

impl FunctionKey {
    /// USB HID keyboard usage ID (page 0x07).
    pub const fn hid_usage(self) -> u8 {
        0x68 + self as u8
    }

    /// Windows virtual-key code (`VK_F13` = 0x7C ... `VK_F24` = 0x87).
    pub const fn windows_vk(self) -> u16 {
        0x7C + self as u16
    }

    pub const fn from_windows_vk(vk: u16) -> Option<Self> {
        Some(match vk {
            0x7C => Self::F13,
            0x7D => Self::F14,
            0x7E => Self::F15,
            0x7F => Self::F16,
            0x80 => Self::F17,
            0x81 => Self::F18,
            0x82 => Self::F19,
            0x83 => Self::F20,
            0x84 => Self::F21,
            0x85 => Self::F22,
            0x86 => Self::F23,
            0x87 => Self::F24,
            _ => return None,
        })
    }
}

/// Modifier namespace held while the F-key arrives. Used to multiplex
/// several modules onto the same eight keys (scope §5, "Multiple Modules").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Modifiers {
    #[default]
    None,
    Shift,
    Ctrl,
    Alt,
    CtrlShift,
}

/// One wire-level code as seen by the hook.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TransportCode {
    pub key: FunctionKey,
    #[serde(default)]
    pub mods: Modifiers,
}

impl TransportCode {
    pub const fn plain(key: FunctionKey) -> Self {
        Self {
            key,
            mods: Modifiers::None,
        }
    }
}

/// Wire code -> semantic event. Also answers the hook's hot-path question
/// "is this F-key reserved at all?" without a hash lookup.
#[derive(Debug, Clone, Default)]
pub struct TransportTable {
    map: HashMap<TransportCode, SemanticEvent>,
    reserved: [bool; 12],
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum TransportError {
    #[error("transport code {0:?} is assigned to both {1} and {2}")]
    Duplicate(TransportCode, SemanticEvent, SemanticEvent),
}

impl TransportTable {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an assignment. Errors on a duplicate wire code (scope Phase 4:
    /// conflict detection), so a config that maps two gestures to `F19` is
    /// rejected up-front rather than silently last-writer-wins.
    pub fn insert(
        &mut self,
        code: TransportCode,
        event: SemanticEvent,
    ) -> Result<(), TransportError> {
        if let Some(existing) = self.map.get(&code) {
            if *existing != event {
                return Err(TransportError::Duplicate(code, *existing, event));
            }
            return Ok(());
        }
        self.map.insert(code, event);
        self.reserved[code.key as usize] = true;
        Ok(())
    }

    pub fn decode(&self, code: TransportCode) -> Option<SemanticEvent> {
        self.map.get(&code).copied()
    }

    /// True if any namespace uses this key. The hook uses this to decide
    /// whether the key is ours; an unreserved F-key passes through untouched.
    pub fn is_reserved(&self, key: FunctionKey) -> bool {
        self.reserved[key as usize]
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{Gesture, Module};

    #[test]
    fn key_codes_line_up() {
        assert_eq!(FunctionKey::F17.hid_usage(), 0x6C);
        assert_eq!(FunctionKey::F24.hid_usage(), 0x73);
        assert_eq!(FunctionKey::F24.windows_vk(), 0x87);
        assert_eq!(FunctionKey::from_windows_vk(0x86), Some(FunctionKey::F23));
        assert_eq!(FunctionKey::from_windows_vk(0x70), None); // F1
    }

    #[test]
    fn decodes_and_tracks_reservation() {
        let mut t = TransportTable::new();
        let cw = SemanticEvent::new(Module::Tune, Gesture::Cw);
        t.insert(TransportCode::plain(FunctionKey::F24), cw)
            .unwrap();
        assert_eq!(t.decode(TransportCode::plain(FunctionKey::F24)), Some(cw));
        assert!(t.is_reserved(FunctionKey::F24));
        assert!(!t.is_reserved(FunctionKey::F17));
        assert_eq!(t.decode(TransportCode::plain(FunctionKey::F17)), None);
    }

    #[test]
    fn modifier_namespaces_are_distinct() {
        let mut t = TransportTable::new();
        let tune = SemanticEvent::new(Module::Tune, Gesture::SwipeLeft);
        let left = SemanticEvent::new(Module::LeftTouch, Gesture::SwipeLeft);
        t.insert(TransportCode::plain(FunctionKey::F20), tune)
            .unwrap();
        t.insert(
            TransportCode {
                key: FunctionKey::F20,
                mods: Modifiers::Shift,
            },
            left,
        )
        .unwrap();
        assert_eq!(t.len(), 2);
    }

    #[test]
    fn duplicate_code_is_an_error() {
        let mut t = TransportTable::new();
        let cw = SemanticEvent::new(Module::Tune, Gesture::Cw);
        let ccw = SemanticEvent::new(Module::Tune, Gesture::Ccw);
        t.insert(TransportCode::plain(FunctionKey::F24), cw)
            .unwrap();
        let err = t
            .insert(TransportCode::plain(FunctionKey::F24), ccw)
            .unwrap_err();
        assert!(matches!(err, TransportError::Duplicate(..)));
    }
}
