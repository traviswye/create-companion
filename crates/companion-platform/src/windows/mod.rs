//! Windows implementation: low-level keyboard hook + foreground watch on one
//! message-pump thread, a `SendInput` executor, autostart, single instance,
//! and a main-thread message loop for the tray.

pub mod autostart;
pub mod foreground;
pub mod hook;
pub mod input;
pub mod instance;
pub mod message_loop;

pub use foreground::{
    current as foreground_app, current_title as foreground_title, foreground_exe,
};
pub use hook::KeyboardHook;
pub use input::SendInputSink;
