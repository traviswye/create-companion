//! Where the engine keeps its files.
//!
//! Windows: config `%APPDATA%\NayaCompanion\config.toml`,
//!          logs   `%LOCALAPPDATA%\NayaCompanion\logs\`.

use std::path::PathBuf;

const DIR_NAME: &str = "NayaCompanion";

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
