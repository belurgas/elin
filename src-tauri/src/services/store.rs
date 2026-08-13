//! Durable Elin state under `%LOCALAPPDATA%/elin`.
//!
//! The cache directory is for Hex/catalog TTL and may be wiped. Project lists,
//! pins, and CLI open-requests live here so "Clear cache" cannot forget them.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// `%LOCALAPPDATA%/elin` (or home/elin as a fallback).
pub fn data_dir() -> PathBuf {
    dirs::data_local_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(std::env::temp_dir)
        .join("elin")
}

pub fn path(name: &str) -> PathBuf {
    data_dir().join(name)
}

/// Copy `name` from the cache dir once, if the durable file is missing.
pub fn migrate_from_cache(name: &str) {
    let dest = path(name);
    if dest.exists() {
        return;
    }
    let src = crate::services::cache::cache_dir().join(name);
    if !src.exists() {
        return;
    }
    let _ = fs::create_dir_all(data_dir());
    let _ = fs::copy(src, dest);
}

pub fn read_json<T: for<'de> Deserialize<'de>>(name: &str) -> Option<T> {
    migrate_from_cache(name);
    let raw = fs::read_to_string(path(name)).ok()?;
    serde_json::from_str(&raw).ok()
}

pub fn write_json<T: Serialize>(name: &str, value: &T) {
    let dir = data_dir();
    let _ = fs::create_dir_all(&dir);
    if let Ok(json) = serde_json::to_string_pretty(value) {
        let _ = fs::write(dir.join(name), json);
    }
}

pub fn write_gui_pid() {
    write_json("gui.pid", &std::process::id());
}

/// True when a previous Elin GUI process is still alive.
pub fn gui_is_running() -> bool {
    let Some(pid) = read_json::<u32>("gui.pid") else {
        return false;
    };
    pid_alive(pid)
}

fn pid_alive(pid: u32) -> bool {
    #[cfg(windows)]
    {
        windows_pid_alive(pid)
    }
    #[cfg(not(windows))]
    {
        let _ = pid;
        false
    }
}

#[cfg(windows)]
fn windows_pid_alive(pid: u32) -> bool {
    const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
    const STILL_ACTIVE: u32 = 259;
    #[link(name = "kernel32")]
    extern "system" {
        fn OpenProcess(access: u32, inherit: i32, pid: u32) -> *mut std::ffi::c_void;
        fn GetExitCodeProcess(handle: *mut std::ffi::c_void, code: *mut u32) -> i32;
        fn CloseHandle(handle: *mut std::ffi::c_void) -> i32;
    }
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            return false;
        }
        let mut code = 0u32;
        let ok = GetExitCodeProcess(handle, &mut code);
        CloseHandle(handle);
        ok != 0 && code == STILL_ACTIVE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_dir_ends_with_elin() {
        assert!(data_dir().ends_with("elin"));
    }
}
