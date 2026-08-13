//! Custom tray popup and toast windows (Tauri v2, no OS chrome).
//!
//! Windows WebView2 paints a rectangular HWND. Combining that with
//! `shadow(true)`, a semi-transparent `.glass` card, and padding around it
//! produces the ghost outlines and "rifts" users saw. These shells are:
//! transparent, unshadowed, filled edge-to-edge with an opaque rounded panel.

use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
};

static LAST_TOAST: Mutex<Option<ToastPayload>> = Mutex::new(None);
static TRAY_SHOWN_AT: Mutex<Option<Instant>> = Mutex::new(None);

pub const TOAST_W: f64 = 360.0;
pub const TOAST_H: f64 = 108.0;
pub const TRAY_W: f64 = 280.0;
pub const TRAY_H: f64 = 404.0;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToastPayload {
    pub id: String,
    pub title: String,
    pub body: String,
    pub kind: String,
    pub page: Option<String>,
}

fn shell_init_script(kind: &str) -> String {
    format!(
        r#"window.__ELIN_SHELL='{kind}';
document.documentElement.style.background='transparent';
document.addEventListener('contextmenu',function(e){{e.preventDefault();}});
document.addEventListener('DOMContentLoaded',function(){{
  document.documentElement.dataset.shell='{kind}';
  document.documentElement.style.background='transparent';
  if(document.body) document.body.style.background='transparent';
}});"#,
        kind = kind
    )
}

fn on_main(app: &AppHandle, f: impl FnOnce() + Send + 'static) {
    let _ = app.run_on_main_thread(f);
}

pub fn setup_shell_windows(app: &AppHandle) -> tauri::Result<()> {
    let icon = app.default_window_icon().cloned();
    let mut tray = TrayIconBuilder::with_id("elin-tray")
        .tooltip("Elin — Elixir companion")
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|tray, event| {
            let app = tray.app_handle().clone();
            match event {
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
                | TrayIconEvent::DoubleClick {
                    button: MouseButton::Left,
                    ..
                } => {
                    let handle = app.clone();
                    on_main(&app, move || {
                        let _ = show_main(&handle);
                    });
                }
                TrayIconEvent::Click {
                    button: MouseButton::Right,
                    button_state: MouseButtonState::Up,
                    position,
                    ..
                } => {
                    let handle = app.clone();
                    let x = position.x;
                    let y = position.y;
                    on_main(&app, move || {
                        toggle_tray_popup(&handle, x, y);
                    });
                }
                _ => {}
            }
        });
    if let Some(icon) = icon {
        tray = tray.icon(icon);
    }
    tray.build(app)?;
    // Build the tray webview now so the first right-click is not a hitch.
    let _ = ensure_shell(app, "tray", "tray", TRAY_W, TRAY_H);
    Ok(())
}

fn ensure_shell(
    app: &AppHandle,
    label: &str,
    kind: &str,
    width: f64,
    height: f64,
) -> tauri::Result<WebviewWindow> {
    if let Some(existing) = app.get_webview_window(label) {
        return Ok(existing);
    }
    WebviewWindowBuilder::new(app, label, WebviewUrl::App("index.html".into()))
        .title(format!("elin-{kind}"))
        .inner_size(width, height)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .focusable(true)
        .visible(false)
        .resizable(false)
        .transparent(true)
        .shadow(false)
        .devtools(false)
        .initialization_script(&shell_init_script(kind))
        .build()
}

fn note_tray_shown() {
    if let Ok(mut slot) = TRAY_SHOWN_AT.lock() {
        *slot = Some(Instant::now());
    }
}

fn tray_shown_recently() -> bool {
    TRAY_SHOWN_AT
        .lock()
        .ok()
        .and_then(|g| *g)
        .is_some_and(|t| t.elapsed() < Duration::from_millis(220))
}

pub fn hide_tray(app: &AppHandle) {
    if let Some(tray) = app.get_webview_window("tray") {
        let _ = tray.hide();
    }
}

/// Dismiss the tray unless it was just opened. Immediate `Focused(false)`
/// after a tray-icon click is a WebView2 race; click-away after that is real.
pub fn hide_tray_on_blur(app: &AppHandle) {
    if tray_shown_recently() {
        return;
    }
    hide_tray(app);
}

fn toggle_tray_popup(app: &AppHandle, x: f64, y: f64) {
    let Ok(window) = ensure_shell(app, "tray", "tray", TRAY_W, TRAY_H) else {
        return;
    };
    if window.is_visible().unwrap_or(false) {
        hide_tray(app);
        return;
    }
    hide_toast(app);
    place_in_work_area(&window, TRAY_W, TRAY_H, Some((x, y)), 12.0);
    note_tray_shown();
    let _ = window.show();
    let _ = window.set_always_on_top(true);
    let _ = window.set_focus();
    // The tray-icon click is still being processed; set_focus often no-ops
    // until that click finishes. Without focus, click-away never fires.
    let delayed = window.clone();
    let handle = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(80));
        let _ = handle.run_on_main_thread(move || {
            let _ = delayed.set_focus();
        });
    });
}

/// Place a flyout in the monitor work area.
/// If `anchor` is set (tray click, physical px), sit above it; otherwise bottom-right.
fn place_in_work_area(
    window: &WebviewWindow,
    logical_w: f64,
    logical_h: f64,
    anchor: Option<(f64, f64)>,
    gap: f64,
) {
    let scale = window.scale_factor().unwrap_or(1.0);
    let w = logical_w * scale;
    let h = logical_h * scale;
    let pad = 12.0 * scale;
    let gap = gap * scale;

    let monitor = window
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| window.primary_monitor().ok().flatten());

    let (left, top, right, bottom) = if let Some(m) = monitor {
        let area = m.work_area();
        let l = area.position.x as f64;
        let t = area.position.y as f64;
        (
            l,
            t,
            l + area.size.width as f64,
            t + area.size.height as f64,
        )
    } else {
        (0.0, 0.0, 1920.0, 1080.0)
    };

    let (mut x, mut y) = if let Some((ax, ay)) = anchor {
        (ax - w / 2.0, ay - h - gap)
    } else {
        (right - w - pad, bottom - h - pad)
    };

    x = x.clamp(left + pad, (right - w - pad).max(left + pad));
    y = y.clamp(top + pad, (bottom - h - pad).max(top + pad));
    let _ = window.set_position(PhysicalPosition::new(x.round() as i32, y.round() as i32));
}

pub fn show_main(app: &AppHandle) -> tauri::Result<()> {
    hide_tray(app);
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        window.show()?;
        window.set_focus()?;
    }
    Ok(())
}

pub fn hide_main_to_tray(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
    hide_tray(app);
}

pub fn emit_toast(app: &AppHandle, toast: ToastPayload) {
    if let Ok(mut slot) = LAST_TOAST.lock() {
        *slot = Some(toast.clone());
    }
    let _ = app.emit("elin-toast", &toast);
    if let Ok(window) = ensure_shell(app, "toast", "toast", TOAST_W, TOAST_H) {
        hide_tray(app);
        place_in_work_area(&window, TOAST_W, TOAST_H, None, 12.0);
        let _ = window.show();
        let _ = window.set_always_on_top(true);
    }
}

pub fn last_toast() -> Option<ToastPayload> {
    LAST_TOAST.lock().ok().and_then(|g| g.clone())
}

pub fn hide_toast(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("toast") {
        let _ = window.hide();
    }
}
