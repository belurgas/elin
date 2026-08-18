//! Debounced project watcher. Ignores `_build` / `deps` / `.git` and only
//! wakes the studio when source, Mix, or lockfiles actually change.

use once_cell::sync::Lazy;
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use notify::Watcher;
use tauri::{AppHandle, Emitter};

const DEBOUNCE: Duration = Duration::from_millis(380);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FsTick {
    pub path: String,
    pub graph: bool,
    pub git: bool,
    pub lock: bool,
}

struct Session {
    stop: Arc<AtomicBool>,
    _watcher: notify::RecommendedWatcher,
}

static SESSIONS: Lazy<Mutex<HashMap<String, Session>>> = Lazy::new(|| Mutex::new(HashMap::new()));

fn key(path: &str) -> String {
    crate::services::host::path_key(path)
}

pub fn start(app: AppHandle, project_path: String) -> crate::error::AppResult<()> {
    let root = PathBuf::from(&project_path);
    if !root.is_dir() {
        return Err(crate::error::AppError::msg("Project folder is missing."));
    }
    stop(&project_path);
    let stop = Arc::new(AtomicBool::new(false));
    let pending: Arc<Mutex<Pending>> = Arc::new(Mutex::new(Pending::default()));
    let emit_path = project_path.clone();
    let stop_tick = stop.clone();
    let pending_tick = pending.clone();
    let app_tick = app.clone();
    std::thread::spawn(move || debounce_loop(app_tick, emit_path, stop_tick, pending_tick));

    let pending_ev = pending.clone();
    let mut watcher = notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
        let Ok(event) = res else { return };
        if matches!(
            event.kind,
            notify::EventKind::Access(_) | notify::EventKind::Other
        ) {
            return;
        }
        if let Ok(mut slot) = pending_ev.lock() {
            for path in event.paths {
                slot.note(&path);
            }
        }
    })
    .map_err(|e| crate::error::AppError::msg(e.to_string()))?;

    watcher
        .watch(&root, notify::RecursiveMode::Recursive)
        .map_err(|e| crate::error::AppError::msg(e.to_string()))?;

    if let Ok(mut map) = SESSIONS.lock() {
        map.insert(
            key(&project_path),
            Session {
                stop,
                _watcher: watcher,
            },
        );
    }
    Ok(())
}

pub fn stop(project_path: &str) {
    if let Ok(mut map) = SESSIONS.lock() {
        if let Some(session) = map.remove(&key(project_path)) {
            session.stop.store(true, Ordering::Relaxed);
        }
    }
}

#[derive(Default)]
struct Pending {
    last: Option<Instant>,
    graph: bool,
    git: bool,
    lock: bool,
}

impl Pending {
    fn note(&mut self, path: &Path) {
        if ignored(path) {
            return;
        }
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let interesting = ext == "ex"
            || ext == "exs"
            || name == "mix.exs"
            || name == "mix.lock"
            || name == ".gitignore"
            || name == "license"
            || name.starts_with("license.");
        if !interesting {
            return;
        }
        self.last = Some(Instant::now());
        self.git = true;
        if name == "mix.lock" {
            self.lock = true;
            self.graph = true;
        } else if ext == "ex" || ext == "exs" || name == "mix.exs" {
            self.graph = true;
        }
    }

    fn take_ready(&mut self) -> Option<(bool, bool, bool)> {
        let last = self.last?;
        if last.elapsed() < DEBOUNCE {
            return None;
        }
        if !self.graph && !self.git && !self.lock {
            return None;
        }
        let tick = (self.graph, self.git, self.lock);
        *self = Pending::default();
        Some(tick)
    }
}

fn debounce_loop(app: AppHandle, path: String, stop: Arc<AtomicBool>, pending: Arc<Mutex<Pending>>) {
    while !stop.load(Ordering::Relaxed) {
        std::thread::sleep(Duration::from_millis(120));
        let tick = pending.lock().ok().and_then(|mut g| g.take_ready());
        if let Some((graph, git, lock)) = tick {
            let _ = app.emit(
                "workspace-fs",
                FsTick {
                    path: path.clone(),
                    graph,
                    git,
                    lock,
                },
            );
        }
    }
}

fn ignored(path: &Path) -> bool {
    static SKIP: &[&str] = &[
        "_build",
        "deps",
        ".git",
        ".elixir_ls",
        "node_modules",
        "cover",
        "doc",
        ".elixir-tools",
        "priv\\static",
        "priv/static",
    ];
    let text = crate::services::host::path_key(&path.to_string_lossy());
    SKIP.iter().any(|seg| {
        let seg = seg.replace('\\', "/");
        text.contains(&format!("/{seg}/")) || text.ends_with(&format!("/{seg}"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skips_build_and_deps() {
        assert!(ignored(Path::new(r"D:\app\_build\dev\lib\foo.beam")));
        assert!(ignored(Path::new(r"D:\app\deps\jason\lib\jason.ex")));
        assert!(!ignored(Path::new(r"D:\app\lib\foo.ex")));
    }
}
