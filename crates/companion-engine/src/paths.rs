//! Where the engine keeps its files.
//!
//! Windows: config `%APPDATA%\CreateCompanion\config.toml`,
//!          logs   `%LOCALAPPDATA%\CreateCompanion\logs\`.

use std::path::PathBuf;

const DIR_NAME: &str = "CreateCompanion";
/// The folder name the program used before it was renamed (2026-09-05).
const LEGACY_DIR_NAME: &str = "NayaCompanion";

/// One-time move of a `NayaCompanion` folder to `CreateCompanion` (config and
/// logs), so an existing setup survives the rename. Idempotent; never
/// overwrites a folder that already exists under the new name.
pub fn migrate_legacy() {
    for (base, what) in [
        (dirs::config_dir(), "config"),
        (dirs::data_local_dir(), "data"),
    ] {
        let Some(base) = base else { continue };
        let old = base.join(LEGACY_DIR_NAME);
        let new = base.join(DIR_NAME);
        if old.is_dir() && !new.exists() {
            match std::fs::rename(&old, &new) {
                Ok(()) => eprintln!(
                    "migrated {what} folder {} -> {}",
                    old.display(),
                    new.display()
                ),
                Err(e) => eprintln!("could not migrate {what} folder {}: {e}", old.display()),
            }
        }
    }
}

pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(DIR_NAME)
}

pub fn config_file() -> PathBuf {
    config_dir().join("config.toml")
}

pub fn log_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(config_dir)
        .join(DIR_NAME)
        .join("logs")
}
