//! Single-instance guard: an advisory lock on a file in the user's
//! Application Support folder, released when the process exits.

use crate::PlatformError;
use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

pub struct SingleInstance {
    fd: i32,
}

impl Drop for SingleInstance {
    fn drop(&mut self) {
        // SAFETY: fd is ours; closing releases the lock.
        unsafe {
            libc::close(self.fd);
        }
    }
}

/// `Ok(Some(_))` if this is the first instance, `Ok(None)` if another
/// process holds the lock. `name` is a file name; the lock lives in
/// `~/Library/Application Support/CreateCompanion/`.
pub fn acquire(name: &str) -> Result<Option<SingleInstance>, PlatformError> {
    let dir = dirs::config_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("CreateCompanion");
    std::fs::create_dir_all(&dir).map_err(|e| PlatformError::Os(e.to_string()))?;
    let path: &Path = &dir.join(format!("{name}.lock"));
    let c =
        CString::new(path.as_os_str().as_bytes()).map_err(|e| PlatformError::Os(e.to_string()))?;
    // SAFETY: valid C string; flags and mode are constants.
    let fd = unsafe {
        libc::open(
            c.as_ptr(),
            libc::O_RDWR | libc::O_CREAT | libc::O_CLOEXEC,
            0o644,
        )
    };
    if fd < 0 {
        return Err(PlatformError::Os(
            std::io::Error::last_os_error().to_string(),
        ));
    }
    // SAFETY: fd is open.
    let r = unsafe { libc::flock(fd, libc::LOCK_EX | libc::LOCK_NB) };
    if r != 0 {
        let err = std::io::Error::last_os_error();
        // SAFETY: fd is open.
        unsafe {
            libc::close(fd);
        }
        return if err.raw_os_error() == Some(libc::EWOULDBLOCK) {
            Ok(None)
        } else {
            Err(PlatformError::Os(err.to_string()))
        };
    }
    Ok(Some(SingleInstance { fd }))
}
