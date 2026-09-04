//! Foreground application tracking.
//!
//! Event-driven: a `WinEvent` hook for `EVENT_SYSTEM_FOREGROUND` (installed on
//! the keyboard hook's message-pump thread) updates a cached identity whenever
//! focus changes. The pipeline reads the cache; nothing polls.

use crate::PlatformError;
use companion_core::profile::AppIdentity;
use std::sync::RwLock;
use windows::core::BOOL;
use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::Accessibility::{SetWinEventHook, UnhookWinEvent, HWINEVENTHOOK};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId, IsWindowVisible,
    EVENT_SYSTEM_FOREGROUND, WINEVENT_OUTOFCONTEXT, WINEVENT_SKIPOWNPROCESS,
};

static CURRENT: RwLock<Option<AppIdentity>> = RwLock::new(None);

/// Executable name (e.g. `chrome.exe`) of the process owning `hwnd`.
pub fn exe_for_hwnd(hwnd: HWND) -> Result<Option<AppIdentity>, PlatformError> {
    if hwnd.0.is_null() {
        return Ok(None);
    }
    // SAFETY: plain Win32 calls with valid out-pointers; handles are closed.
    unsafe {
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == 0 {
            return Ok(None);
        }
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid)
            .map_err(|e| PlatformError::Os(e.to_string()))?;
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        let result = QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_WIN32,
            windows::core::PWSTR(buf.as_mut_ptr()),
            &mut len,
        );
        let _ = CloseHandle(handle);
        result.map_err(|e| PlatformError::Os(e.to_string()))?;
        let path = String::from_utf16_lossy(&buf[..len as usize]);
        let exe = path.rsplit(['\\', '/']).next().unwrap_or(&path).to_owned();
        Ok(Some(AppIdentity::WindowsExe(exe)))
    }
}

/// One-shot lookup of the current foreground process.
pub fn foreground_exe() -> Result<Option<AppIdentity>, PlatformError> {
    // SAFETY: no preconditions.
    exe_for_hwnd(unsafe { GetForegroundWindow() })
}

/// Cached foreground identity, falling back to a live lookup if the cache is
/// empty (before the watch is installed, or after a lookup failure).
pub fn current() -> Option<AppIdentity> {
    if let Ok(guard) = CURRENT.read() {
        if guard.is_some() {
            return guard.clone();
        }
    }
    foreground_exe().ok().flatten()
}

/// Title of the foreground window, read on demand (used for website profiles).
pub fn current_title() -> Option<String> {
    // SAFETY: GetWindowTextW writes at most `buf.len()` UTF-16 units.
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0.is_null() {
            return None;
        }
        let mut buf = [0u16; 512];
        let n = GetWindowTextW(hwnd, &mut buf);
        if n <= 0 {
            return None;
        }
        Some(String::from_utf16_lossy(&buf[..n as usize]))
    }
}

/// A top-level window the user can see: its owning executable and title.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisibleWindow {
    pub exe: String,
    pub title: String,
}

/// Enumerate visible, titled top-level windows (for "add application" pickers).
pub fn visible_windows() -> Vec<VisibleWindow> {
    unsafe extern "system" fn cb(hwnd: HWND, lparam: LPARAM) -> BOOL {
        // SAFETY: lparam is the Vec we passed below; the callback runs synchronously.
        let out = unsafe { &mut *(lparam.0 as *mut Vec<VisibleWindow>) };
        if unsafe { IsWindowVisible(hwnd) }.as_bool() {
            let mut buf = [0u16; 512];
            let n = unsafe { GetWindowTextW(hwnd, &mut buf) };
            if n > 0 {
                let title = String::from_utf16_lossy(&buf[..n as usize]);
                if let Ok(Some(AppIdentity::WindowsExe(exe))) = exe_for_hwnd(hwnd) {
                    out.push(VisibleWindow { exe, title });
                }
            }
        }
        BOOL(1)
    }
    let mut out: Vec<VisibleWindow> = Vec::new();
    // SAFETY: standard EnumWindows with a pointer to a live Vec.
    unsafe {
        let _ = EnumWindows(Some(cb), LPARAM(&mut out as *mut _ as isize));
    }
    out
}

fn set_current(id: Option<AppIdentity>) {
    if let Ok(mut guard) = CURRENT.write() {
        *guard = id;
    }
}

unsafe extern "system" fn win_event_proc(
    _hook: HWINEVENTHOOK,
    event: u32,
    hwnd: HWND,
    _id_object: i32,
    _id_child: i32,
    _thread: u32,
    _time: u32,
) {
    if event == EVENT_SYSTEM_FOREGROUND {
        // On failure (e.g. an elevated process we cannot open) clear the cache
        // so the pipeline falls back to the default profile.
        set_current(exe_for_hwnd(hwnd).ok().flatten());
    }
}

/// Install the foreground watch on the *calling* thread, which must pump
/// messages. Returns the hook handle for [`uninstall_watch`].
pub(crate) fn install_watch() -> Option<HWINEVENTHOOK> {
    // SAFETY: out-of-context hook with a valid callback; no module handle needed.
    let hook = unsafe {
        SetWinEventHook(
            EVENT_SYSTEM_FOREGROUND,
            EVENT_SYSTEM_FOREGROUND,
            None,
            Some(win_event_proc),
            0,
            0,
            WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
        )
    };
    if hook.0.is_null() {
        return None;
    }
    set_current(foreground_exe().ok().flatten());
    Some(hook)
}

pub(crate) fn uninstall_watch(hook: HWINEVENTHOOK) {
    // SAFETY: handle came from SetWinEventHook on this thread.
    unsafe {
        let _ = UnhookWinEvent(hook);
    }
}
