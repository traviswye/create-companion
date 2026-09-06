//! The two permissions macOS gates us behind. Neither can be granted from
//! code: the request calls make the system add us to the list in System
//! Settings > Privacy & Security, where the user flips the switch.
//!
//! - Input Monitoring: required for the event tap (seeing the keys).
//! - Accessibility: required for posting keys and reading window titles.

use objc2_core_graphics::{CGPreflightListenEventAccess, CGRequestListenEventAccess};

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrusted() -> bool;
}

/// Whether the event tap can be created (Input Monitoring granted).
pub fn input_monitoring() -> bool {
    CGPreflightListenEventAccess()
}

/// Ask macOS to prompt for Input Monitoring. Returns the current state.
pub fn request_input_monitoring() -> bool {
    CGRequestListenEventAccess()
}

/// Whether we may post keyboard events and read window titles.
pub fn accessibility() -> bool {
    // SAFETY: no preconditions.
    unsafe { AXIsProcessTrusted() }
}

/// Open the relevant pane of System Settings.
pub fn open_settings(pane: Pane) {
    let url = match pane {
        Pane::InputMonitoring => {
            "x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent"
        }
        Pane::Accessibility => {
            "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility"
        }
    };
    let _ = std::process::Command::new("open").arg(url).spawn();
}

#[derive(Debug, Clone, Copy)]
pub enum Pane {
    InputMonitoring,
    Accessibility,
}
