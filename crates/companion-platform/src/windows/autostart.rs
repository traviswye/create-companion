//! Start-at-login via the per-user Run registry key. No elevation.

use crate::PlatformError;
use auto_launch::{AutoLaunch, AutoLaunchBuilder};

const APP_NAME: &str = "NayaCompanion";

fn launcher() -> Result<AutoLaunch, PlatformError> {
    let exe = std::env::current_exe().map_err(|e| PlatformError::Os(e.to_string()))?;
    AutoLaunchBuilder::new()
        .set_app_name(APP_NAME)
        .set_app_path(&exe.to_string_lossy())
        .build()
        .map_err(|e| PlatformError::Os(e.to_string()))
}

pub fn is_enabled() -> Result<bool, PlatformError> {
    launcher()?
        .is_enabled()
        .map_err(|e| PlatformError::Os(e.to_string()))
}

/// Idempotent: only touches the registry when the state actually changes.
pub fn set_enabled(enabled: bool) -> Result<(), PlatformError> {
    let al = launcher()?;
    let now = al
        .is_enabled()
        .map_err(|e| PlatformError::Os(e.to_string()))?;
    if now == enabled {
        return Ok(());
    }
    let r = if enabled { al.enable() } else { al.disable() };
    r.map_err(|e| PlatformError::Os(e.to_string()))
}
