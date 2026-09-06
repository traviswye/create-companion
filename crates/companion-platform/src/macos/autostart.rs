//! Start-at-login via a per-user LaunchAgent
//! (`~/Library/LaunchAgents/dev.createcompanion.engine.plist`). No elevation.

use crate::PlatformError;
use std::path::PathBuf;

const LABEL: &str = "dev.createcompanion.engine";

fn plist_path() -> Result<PathBuf, PlatformError> {
    let home = dirs::home_dir().ok_or(PlatformError::Os("no home directory".into()))?;
    Ok(home
        .join("Library/LaunchAgents")
        .join(format!("{LABEL}.plist")))
}

pub fn is_enabled() -> Result<bool, PlatformError> {
    Ok(plist_path()?.exists())
}

/// Idempotent. Writes (or removes) the agent and asks launchd to pick it up
/// for the current session; a launchctl failure is not fatal, the plist
/// alone takes effect at next login.
pub fn set_enabled(enabled: bool) -> Result<(), PlatformError> {
    let path = plist_path()?;
    if enabled == path.exists() {
        return Ok(());
    }
    let uid = unsafe { libc::getuid() };
    if enabled {
        let exe = std::env::current_exe().map_err(|e| PlatformError::Os(e.to_string()))?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| PlatformError::Os(e.to_string()))?;
        }
        let plist = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key><string>{LABEL}</string>
    <key>ProgramArguments</key><array><string>{}</string></array>
    <key>RunAtLoad</key><true/>
    <key>ProcessType</key><string>Interactive</string>
    <key>LimitLoadToSessionType</key><string>Aqua</string>
</dict>
</plist>
"#,
            xml_escape(&exe.to_string_lossy())
        );
        std::fs::write(&path, plist).map_err(|e| PlatformError::Os(e.to_string()))?;
        let _ = std::process::Command::new("launchctl")
            .args(["bootstrap", &format!("gui/{uid}")])
            .arg(&path)
            .status();
    } else {
        let _ = std::process::Command::new("launchctl")
            .args(["bootout", &format!("gui/{uid}/{LABEL}")])
            .status();
        std::fs::remove_file(&path).map_err(|e| PlatformError::Os(e.to_string()))?;
    }
    Ok(())
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
