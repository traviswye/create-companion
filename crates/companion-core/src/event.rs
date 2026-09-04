//! Semantic events: what the user physically did, independent of how the
//! keyboard encoded it on the wire.
//!
//! The string form (`TUNE_CW`, `LEFT_TOUCH_SWIPE_LEFT`) is the stable identifier
//! used in config files. Never persist the enum discriminant.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Which physical module produced the event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Module {
    Tune,
    LeftTouch,
    RightTouch,
}

/// A discrete gesture on a module. Mirrors the discrete subset of Naya's
/// gesture enum (`docs/reference/naya-gesture-enum.json`) that the firmware
/// can emit as a HID keypress.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Gesture {
    /// Dial rotated clockwise (Tune only).
    Cw,
    /// Dial rotated counterclockwise (Tune only).
    Ccw,
    /// One-finger tap. On Tune this is the "press".
    Press,
    Tap,
    DoubleTap,
    SwipeLeft,
    SwipeRight,
    SwipeUp,
    SwipeDown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SemanticEvent {
    pub module: Module,
    pub gesture: Gesture,
}

impl SemanticEvent {
    pub const fn new(module: Module, gesture: Gesture) -> Self {
        Self { module, gesture }
    }
}

impl fmt::Display for SemanticEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let module = match self.module {
            Module::Tune => "TUNE",
            Module::LeftTouch => "LEFT_TOUCH",
            Module::RightTouch => "RIGHT_TOUCH",
        };
        let gesture = match self.gesture {
            Gesture::Cw => "CW",
            Gesture::Ccw => "CCW",
            Gesture::Press => "PRESS",
            Gesture::Tap => "TAP",
            Gesture::DoubleTap => "DOUBLE_TAP",
            Gesture::SwipeLeft => "SWIPE_LEFT",
            Gesture::SwipeRight => "SWIPE_RIGHT",
            Gesture::SwipeUp => "SWIPE_UP",
            Gesture::SwipeDown => "SWIPE_DOWN",
        };
        write!(f, "{module}_{gesture}")
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("unknown semantic event `{0}`")]
pub struct ParseEventError(pub String);

impl FromStr for SemanticEvent {
    type Err = ParseEventError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (module, rest) = if let Some(r) = s.strip_prefix("TUNE_") {
            (Module::Tune, r)
        } else if let Some(r) = s.strip_prefix("LEFT_TOUCH_") {
            (Module::LeftTouch, r)
        } else if let Some(r) = s.strip_prefix("RIGHT_TOUCH_") {
            (Module::RightTouch, r)
        } else {
            return Err(ParseEventError(s.to_owned()));
        };
        let gesture = match rest {
            "CW" => Gesture::Cw,
            "CCW" => Gesture::Ccw,
            "PRESS" => Gesture::Press,
            "TAP" => Gesture::Tap,
            "DOUBLE_TAP" => Gesture::DoubleTap,
            "SWIPE_LEFT" => Gesture::SwipeLeft,
            "SWIPE_RIGHT" => Gesture::SwipeRight,
            "SWIPE_UP" => Gesture::SwipeUp,
            "SWIPE_DOWN" => Gesture::SwipeDown,
            _ => return Err(ParseEventError(s.to_owned())),
        };
        Ok(SemanticEvent { module, gesture })
    }
}

impl Serialize for SemanticEvent {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for SemanticEvent {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_string_form() {
        for (m, g, expected) in [
            (Module::Tune, Gesture::Cw, "TUNE_CW"),
            (Module::Tune, Gesture::Press, "TUNE_PRESS"),
            (
                Module::LeftTouch,
                Gesture::SwipeLeft,
                "LEFT_TOUCH_SWIPE_LEFT",
            ),
            (
                Module::RightTouch,
                Gesture::DoubleTap,
                "RIGHT_TOUCH_DOUBLE_TAP",
            ),
        ] {
            let ev = SemanticEvent::new(m, g);
            assert_eq!(ev.to_string(), expected);
            assert_eq!(expected.parse::<SemanticEvent>().unwrap(), ev);
        }
    }

    #[test]
    fn rejects_unknown() {
        assert!("TUNE_WIGGLE".parse::<SemanticEvent>().is_err());
        assert!("FOOT_CW".parse::<SemanticEvent>().is_err());
    }
}
