//! macOS implementation: a CGEvent tap on its own run-loop thread, the
//! frontmost application from NSWorkspace (plus the focused window's title
//! through the Accessibility API), CGEvent posting for actions, a LaunchAgent
//! for start-at-login, a lock file for single instance, and a main-thread
//! AppKit event pump for the tray.
//!
//! Permissions the user has to grant (see [`permissions`]): Input Monitoring
//! for the tap, Accessibility for posting keys and reading window titles.

pub mod autostart;
pub mod foreground;
pub mod hook;
pub mod input;
pub mod instance;
pub mod message_loop;
pub mod permissions;

pub use foreground::{current as foreground_app, current_title as foreground_title};
pub use hook::EventTapHook as KeyboardHook;
pub use input::CGEventSink;
/// The action executor under the name the engine uses on every platform.
pub use input::CGEventSink as Sink;
