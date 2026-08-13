//! One GUI process at a time.
//!
//! The CLI (`elin`, `elin open`) must never start a second window. A named
//! mutex is the source of truth; the pid file and FindWindow are fallbacks.
//!
//! A second desktop launch must restore the **main** window, not the tray
//! flyout. Those shells used to share the title "Elin", so FindWindow showed
//! the popup instead.

use std::ffi::c_void;

const MUTEX_NAME: &str = "Local\\ElinGuiSingleton";
const WAKE_NAME: &str = "Local\\ElinWakeGui";
const MAIN_TITLE: &str = "Elin";

/// True when an Elin GUI already owns the singleton mutex (or a live window).
pub fn is_running() -> bool {
    #[cfg(windows)]
    {
        if mutex_open() {
            return true;
        }
        if find_main_hwnd().is_some() {
            return true;
        }
    }
    crate::services::store::gui_is_running()
}

/// Take the singleton. `false` means another GUI already holds it.
pub fn try_claim() -> bool {
    #[cfg(windows)]
    {
        return claim_mutex();
    }
    #[cfg(not(windows))]
    {
        !crate::services::store::gui_is_running()
    }
}

/// Restore the main window even if it is sitting in the tray.
///
/// The running process handles the wake (Tauri `show_main`). This process
/// also ShowWindow's the HWND so Windows lets it take foreground — the user
/// just clicked the shortcut, so we have that right.
pub fn focus() -> bool {
    #[cfg(windows)]
    {
        let woken = wake();
        let shown = focus_hwnd();
        return woken || shown;
    }
    #[cfg(not(windows))]
    {
        false
    }
}

/// Background thread: when a second `elin.exe` signals, show the main window.
pub fn spawn_wake_listener<F>(on_wake: F)
where
    F: Fn() + Send + 'static,
{
    #[cfg(windows)]
    {
        let _ = std::thread::Builder::new().name("elin-wake".into()).spawn(move || {
            let event = create_wake_event();
            if event.is_null() {
                return;
            }
            loop {
                if wait_wake(event) {
                    on_wake();
                }
            }
        });
    }
    #[cfg(not(windows))]
    {
        let _ = on_wake;
    }
}

#[cfg(windows)]
fn wide(name: &str) -> Vec<u16> {
    name.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(windows)]
fn mutex_open() -> bool {
    const SYNCHRONIZE: u32 = 0x0010_0000;
    #[link(name = "kernel32")]
    extern "system" {
        fn OpenMutexW(access: u32, inherit: i32, name: *const u16) -> *mut c_void;
        fn CloseHandle(handle: *mut c_void) -> i32;
    }
    unsafe {
        let name = wide(MUTEX_NAME);
        let handle = OpenMutexW(SYNCHRONIZE, 0, name.as_ptr());
        if handle.is_null() {
            return false;
        }
        CloseHandle(handle);
        true
    }
}

#[cfg(windows)]
fn claim_mutex() -> bool {
    const ERROR_ALREADY_EXISTS: u32 = 183;
    #[link(name = "kernel32")]
    extern "system" {
        fn CreateMutexW(sa: *const c_void, owner: i32, name: *const u16) -> *mut c_void;
        fn CloseHandle(handle: *mut c_void) -> i32;
        fn GetLastError() -> u32;
    }
    unsafe {
        let name = wide(MUTEX_NAME);
        let handle = CreateMutexW(std::ptr::null(), 1, name.as_ptr());
        if handle.is_null() {
            return true;
        }
        if GetLastError() == ERROR_ALREADY_EXISTS {
            CloseHandle(handle);
            return false;
        }
        // Keep the handle for the process lifetime so the mutex stays held.
        std::mem::forget(HandleLive(handle));
        true
    }
}

#[cfg(windows)]
fn create_wake_event() -> *mut c_void {
    #[link(name = "kernel32")]
    extern "system" {
        fn CreateEventW(sa: *const c_void, manual: i32, initial: i32, name: *const u16) -> *mut c_void;
    }
    unsafe { CreateEventW(std::ptr::null(), 0, 0, wide(WAKE_NAME).as_ptr()) }
}

#[cfg(windows)]
fn wait_wake(event: *mut c_void) -> bool {
    #[link(name = "kernel32")]
    extern "system" {
        fn WaitForSingleObject(handle: *mut c_void, ms: u32) -> u32;
    }
    const INFINITE: u32 = 0xFFFF_FFFF;
    const WAIT_OBJECT_0: u32 = 0;
    unsafe { WaitForSingleObject(event, INFINITE) == WAIT_OBJECT_0 }
}

#[cfg(windows)]
fn wake() -> bool {
    const EVENT_MODIFY_STATE: u32 = 0x0002;
    #[link(name = "kernel32")]
    extern "system" {
        fn OpenEventW(access: u32, inherit: i32, name: *const u16) -> *mut c_void;
        fn SetEvent(handle: *mut c_void) -> i32;
        fn CloseHandle(handle: *mut c_void) -> i32;
    }
    unsafe {
        let handle = OpenEventW(EVENT_MODIFY_STATE, 0, wide(WAKE_NAME).as_ptr());
        if handle.is_null() {
            return false;
        }
        let ok = SetEvent(handle) != 0;
        CloseHandle(handle);
        ok
    }
}

#[cfg(windows)]
#[allow(dead_code)]
struct HandleLive(*mut c_void);
#[cfg(windows)]
unsafe impl Send for HandleLive {}
#[cfg(windows)]
unsafe impl Sync for HandleLive {}

#[cfg(windows)]
struct Search {
    hwnd: *mut c_void,
}

#[cfg(windows)]
fn find_main_hwnd() -> Option<*mut c_void> {
    #[link(name = "user32")]
    extern "system" {
        fn EnumWindows(cb: unsafe extern "system" fn(*mut c_void, isize) -> i32, lparam: isize) -> i32;
    }
    let mut search = Search {
        hwnd: std::ptr::null_mut(),
    };
    unsafe {
        EnumWindows(enum_main, std::ptr::addr_of_mut!(search) as isize);
    }
    if search.hwnd.is_null() {
        None
    } else {
        Some(search.hwnd)
    }
}

#[cfg(windows)]
unsafe extern "system" fn enum_main(hwnd: *mut c_void, lparam: isize) -> i32 {
    let search = unsafe { &mut *(lparam as *mut Search) };
    if is_main_window(hwnd) {
        search.hwnd = hwnd;
        0
    } else {
        1
    }
}

#[cfg(windows)]
fn is_main_window(hwnd: *mut c_void) -> bool {
    const GWL_EXSTYLE: i32 = -20;
    const WS_EX_TOOLWINDOW: isize = 0x0000_0080;
    const GW_OWNER: u32 = 4;
    #[link(name = "user32")]
    extern "system" {
        fn GetWindowTextW(hwnd: *mut c_void, buf: *mut u16, max: i32) -> i32;
        fn GetWindowLongPtrW(hwnd: *mut c_void, index: i32) -> isize;
        fn GetWindow(hwnd: *mut c_void, cmd: u32) -> *mut c_void;
    }
    let mut buf = [0u16; 32];
    let n = unsafe { GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32) };
    if n <= 0 {
        return false;
    }
    let title = String::from_utf16_lossy(&buf[..n as usize]);
    if title != MAIN_TITLE {
        return false;
    }
    unsafe {
        if !GetWindow(hwnd, GW_OWNER).is_null() {
            return false;
        }
        if GetWindowLongPtrW(hwnd, GWL_EXSTYLE) & WS_EX_TOOLWINDOW != 0 {
            return false;
        }
    }
    true
}

#[cfg(windows)]
fn focus_hwnd() -> bool {
    const SW_RESTORE: i32 = 9;
    #[link(name = "user32")]
    extern "system" {
        fn ShowWindow(hwnd: *mut c_void, cmd: i32) -> i32;
        fn SetForegroundWindow(hwnd: *mut c_void) -> i32;
        fn BringWindowToTop(hwnd: *mut c_void) -> i32;
    }
    let Some(hwnd) = find_main_hwnd() else {
        return false;
    };
    unsafe {
        ShowWindow(hwnd, SW_RESTORE);
        BringWindowToTop(hwnd);
        SetForegroundWindow(hwnd) != 0
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn mutex_name_is_local() {
        assert!(super::MUTEX_NAME.starts_with("Local\\"));
    }

    #[test]
    fn wake_name_is_local() {
        assert!(super::WAKE_NAME.starts_with("Local\\"));
    }

    #[test]
    fn main_title_is_exact() {
        assert_eq!(super::MAIN_TITLE, "Elin");
    }
}
