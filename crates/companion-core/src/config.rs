//! On-disk configuration (TOML). Versioned from day one; see `PLAN.md` §6.
//!
//! Transport mapping, semantic events, application matching and actions are
//! kept as separate concepts (scope §14).

use crate::event::SemanticEvent;
use crate::profile::{Profile, ProfileResolver};
use crate::transport::{TransportCode, TransportError, TransportTable};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const CURRENT_SCHEMA_VERSION: u32 = 1;

/// Engine-level settings that are not about mappings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct EngineSettings {
    /// Register the engine to start at login (HKCU Run key / LaunchAgent).
    pub start_at_login: bool,
    /// `error` | `warn` | `info` | `debug` | `trace`. `RUST_LOG` overrides.
    pub log_level: String,
}

impl Default for EngineSettings {
    fn default() -> Self {
        Self {
            start_at_login: false,
            log_level: "info".into(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "current_version")]
    pub schema_version: u32,
    #[serde(default)]
    pub engine: EngineSettings,
    /// Wire code -> semantic event. Keyed by event so each gesture appears
    /// once; duplicate codes are caught when building the table.
    #[serde(default)]
    pub transport: BTreeMap<SemanticEvent, TransportCode>,
    #[serde(default)]
    pub default_profile: Profile,
    #[serde(default)]
    pub profiles: Vec<Profile>,
}

fn current_version() -> u32 {
    CURRENT_SCHEMA_VERSION
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error(
        "config schema version {0} is newer than this build supports ({CURRENT_SCHEMA_VERSION})"
    )]
    TooNew(u32),
    #[error(transparent)]
    Transport(#[from] TransportError),
    #[error("parse error: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("serialize error: {0}")]
    Serialize(#[from] toml::ser::Error),
}

impl Config {
    pub fn from_toml(text: &str) -> Result<Self, ConfigError> {
        let cfg: Config = toml::from_str(text)?;
        if cfg.schema_version > CURRENT_SCHEMA_VERSION {
            return Err(ConfigError::TooNew(cfg.schema_version));
        }
        // Future: migrations for schema_version < CURRENT run here.
        cfg.transport_table()?; // validate duplicates eagerly
        Ok(cfg)
    }

    pub fn to_toml(&self) -> Result<String, ConfigError> {
        Ok(toml::to_string_pretty(self)?)
    }

    pub fn transport_table(&self) -> Result<TransportTable, TransportError> {
        let mut t = TransportTable::new();
        for (ev, code) in &self.transport {
            t.insert(*code, *ev)?;
        }
        Ok(t)
    }

    pub fn resolver(&self) -> ProfileResolver {
        ProfileResolver::new(self.default_profile.clone(), self.profiles.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
schema_version = 1

[transport]
TUNE_CW  = { key = "F24" }
TUNE_CCW = { key = "F23" }
TUNE_PRESS = { key = "F22" }
LEFT_TOUCH_SWIPE_LEFT = { key = "F20", mods = "shift" }

[default_profile]
name = "Default"
[default_profile.bindings.TUNE_CW]
action = { type = "media", key = "volume_up" }
accel = "light"
[default_profile.bindings.TUNE_CCW]
action = { type = "media", key = "volume_down" }
accel = "light"
[default_profile.bindings.TUNE_PRESS]
action = { type = "media", key = "mute" }

[[profiles]]
name = "Chrome"
match = { windows_exe = ["chrome.exe", "msedge.exe"], macos_bundle = ["com.google.Chrome"] }
[profiles.bindings.TUNE_CW]
action = { type = "keys", chord = "Ctrl+Tab" }
[profiles.bindings.TUNE_CCW]
action = { type = "keys", chord = "Ctrl+Shift+Tab" }
[profiles.bindings.LEFT_TOUCH_SWIPE_LEFT]
action = { type = "keys", chord = "Alt+Left" }
"#;

    #[test]
    fn parses_sample_and_round_trips() {
        let cfg = Config::from_toml(SAMPLE).unwrap();
        assert_eq!(cfg.transport.len(), 4);
        assert_eq!(cfg.profiles.len(), 1);
        assert_eq!(cfg.default_profile.bindings.len(), 3);

        let table = cfg.transport_table().unwrap();
        assert_eq!(table.len(), 4);

        let text = cfg.to_toml().unwrap();
        let again = Config::from_toml(&text).unwrap();
        assert_eq!(cfg, again);
    }

    #[test]
    fn rejects_duplicate_transport_code() {
        let bad = r#"
[transport]
TUNE_CW  = { key = "F24" }
TUNE_CCW = { key = "F24" }
"#;
        assert!(matches!(
            Config::from_toml(bad),
            Err(ConfigError::Transport(_))
        ));
    }

    #[test]
    fn rejects_future_schema() {
        assert!(matches!(
            Config::from_toml("schema_version = 99"),
            Err(ConfigError::TooNew(99))
        ));
    }
}
