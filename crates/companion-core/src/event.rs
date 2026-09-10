//! Semantic events: what the user physically did, independent of how the
//! keyboard encoded it on the wire.
//!
//! The string form is the stable identifier used in config files:
//!
//! ```text
//! <MODULE>_<GESTURE>[_<n>F]
//! TUNE_CW              dial, never has a finger count
//! TUNE_TAP_1F          one-finger tap (the Tune "press")
//! LEFT_TOUCH_SWIPE_UP_3F
//! TUNE_SWIPE_LEFT      no finger count: matches as an "any count" default
//! ```
//!
//! This mirrors the module profile's gesture vocabulary (`swipe_left:tune:3_fingers`,
//! `rotate:tune:dial` split into `-`/`+`), see [`SemanticEvent::module_behavior`].
//! Never persist the enum discriminant.

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

impl Module {
    pub const ALL: [Module; 3] = [Module::Tune, Module::LeftTouch, Module::RightTouch];

    fn prefix(self) -> &'static str {
        match self {
            Module::Tune => "TUNE",
            Module::LeftTouch => "LEFT_TOUCH",
            Module::RightTouch => "RIGHT_TOUCH",
        }
    }

    /// The module token inside a module-profile behavior string.
    pub fn module_token(self) -> &'static str {
        match self {
            Module::Tune => "tune",
            Module::LeftTouch | Module::RightTouch => "touch",
        }
    }

    /// `moduleType` for an OpenFlow module profile.
    pub fn module_type_token(self) -> &'static str {
        match self {
            Module::Tune => "TUNE",
            Module::LeftTouch | Module::RightTouch => "TOUCH",
        }
    }
}

/// A discrete gesture the firmware can emit as a HID keypress.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Gesture {
    /// Dial rotated clockwise (`rotate:tune:dial` split `+`). Tune only.
    Cw,
    /// Dial rotated counterclockwise (`rotate:tune:dial` split `-`). Tune only.
    Ccw,
    Tap,
    DoubleTap,
    SwipeLeft,
    SwipeRight,
    SwipeUp,
    SwipeDown,
    /// The `pinch&spread` axis split into keys: `-` is pinch, `+` is spread.
    Pinch,
    Spread,
    /// The `horizontal` axis split into keys: `-` is left, `+` is right.
    ScrollLeft,
    ScrollRight,
    /// The `vertical` axis split into keys: `-` is up, `+` is down.
    ScrollUp,
    ScrollDown,
}

impl Gesture {
    pub const ALL: [Gesture; 14] = [
        Gesture::Cw,
        Gesture::Ccw,
        Gesture::Tap,
        Gesture::DoubleTap,
        Gesture::SwipeLeft,
        Gesture::SwipeRight,
        Gesture::SwipeUp,
        Gesture::SwipeDown,
        Gesture::Pinch,
        Gesture::Spread,
        Gesture::ScrollLeft,
        Gesture::ScrollRight,
        Gesture::ScrollUp,
        Gesture::ScrollDown,
    ];

    fn token(self) -> &'static str {
        match self {
            Gesture::Cw => "CW",
            Gesture::Ccw => "CCW",
            Gesture::Tap => "TAP",
            Gesture::DoubleTap => "DOUBLE_TAP",
            Gesture::SwipeLeft => "SWIPE_LEFT",
            Gesture::SwipeRight => "SWIPE_RIGHT",
            Gesture::SwipeUp => "SWIPE_UP",
            Gesture::SwipeDown => "SWIPE_DOWN",
            Gesture::Pinch => "PINCH",
            Gesture::Spread => "SPREAD",
            Gesture::ScrollLeft => "SCROLL_LEFT",
            Gesture::ScrollRight => "SCROLL_RIGHT",
            Gesture::ScrollUp => "SCROLL_UP",
            Gesture::ScrollDown => "SCROLL_DOWN",
        }
    }

    fn from_token(s: &str) -> Option<Self> {
        Some(match s {
            "CW" => Gesture::Cw,
            "CCW" => Gesture::Ccw,
            "TAP" | "PRESS" => Gesture::Tap,
            "DOUBLE_TAP" => Gesture::DoubleTap,
            "SWIPE_LEFT" => Gesture::SwipeLeft,
            "SWIPE_RIGHT" => Gesture::SwipeRight,
            "SWIPE_UP" => Gesture::SwipeUp,
            "SWIPE_DOWN" => Gesture::SwipeDown,
            "PINCH" => Gesture::Pinch,
            "SPREAD" => Gesture::Spread,
            "SCROLL_LEFT" => Gesture::ScrollLeft,
            "SCROLL_RIGHT" => Gesture::ScrollRight,
            "SCROLL_UP" => Gesture::ScrollUp,
            "SCROLL_DOWN" => Gesture::ScrollDown,
            _ => return None,
        })
    }

    /// Dial rotation carries no finger count.
    pub fn takes_fingers(self) -> bool {
        !matches!(self, Gesture::Cw | Gesture::Ccw)
    }

    /// Only the Tune has a dial.
    pub fn available_on(self, module: Module) -> bool {
        module == Module::Tune || !matches!(self, Gesture::Cw | Gesture::Ccw)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SemanticEvent {
    pub module: Module,
    pub gesture: Gesture,
    /// 1–4, or `None` for "any count" (legacy names, and defaults that apply
    /// regardless of how many fingers the module was flashed for).
    pub fingers: Option<u8>,
}

impl SemanticEvent {
    pub const fn new(module: Module, gesture: Gesture) -> Self {
        Self {
            module,
            gesture,
            fingers: None,
        }
    }

    pub const fn with_fingers(module: Module, gesture: Gesture, fingers: u8) -> Self {
        Self {
            module,
            gesture,
            fingers: Some(fingers),
        }
    }

    /// Whether the module emits this gesture as a run of keys scaled to the
    /// finger travel rather than one key per gesture.
    ///
    /// Tune (measured 2026-09-05): two-finger swipes stream (one key per
    /// ~1/25 of the pad, at most one per touch report); one- and three-finger
    /// swipes, taps and dial detents send exactly one key.
    ///
    /// Touch (measured 2026-09-10, `tools/plans/census-touch.log`): the
    /// two-finger scroll axes stream one key per report in every direction
    /// (9–11 keys for half the pad, none while the fingers rest), and the
    /// four-finger swipes up and down stream scaled to distance (1 key for a
    /// flick, 5–6 for half the pad, 10 for the whole pad). Three-finger swipes
    /// at any speed, the four-finger swipes left and right, and every tap send
    /// exactly one key. The Touch's two-finger swipe fields have no known
    /// device slot yet; they are treated like its scroll axes, which is what a
    /// two-finger motion is on that module.
    pub fn streams(self) -> bool {
        let swipe = matches!(
            self.gesture,
            Gesture::SwipeLeft | Gesture::SwipeRight | Gesture::SwipeUp | Gesture::SwipeDown
        );
        let scroll = matches!(
            self.gesture,
            Gesture::ScrollLeft | Gesture::ScrollRight | Gesture::ScrollUp | Gesture::ScrollDown
        );
        match self.module {
            Module::Tune => swipe && self.fingers == Some(2),
            Module::LeftTouch | Module::RightTouch => {
                ((swipe || scroll) && self.fingers == Some(2))
                    || (matches!(self.gesture, Gesture::SwipeUp | Gesture::SwipeDown)
                        && self.fingers == Some(4))
            }
        }
    }

    /// The same gesture with no finger count, used as a lookup fallback.
    pub fn without_fingers(self) -> Self {
        Self {
            fingers: None,
            ..self
        }
    }

    /// The module-profile behavior string for this event, plus which half of a direction
    /// pair it is (`Some('-')`, `Some('+')`) when the gesture is one half of
    /// an axis or the dial. `None` when a finger count is required but missing.
    ///
    /// ```text
    /// TUNE_TAP_1F             -> ("tap:tune:1_finger", None)
    /// TUNE_CW                 -> ("rotate:tune:dial", Some('+'))
    /// LEFT_TOUCH_SCROLL_UP_2F -> ("vertical:touch:2_fingers", Some('-'))
    /// ```
    pub fn module_behavior(self) -> Option<(String, Option<char>)> {
        let m = self.module.module_token();
        if matches!(self.gesture, Gesture::Cw | Gesture::Ccw) {
            let half = if self.gesture == Gesture::Cw {
                '+'
            } else {
                '-'
            };
            return Some((format!("rotate:{m}:dial"), Some(half)));
        }
        let n = self.fingers?;
        let q = if n == 1 {
            "1_finger".to_string()
        } else {
            format!("{n}_fingers")
        };
        let (g, half) = match self.gesture {
            Gesture::Tap => ("tap", None),
            Gesture::DoubleTap => ("double_tap", None),
            Gesture::SwipeLeft => ("swipe_left", None),
            Gesture::SwipeRight => ("swipe_right", None),
            Gesture::SwipeUp => ("swipe_up", None),
            Gesture::SwipeDown => ("swipe_down", None),
            Gesture::Pinch => ("pinch&spread", Some('-')),
            Gesture::Spread => ("pinch&spread", Some('+')),
            Gesture::ScrollLeft => ("horizontal", Some('-')),
            Gesture::ScrollRight => ("horizontal", Some('+')),
            Gesture::ScrollUp => ("vertical", Some('-')),
            Gesture::ScrollDown => ("vertical", Some('+')),
            Gesture::Cw | Gesture::Ccw => unreachable!(),
        };
        Some((format!("{g}:{m}:{q}"), half))
    }
}

impl fmt::Display for SemanticEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}_{}", self.module.prefix(), self.gesture.token())?;
        if let Some(n) = self.fingers {
            write!(f, "_{n}F")?;
        }
        Ok(())
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("unknown semantic event `{0}`")]
pub struct ParseEventError(pub String);

impl FromStr for SemanticEvent {
    type Err = ParseEventError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let err = || ParseEventError(s.to_owned());
        let (module, rest) = Module::ALL
            .iter()
            .find_map(|m| {
                s.strip_prefix(m.prefix())
                    .and_then(|r| r.strip_prefix('_'))
                    .map(|r| (*m, r))
            })
            .ok_or_else(err)?;
        // Optional `_<n>F` suffix.
        let (gesture_tok, fingers) = match rest.rsplit_once('_') {
            Some((g, suf))
                if suf.len() == 2 && suf.ends_with('F') && suf.as_bytes()[0].is_ascii_digit() =>
            {
                let n = suf.as_bytes()[0] - b'0';
                if !(1..=4).contains(&n) {
                    return Err(err());
                }
                (g, Some(n))
            }
            _ => (rest, None),
        };
        let mut gesture = Gesture::from_token(gesture_tok).ok_or_else(err)?;
        let mut fingers = fingers;
        // Legacy: PRESS is the one-finger tap.
        if gesture_tok == "PRESS" {
            gesture = Gesture::Tap;
            fingers = Some(1);
        }
        if fingers.is_some() && !gesture.takes_fingers() {
            return Err(err());
        }
        if !gesture.available_on(module) {
            return Err(err());
        }
        Ok(SemanticEvent {
            module,
            gesture,
            fingers,
        })
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
        for (ev, expected) in [
            (SemanticEvent::new(Module::Tune, Gesture::Cw), "TUNE_CW"),
            (
                SemanticEvent::with_fingers(Module::Tune, Gesture::Tap, 1),
                "TUNE_TAP_1F",
            ),
            (
                SemanticEvent::new(Module::Tune, Gesture::SwipeLeft),
                "TUNE_SWIPE_LEFT",
            ),
            (
                SemanticEvent::with_fingers(Module::LeftTouch, Gesture::SwipeUp, 3),
                "LEFT_TOUCH_SWIPE_UP_3F",
            ),
            (
                SemanticEvent::with_fingers(Module::RightTouch, Gesture::DoubleTap, 2),
                "RIGHT_TOUCH_DOUBLE_TAP_2F",
            ),
            (
                SemanticEvent::with_fingers(Module::Tune, Gesture::ScrollUp, 1),
                "TUNE_SCROLL_UP_1F",
            ),
        ] {
            assert_eq!(ev.to_string(), expected);
            assert_eq!(expected.parse::<SemanticEvent>().unwrap(), ev);
        }
    }

    #[test]
    fn legacy_press_is_one_finger_tap() {
        let ev: SemanticEvent = "TUNE_PRESS".parse().unwrap();
        assert_eq!(
            ev,
            SemanticEvent::with_fingers(Module::Tune, Gesture::Tap, 1)
        );
        assert_eq!(ev.to_string(), "TUNE_TAP_1F");
    }

    #[test]
    fn rejects_unknown_and_invalid() {
        for bad in [
            "TUNE_WIGGLE",
            "FOOT_CW",
            "TUNE_CW_2F",
            "LEFT_TOUCH_CW",
            "TUNE_TAP_5F",
            "TUNE_TAP_0F",
        ] {
            assert!(bad.parse::<SemanticEvent>().is_err(), "{bad}");
        }
    }

    #[test]
    fn streams_per_module() {
        let s = |e: &str| e.parse::<SemanticEvent>().unwrap().streams();
        // Tune: two-finger swipes only.
        for e in ["TUNE_SWIPE_UP_2F", "TUNE_SWIPE_LEFT_2F"] {
            assert!(s(e), "{e}");
        }
        for e in [
            "TUNE_SWIPE_UP_1F",
            "TUNE_SWIPE_UP_3F",
            "TUNE_TAP_2F",
            "TUNE_CW",
        ] {
            assert!(!s(e), "{e}");
        }
        // Touch: two-finger motion in any direction, and the four-finger swipes up and down.
        for e in [
            "LEFT_TOUCH_SCROLL_UP_2F",
            "LEFT_TOUCH_SCROLL_LEFT_2F",
            "RIGHT_TOUCH_SWIPE_DOWN_2F",
            "LEFT_TOUCH_SWIPE_UP_4F",
            "RIGHT_TOUCH_SWIPE_DOWN_4F",
        ] {
            assert!(s(e), "{e}");
        }
        for e in [
            "LEFT_TOUCH_SWIPE_UP_3F",
            "LEFT_TOUCH_SWIPE_LEFT_4F",
            "LEFT_TOUCH_SWIPE_RIGHT_4F",
            "LEFT_TOUCH_TAP_4F",
            "LEFT_TOUCH_SCROLL_UP_1F",
            "LEFT_TOUCH_PINCH_2F",
            "LEFT_TOUCH_SWIPE_UP",
        ] {
            assert!(!s(e), "{e}");
        }
    }

    #[test]
    fn module_behaviors() {
        let b = |s: &str| s.parse::<SemanticEvent>().unwrap().module_behavior();
        assert_eq!(b("TUNE_TAP_1F"), Some(("tap:tune:1_finger".into(), None)));
        assert_eq!(
            b("TUNE_SWIPE_LEFT_3F"),
            Some(("swipe_left:tune:3_fingers".into(), None))
        );
        assert_eq!(b("TUNE_CW"), Some(("rotate:tune:dial".into(), Some('+'))));
        assert_eq!(b("TUNE_CCW"), Some(("rotate:tune:dial".into(), Some('-'))));
        assert_eq!(
            b("LEFT_TOUCH_SCROLL_UP_2F"),
            Some(("vertical:touch:2_fingers".into(), Some('-')))
        );
        assert_eq!(
            b("RIGHT_TOUCH_PINCH_2F"),
            Some(("pinch&spread:touch:2_fingers".into(), Some('-')))
        );
        assert_eq!(
            b("TUNE_SPREAD_3F"),
            Some(("pinch&spread:tune:3_fingers".into(), Some('+')))
        );
        assert_eq!(b("TUNE_SWIPE_LEFT"), None, "finger count required");
    }
}
