//! OS-specific glue for Create Companion. Everything here is behind a trait or a
//! small function so the engine and core never see a Win32 or Cocoa type.
//!
//! Windows implementation: Phase 0/1. macOS: Phase 5.

use companion_core::action::Action;
use companion_core::transport::{Modifiers, TransportCode};
use std::time::Instant;

/// A swallowed transport key event, forwarded from the hook thread.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawTransportEvent {
    /// The key plus the modifiers the *module* sent with it (pressed within
    /// the namespace window before the key).
    pub code: TransportCode,
    /// Modifiers that were already down for longer than the window: the user
    /// holding a key. Not part of the code; reported so the engine can log it.
    pub held: Modifiers,
    pub pressed: bool,
    pub at: Instant,
    /// True when the key is in the transport table (and was swallowed).
    /// False only in learn mode, where unreserved F-keys are reported but
    /// passed through to the foreground app.
    pub reserved: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum PlatformError {
    #[error("{0}")]
    Os(String),
    #[error("unsupported on this platform")]
    Unsupported,
}

/// Global capture of reserved transport keys. Implementations must swallow
/// reserved keys so they never reach the foreground app, and must ignore
/// events they injected themselves.
pub trait InputHook: Send {
    fn start(&mut self) -> Result<(), PlatformError>;
    fn stop(&mut self);
}

/// Executes actions as synthetic input or process launches.
pub trait ActionSink: Send {
    fn execute(&mut self, action: &Action, repeat: u32) -> Result<(), PlatformError>;

    /// Release the modifier keys a transport namespace holds (PLAN.md §4.3).
    /// With `Shift+F19` the firmware's Shift is still down when we act, so a
    /// mapped `Ctrl+T` would otherwise arrive as `Ctrl+Shift+T`.
    fn release_modifiers(&mut self, mods: Modifiers) -> Result<(), PlatformError>;
}

#[cfg(windows)]
pub mod windows;
