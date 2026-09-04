//! What to do when a semantic event fires in a given profile.
//!
//! Phase 1 action types per scope §10. The executor lives in
//! `companion-platform`; this is the data model only.

use serde::{Deserialize, Serialize};

/// A named key chord such as `Ctrl+Shift+Tab`. Stored as the user typed it;
/// parsed by the platform layer into virtual-key codes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyChord(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaKey {
    VolumeUp,
    VolumeDown,
    Mute,
    PlayPause,
    NextTrack,
    PreviousTrack,
    BrightnessUp,
    BrightnessDown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScrollDirection {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Action {
    /// Do nothing. Useful to explicitly silence an event in one app.
    Noop,
    /// Press and release a chord.
    Keys { chord: KeyChord },
    /// Several chords in order (e.g. `Esc`, then `Ctrl+K`).
    Sequence { chords: Vec<KeyChord> },
    /// Consumer-control / system key.
    Media { key: MediaKey },
    /// Mouse wheel; `lines` defaults to 1 and is multiplied by acceleration.
    Scroll {
        direction: ScrollDirection,
        #[serde(default = "one")]
        lines: u32,
    },
    /// Start a program (no shell).
    Launch {
        program: String,
        #[serde(default)]
        args: Vec<String>,
    },
    /// Run a shell command. Requires explicit user configuration (scope §20).
    Command { command: String },
}

fn one() -> u32 {
    1
}

impl Action {
    /// Whether repeating this action N times is meaningful. Tab switching is
    /// best one-per-detent; volume and scroll benefit from acceleration.
    pub fn supports_repeat(&self) -> bool {
        matches!(
            self,
            Action::Keys { .. } | Action::Media { .. } | Action::Scroll { .. }
        )
    }
}
