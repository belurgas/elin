//! Tiny ANSI helper for the CLI. No extra crate — just VT sequences.

use std::io::{self, IsTerminal};
use std::sync::atomic::{AtomicBool, Ordering};

static COLOR: AtomicBool = AtomicBool::new(false);

pub fn enable() {
    let on = std::env::var_os("NO_COLOR").is_none() && io::stdout().is_terminal();
    COLOR.store(on, Ordering::Relaxed);
    #[cfg(windows)]
    if on {
        enable_vt();
    }
}

pub fn on() -> bool {
    COLOR.load(Ordering::Relaxed)
}

pub fn paint(code: &str, text: &str) -> String {
    if on() {
        format!("\x1b[{code}m{text}\x1b[0m")
    } else {
        text.to_string()
    }
}

pub fn bold(text: &str) -> String {
    paint("1", text)
}
pub fn dim(text: &str) -> String {
    paint("2", text)
}
pub fn violet(text: &str) -> String {
    paint("38;5;141", text)
}
pub fn green(text: &str) -> String {
    paint("32", text)
}
pub fn red(text: &str) -> String {
    paint("31", text)
}
pub fn yellow(text: &str) -> String {
    paint("33", text)
}
pub fn cyan(text: &str) -> String {
    paint("36", text)
}

pub fn ok_mark() -> String {
    green("✓")
}
pub fn fail_mark() -> String {
    red("✗")
}
pub fn warn_mark() -> String {
    yellow("!")
}
pub fn skip_mark() -> String {
    dim("·")
}

pub fn banner() {
    println!(
        "  {}  {}",
        bold(&violet("elin")),
        dim("elixir companion")
    );
    println!();
}

pub fn err(msg: &str) {
    eprintln!("  {}  {msg}", fail_mark());
}

pub fn ok(msg: &str) {
    println!("  {}  {msg}", ok_mark());
}

pub fn info(msg: &str) {
    println!("  {}  {msg}", violet("→"));
}

#[cfg(windows)]
fn enable_vt() {
    const ENABLE_VIRTUAL_TERMINAL_PROCESSING: u32 = 0x0004;
    const ENABLE_PROCESSED_OUTPUT: u32 = 0x0001;
    const STD_OUTPUT_HANDLE: u32 = 0xFFFF_FFF5;
    #[link(name = "kernel32")]
    extern "system" {
        fn GetStdHandle(n: u32) -> *mut std::ffi::c_void;
        fn GetConsoleMode(h: *mut std::ffi::c_void, mode: *mut u32) -> i32;
        fn SetConsoleMode(h: *mut std::ffi::c_void, mode: u32) -> i32;
    }
    unsafe {
        let handle = GetStdHandle(STD_OUTPUT_HANDLE);
        if handle.is_null() || handle == (-1isize as *mut std::ffi::c_void) {
            return;
        }
        let mut mode = 0u32;
        if GetConsoleMode(handle, &mut mode) == 0 {
            return;
        }
        SetConsoleMode(handle, mode | ENABLE_PROCESSED_OUTPUT | ENABLE_VIRTUAL_TERMINAL_PROCESSING);
    }
}
