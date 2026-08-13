//! One large workspace window per Mix project.
//!
//! The main Elin window stays the Elixir/OTP command center. Deep project work
//! (graph, scan, Mix, Hex) happens here, sized to the current monitor.

use crate::error::{AppError, AppResult};
use once_cell::sync::Lazy;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::webview::PageLoadEvent;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

static PATHS: Lazy<Mutex<HashMap<String, String>>> = Lazy::new(|| Mutex::new(HashMap::new()));

pub fn label_for(path: &str) -> String {
    let key = path.replace('/', "\\").trim_end_matches('\\').to_lowercase();
    let hash = Sha256::digest(key.as_bytes());
    format!("ws-{}", hex::encode(&hash[..8]))
}

pub fn path_for_label(label: &str) -> Option<String> {
    PATHS.lock().ok()?.get(label).cloned()
}

fn remember(label: &str, path: &str) {
    if let Ok(mut map) = PATHS.lock() {
        map.insert(label.to_string(), path.to_string());
    }
}

/// Same origin as the working main window. Do not append a query string:
/// WebView2 + Vite often fail to load `http://localhost:1420/?shell=…`.
/// `eval` right after `build()` cancels the first navigation and leaves a
/// dark empty HWND — inject only via init script / `on_page_load`.
fn page_url(app: &AppHandle) -> WebviewUrl {
    if let Some(main) = app.get_webview_window("main") {
        if let Ok(url) = main.url() {
            return WebviewUrl::External(url);
        }
    }
    if cfg!(debug_assertions) {
        if let Some(url) = app.config().build.dev_url.clone() {
            return WebviewUrl::External(url);
        }
    }
    WebviewUrl::App("index.html".into())
}

fn build_workspace(
    app: &AppHandle,
    label: &str,
    name: &str,
    width: f64,
    height: f64,
    x: f64,
    y: f64,
    script: &str,
) -> AppResult<tauri::WebviewWindow> {
    let after_load = script.to_string();
    WebviewWindowBuilder::new(app, label, page_url(app))
        .title(format!("Elin · {name}"))
        .inner_size(width, height)
        .min_inner_size(960.0, 640.0)
        .position(x, y)
        .decorations(false)
        .resizable(true)
        .visible(true)
        .skip_taskbar(false)
        .shadow(true)
        .devtools(false)
        .background_color(tauri::window::Color(0x0B, 0x0A, 0x12, 0xFF))
        .initialization_script(script)
        .on_page_load(move |w, payload| {
            if payload.event() == PageLoadEvent::Finished {
                let _ = w.eval(&after_load);
            }
        })
        .build()
        .map_err(|e| AppError::msg(e.to_string()))
}

/// Focus an existing workspace or create one. Idempotent per project path.
pub fn open(app: &AppHandle, project_path: &str) -> AppResult<()> {
    let mix = std::path::Path::new(project_path).join("mix.exs");
    if !mix.is_file() {
        return Err(AppError::msg("No mix.exs in that folder."));
    }
    crate::services::projects::touch_recent(project_path);
    let label = label_for(project_path);
    remember(&label, project_path);
    if let Some(existing) = app.get_webview_window(&label) {
        let _ = existing.unminimize();
        let _ = existing.show();
        let _ = existing.set_focus();
        return Ok(());
    }

    let name = std::path::Path::new(project_path)
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "project".into());
    let (width, height, x, y) = fit_monitor(app);
    let payload = serde_json::to_string(project_path).unwrap_or_else(|_| "\"\"".into());
    let script = format!(
        "window.__ELIN_SHELL='workspace';window.__ELIN_WORKSPACE={payload};document.documentElement.dataset.shell='workspace';document.documentElement.style.background='#0b0a12';document.addEventListener('contextmenu',function(e){{e.preventDefault();}});"
    );

    let window = build_workspace(app, &label, &name, width, height, x, y, &script)?;
    let _ = window.unminimize();
    let _ = window.show();
    let _ = window.set_focus();
    Ok(())
}

/// ~88% of the work area, clamped so a 13" laptop still fits and a 4K screen
/// does not open a postage stamp or a billboard.
fn fit_monitor(app: &AppHandle) -> (f64, f64, f64, f64) {
    let monitor = app
        .get_webview_window("main")
        .and_then(|w| w.current_monitor().ok().flatten())
        .or_else(|| app.primary_monitor().ok().flatten());
    let Some(m) = monitor else {
        return (1440.0, 900.0, 80.0, 60.0);
    };
    let scale = m.scale_factor();
    let area = m.work_area();
    let work_w = area.size.width as f64 / scale;
    let work_h = area.size.height as f64 / scale;
    let origin_x = area.position.x as f64 / scale;
    let origin_y = area.position.y as f64 / scale;
    let margin = 16.0;
    let usable_w = (work_w - margin * 2.0).max(800.0);
    let usable_h = (work_h - margin * 2.0).max(600.0);
    let width = (usable_w * 0.88).clamp(usable_w.min(1100.0), usable_w.min(1760.0));
    let height = (usable_h * 0.88).clamp(usable_h.min(720.0), usable_h.min(1120.0));
    let x = origin_x + (work_w - width) / 2.0;
    let y = origin_y + (work_h - height) / 2.0;
    (width, height, x, y)
}

/// Whitelisted Mix tasks the workspace run bar may fire.
pub fn run_task(project_path: &str, task: &str) -> AppResult<String> {
    let (args, secs): (&[&str], u64) = match task {
        "compile" => (&["compile"], 180),
        "test" => (&["test"], 300),
        "format" => (&["format"], 60),
        "deps.get" => (&["deps.get"], 180),
        _ => return Err(AppError::msg(format!("Unknown Mix task `{task}`."))),
    };
    crate::services::mixcmd::mix_in_project(
        std::path::Path::new(project_path),
        args,
        std::time::Duration::from_secs(secs),
    )
}

/// Insert `{:name, "req"}` into mix.exs and run `mix deps.get`.
pub fn add_hex_dep(project_path: &str, name: &str, requirement: &str) -> AppResult<String> {
    if name.is_empty()
        || !name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    {
        return Err(AppError::msg("Package name looks invalid."));
    }
    let req = requirement.trim();
    let req = if req.is_empty() { "~> 0.1" } else { req };
    if req.len() > 40 || req.contains('"') || req.contains('{') {
        return Err(AppError::msg("Requirement looks invalid."));
    }
    let mix = std::path::PathBuf::from(project_path).join("mix.exs");
    let text = std::fs::read_to_string(&mix)?;
    let tuple = format!(r#"{{:{name}, "{req}"}}"#);
    let next = crate::services::mixexs::insert_dep(&text, &tuple).map_err(AppError::msg)?;
    let mut log = Vec::new();
    if next != text {
        std::fs::write(&mix, next)?;
        log.push(format!("added {tuple}"));
    } else {
        log.push(format!("{name} already in mix.exs"));
    }
    match run_task(project_path, "deps.get") {
        Ok(out) => {
            if !out.trim().is_empty() {
                log.push(out);
            } else {
                log.push("mix deps.get finished.".into());
            }
        }
        Err(err) => log.push(format!("mix deps.get: {err}")),
    }
    Ok(log.join("\n"))
}

pub fn remove_hex_dep(project_path: &str, name: &str) -> AppResult<String> {
    if name.is_empty()
        || !name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    {
        return Err(AppError::msg("Package name looks invalid."));
    }
    let mix = std::path::PathBuf::from(project_path).join("mix.exs");
    let text = std::fs::read_to_string(&mix)?;
    let next = crate::services::mixexs::remove_dep(&text, name).map_err(AppError::msg)?;
    if next == text {
        return Ok(format!("{name} was not in mix.exs"));
    }
    std::fs::write(&mix, next)?;
    let mut log = vec![format!("removed :{name} from mix.exs")];
    match crate::services::mixcmd::mix_in_project(
        std::path::Path::new(project_path),
        &["deps.unlock", name],
        std::time::Duration::from_secs(60),
    ) {
        Ok(out) => {
            if !out.trim().is_empty() {
                log.push(out);
            }
        }
        Err(err) => log.push(format!("mix deps.unlock: {err}")),
    }
    match run_task(project_path, "deps.get") {
        Ok(out) => {
            if !out.trim().is_empty() {
                log.push(out);
            }
        }
        Err(err) => log.push(format!("mix deps.get: {err}")),
    }
    let lock = std::fs::read_to_string(std::path::PathBuf::from(project_path).join("mix.lock")).unwrap_or_default();
    if lock.contains(&format!("\"{name}\":")) {
        log.push(format!(
            "{name} is still in mix.lock because another package depends on it. That is expected — it is no longer a direct dep."
        ));
    } else {
        log.push(format!("{name} dropped from mix.lock."));
    }
    Ok(log.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_is_stable_and_short() {
        let a = label_for(r"D:\code\app");
        let b = label_for(r"D:/code/app\");
        assert_eq!(a, b);
        assert!(a.starts_with("ws-"));
        assert_eq!(a.len(), 19);
    }

    #[test]
    fn rejects_unknown_mix_task() {
        assert!(run_task(".", "phx.server").is_err());
    }

    #[test]
    fn remembers_path_by_label() {
        remember("ws-testlabel", r"D:\code\app");
        assert_eq!(path_for_label("ws-testlabel").as_deref(), Some(r"D:\code\app"));
    }

    #[test]
    fn query_encodes_windows_path() {
        let q = format!(
            "shell=workspace&workspace={}",
            urlencoding::encode(r"D:\code\my app")
        );
        assert!(q.starts_with("shell=workspace&workspace="));
        assert!(q.contains("my+app") || q.contains("my%20app"));
    }
}
