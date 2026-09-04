//! Single-instance guard using a named mutex.

use crate::PlatformError;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS, HANDLE};
use windows::Win32::System::Threading::CreateMutexW;

/// Held for the life of the process; the mutex is released when it exits.
pub struct SingleInstance {
    _handle: HANDLE,
}

// HANDLE is a plain pointer-sized value; we never dereference it.
unsafe impl Send for SingleInstance {}

/// `Ok(Some(_))` if this is the first instance, `Ok(None)` if another
/// instance already holds the mutex.
pub fn acquire(name: &str) -> Result<Option<SingleInstance>, PlatformError> {
    let wide: Vec<u16> = format!("Local\\{name}")
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    // SAFETY: valid null-terminated wide string; no security attributes.
    let handle = unsafe { CreateMutexW(None, false, PCWSTR(wide.as_ptr())) }
        .map_err(|e| PlatformError::Os(e.to_string()))?;
    // SAFETY: no preconditions.
    if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        return Ok(None);
    }
    Ok(Some(SingleInstance { _handle: handle }))
}
