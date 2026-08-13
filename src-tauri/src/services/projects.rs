//! Mix project discovery, pins, recents, and the remembered project list.
//!
//! Durable files live under `%LOCALAPPDATA%/elin` via [`crate::services::store`],
//! not the cache directory — "Clear cache" must not forget projects.

use crate::error::{AppError, AppResult};
use crate::services::analyze;
use crate::services::cache;
use crate::services::store;
use crate::services::studios::{Studio, StudioFamily};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter};
use walkdir::WalkDir;

pub use crate::services::analyze::ModuleGraph;

static CANCEL: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MixDep {
    pub name: String,
    pub spec: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MixProject {
    pub name: String,
    pub path: String,
    pub mix_exs: String,
    pub deps: Vec<MixDep>,
    pub locked: Vec<MixDep>,
    pub has_phoenix: bool,
    pub has_liveview: bool,
    #[serde(default)]
    pub has_application: bool,
    #[serde(default)]
    pub elixir_req: Option<String>,
    #[serde(default)]
    pub pinned_elixir: Option<String>,
    #[serde(default)]
    pub pinned_otp: Option<String>,
    #[serde(default)]
    pub resolved_elixir: Option<String>,
    #[serde(default)]
    pub resolved_otp: Option<String>,
    #[serde(default)]
    pub starred: bool,
    #[serde(default)]
    pub last_opened: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanProgress {
    pub visited: u64,
    pub found: u32,
    pub current: String,
    pub done: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct ProjectMeta {
    #[serde(default)]
    starred: Vec<String>,
    #[serde(default)]
    recents: Vec<String>,
    /// Unix seconds when the project was last opened in Studio.
    #[serde(default)]
    opened_at: BTreeMap<String, u64>,
}

/// Remembered projects from the last scan / add. Instant — does not walk the disk.
pub fn remembered() -> Vec<MixProject> {
    load_list().into_iter().map(enrich).collect()
}

fn load_list() -> Vec<MixProject> {
    store::read_json::<Vec<MixProject>>("projects.json").unwrap_or_default()
}

fn mix_exists(project: &MixProject) -> bool {
    Path::new(&project.mix_exs).is_file() || Path::new(&project.path).join("mix.exs").is_file()
}

/// Merge incoming projects into the remembered list. Does not drop projects
/// the current scan did not visit, as long as mix.exs is still on disk.
pub fn merge_and_save(incoming: &[MixProject]) -> Vec<MixProject> {
    let mut map: BTreeMap<String, MixProject> = BTreeMap::new();
    for project in load_list() {
        if mix_exists(&project) {
            map.insert(pin_key(&project.path), project);
        }
    }
    for project in incoming {
        map.insert(pin_key(&project.path), project.clone());
    }
    let list: Vec<MixProject> = map.into_values().collect();
    store::write_json("projects.json", &list);
    list.into_iter().map(enrich).collect()
}

/// Walk up from `path` until mix.exs, then merge into the remembered list.
pub fn add_project(path: &str) -> AppResult<MixProject> {
    let mix = find_mix_exs(Path::new(path))?;
    let project = parse_project(&mix).ok_or_else(|| AppError::msg("Could not read mix.exs"))?;
    if project.pinned_elixir.is_none() {
        if let (Some(elixir), Some(otp)) = (&project.resolved_elixir, &project.resolved_otp) {
            let _ = set_pin(&project.path, elixir, otp);
        }
    }
    touch_recent(&project.path);
    let list = merge_and_save(&[project.clone()]);
    list.into_iter()
        .find(|p| pin_key(&p.path) == pin_key(&project.path))
        .ok_or_else(|| AppError::msg("Project was added but could not be re-read."))
}

pub fn find_mix_exs(start: &Path) -> AppResult<PathBuf> {
    let mut cur = if start.is_file() {
        start.parent().map(Path::to_path_buf)
    } else {
        Some(start.to_path_buf())
    };
    while let Some(dir) = cur {
        let mix = dir.join("mix.exs");
        if mix.is_file() {
            return Ok(mix);
        }
        cur = dir.parent().map(Path::to_path_buf);
    }
    Err(AppError::msg(
        "No mix.exs above that path. Open a Mix project folder.",
    ))
}

/// Read a pending `elin open` path. The file is deleted only after a successful parse
/// so a crash mid-read cannot lose the request.
pub fn take_open_request() -> Option<String> {
    let path = store::path("open-request.json");
    let raw = fs::read_to_string(&path).ok()?;
    let parsed = serde_json::from_str::<String>(&raw).ok().or_else(|| {
        serde_json::from_str::<serde_json::Value>(&raw)
            .ok()
            .and_then(|v| v.get("path").and_then(|p| p.as_str()).map(str::to_string))
    });
    if parsed.is_some() {
        let _ = fs::remove_file(&path);
    }
    parsed
}

pub fn write_open_request(path: &str) -> AppResult<()> {
    store::write_json("open-request.json", &path);
    Ok(())
}

/// Quick scan of likely folders. Does not walk the home root or entire drives.
pub fn quick_scan() -> AppResult<Vec<MixProject>> {
    let mut roots = Vec::new();
    if let Some(home) = dirs::home_dir() {
        roots.push(home.join("Documents"));
        roots.push(home.join("Desktop"));
        roots.push(home.join("Projects"));
        roots.push(home.join("Developer"));
        roots.push(home.join("source"));
        roots.push(home.join("code"));
        roots.push(home.join("src"));
        roots.push(home.join("dev"));
    }
    let projects = scan_roots(&roots, 4, None)?;
    Ok(merge_and_save(&projects))
}

/// Optional deep scan. Emits `project-scan` progress. Cancellable.
pub fn deep_scan(app: AppHandle, extra_roots: Vec<String>) -> AppResult<Vec<MixProject>> {
    CANCEL.store(false, Ordering::SeqCst);
    let mut roots: Vec<PathBuf> = extra_roots.into_iter().map(PathBuf::from).collect();
    if roots.is_empty() {
        for letter in b'C'..=b'Z' {
            let drive = PathBuf::from(format!("{}:\\", letter as char));
            if drive.exists() {
                roots.push(drive);
            }
        }
    }
    let projects = scan_roots(&roots, 8, Some(&app))?;
    let merged = merge_and_save(&projects);
    let _ = app.emit(
        "project-scan",
        ScanProgress {
            visited: 0,
            found: merged.len() as u32,
            current: String::new(),
            done: true,
        },
    );
    Ok(merged)
}

pub fn cancel_scan() {
    CANCEL.store(true, Ordering::SeqCst);
}

fn scan_roots(roots: &[PathBuf], max_depth: usize, app: Option<&AppHandle>) -> AppResult<Vec<MixProject>> {
    let mut found: BTreeMap<String, MixProject> = BTreeMap::new();
    let mut visited = 0u64;
    for root in roots {
        if !root.exists() {
            continue;
        }
        for entry in WalkDir::new(root)
            .max_depth(max_depth)
            .into_iter()
            .filter_entry(|e| {
                let name = e.file_name().to_string_lossy();
                e.depth() == 0 || !skip_dir(&name)
            })
            .flatten()
        {
            if CANCEL.load(Ordering::SeqCst) {
                break;
            }
            visited += 1;
            if visited % 250 == 0 {
                if let Some(app) = app {
                    let _ = app.emit(
                        "project-scan",
                        ScanProgress {
                            visited,
                            found: found.len() as u32,
                            current: entry.path().to_string_lossy().into(),
                            done: false,
                        },
                    );
                }
            }
            if entry.file_name() == "mix.exs" {
                if let Some(project) = parse_project(entry.path()) {
                    found.entry(project.path.clone()).or_insert(project);
                }
            }
        }
    }
    Ok(found.into_values().collect())
}

fn skip_dir(name: &str) -> bool {
    matches!(
        name,
        "node_modules"
            | "deps"
            | "_build"
            | ".git"
            | "target"
            | "Windows"
            | "Program Files"
            | "Program Files (x86)"
            | "ProgramData"
            | "$Recycle.Bin"
            | "System Volume Information"
            | "AppData"
            | "Application Data"
            | "Local Settings"
            | ".elixir_ls"
            | "vendor"
            | "dist"
            | ".cache"
            | ".nuget"
            | ".cargo"
            | ".rustup"
            | ".npm"
            | ".local"
            | "Temp"
            | "tmp"
            | "__pycache__"
            | ".vscode"
            | ".cursor"
            | "INetCache"
            | "Packages"
    )
}

pub fn parse_project(mix_exs: &Path) -> Option<MixProject> {
    let text = fs::read_to_string(mix_exs).ok()?;
    let dir = mix_exs.parent()?;
    // Skip Hex/Mix dependency checkouts.
    if dir.components().any(|c| c.as_os_str() == "deps") {
        return None;
    }
    let name = capture_app_name(&text)
        .or_else(|| {
            dir.file_name()
                .map(|n| n.to_string_lossy().to_string())
        })
        .unwrap_or_else(|| "mix".into());
    let deps = parse_deps(&text);
    let locked = parse_lock(&dir.join("mix.lock"));
    let has_phoenix = deps.iter().any(|d| d.name == "phoenix")
        || locked.iter().any(|d| d.name == "phoenix");
    let has_liveview = deps.iter().any(|d| d.name == "phoenix_live_view")
        || locked.iter().any(|d| d.name == "phoenix_live_view");
    let has_application = APP_MOD.is_match(&text);
    Some(enrich(MixProject {
        name,
        path: dir.to_string_lossy().into(),
        mix_exs: mix_exs.to_string_lossy().into(),
        deps,
        locked,
        has_phoenix,
        has_liveview,
        has_application,
        elixir_req: capture_elixir_req(&text),
        pinned_elixir: None,
        pinned_otp: None,
        resolved_elixir: None,
        resolved_otp: None,
        starred: false,
        last_opened: None,
    }))
}

static APP_NAME: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"app:\s*:([A-Za-z0-9_]+)").expect("app name regex"));
static APP_MOD: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?s)def application\b.*?mod:\s*\{").expect("application mod regex"));
static ELIXIR_REQ: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"elixir:\s*["']([^"']+)["']"#).expect("elixir req regex"));
static DEPS: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"\{:([a-zA-Z0-9_]+),\s*"([^"]+)""#).expect("deps regex"));
static LOCK: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#""([a-zA-Z0-9_]+)":\s*\{:hex,\s*:([a-zA-Z0-9_]+),\s*"([^"]+)""#)
        .expect("lock regex")
});

fn capture_app_name(text: &str) -> Option<String> {
    APP_NAME
        .captures(text)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
}

fn capture_elixir_req(text: &str) -> Option<String> {
    ELIXIR_REQ
        .captures(text)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().trim().to_string())
        .filter(|s| !s.is_empty())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProjectPin {
    elixir: String,
    otp: String,
}

fn pin_key(path: &str) -> String {
    path.replace('/', "\\")
        .trim_end_matches('\\')
        .to_lowercase()
}

fn load_pins() -> BTreeMap<String, ProjectPin> {
    store::read_json("project-pins.json").unwrap_or_default()
}

fn save_pins(pins: &BTreeMap<String, ProjectPin>) -> AppResult<()> {
    let path = store::path("project-pins.json");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_string_pretty(pins).unwrap_or_default())?;
    Ok(())
}

pub fn set_pin(project_path: &str, elixir: &str, otp: &str) -> AppResult<()> {
    let mut pins = load_pins();
    pins.insert(
        pin_key(project_path),
        ProjectPin {
            elixir: elixir.into(),
            otp: otp.into(),
        },
    );
    save_pins(&pins)
}

pub fn enrich(mut project: MixProject) -> MixProject {
    if project.elixir_req.is_none() {
        if let Ok(text) = fs::read_to_string(&project.mix_exs) {
            project.elixir_req = capture_elixir_req(&text);
        }
    }
    let meta = load_meta();
    let key = pin_key(&project.path);
    project.starred = meta.starred.iter().any(|s| s == &key);
    project.last_opened = meta.opened_at.get(&key).copied();
    if let Some(pin) = load_pins().get(&key).cloned() {
        project.pinned_elixir = Some(pin.elixir.clone());
        project.pinned_otp = Some(pin.otp.clone());
        if let Some(pair) = crate::services::install::find_pair(&pin.elixir, Some(&pin.otp)) {
            project.resolved_elixir = Some(pair.elixir);
            project.resolved_otp = Some(pair.otp);
            return project;
        }
    }
    if let Some(req) = project.elixir_req.as_deref() {
        if let Some(pair) = crate::services::install::pair_satisfying(req) {
            project.resolved_elixir = Some(pair.elixir);
            project.resolved_otp = Some(pair.otp);
        }
    }
    project
}

fn load_meta() -> ProjectMeta {
    let mut meta: ProjectMeta = store::read_json("project-meta.json").unwrap_or_default();
    if meta.opened_at.is_empty() && !meta.recents.is_empty() {
        let now = cache::now_unix();
        for (i, key) in meta.recents.iter().enumerate() {
            meta.opened_at
                .insert(key.clone(), now.saturating_sub(i as u64));
        }
        save_meta(&meta);
    }
    meta
}

fn save_meta(meta: &ProjectMeta) {
    store::write_json("project-meta.json", meta);
}

pub fn touch_recent(project_path: &str) {
    let key = pin_key(project_path);
    let mut meta = load_meta();
    meta.recents.retain(|s| s != &key);
    meta.recents.insert(0, key.clone());
    meta.recents.truncate(40);
    meta.opened_at.insert(key, cache::now_unix());
    save_meta(&meta);
}

pub fn toggle_star(project_path: &str) -> AppResult<MixProject> {
    let key = pin_key(project_path);
    let mut meta = load_meta();
    if let Some(idx) = meta.starred.iter().position(|s| s == &key) {
        meta.starred.remove(idx);
    } else {
        meta.starred.push(key);
    }
    save_meta(&meta);
    add_project(project_path)
}

pub fn bins_for_project(project_path: &str) -> Option<(PathBuf, PathBuf)> {
    let mix = PathBuf::from(project_path).join("mix.exs");
    let project = parse_project(&mix)?;
    let elixir = project.resolved_elixir.as_deref()?;
    let otp = project.resolved_otp.as_deref();
    let pair = crate::services::install::find_pair(elixir, otp)?;
    if pair.otp_path.is_empty() || pair.elixir_path.is_empty() {
        return None;
    }
    Some((PathBuf::from(pair.otp_path), PathBuf::from(pair.elixir_path)))
}

fn parse_deps(text: &str) -> Vec<MixDep> {
    DEPS.captures_iter(text)
        .map(|c| MixDep {
            name: c[1].to_string(),
            spec: c[2].to_string(),
        })
        .collect()
}

fn parse_lock(path: &Path) -> Vec<MixDep> {
    let Ok(text) = fs::read_to_string(path) else {
        return Vec::new();
    };
    LOCK.captures_iter(&text)
        .map(|c| MixDep {
            name: c[1].to_string(),
            spec: c[3].to_string(),
        })
        .collect()
}

/// Static module graph from lib/ and test/ — no Mix compilation.
pub fn module_graph(project_path: String) -> AppResult<ModuleGraph> {
    let mut graph = analyze::analyze_project(&project_path)?;
    let git = crate::services::git::snapshot(&project_path);
    crate::services::git::overlay(&mut graph, &git);
    Ok(graph)
}

pub fn open_in_studio(studio: Studio, path: String, file: Option<String>, line: Option<u32>) -> AppResult<()> {
    let target = PathBuf::from(&path);
    if !target.exists() {
        return Err(AppError::msg("That project folder does not exist"));
    }
    let program = studio
        .cli
        .clone()
        .or(studio.executable.clone())
        .ok_or_else(|| AppError::msg("This studio has no CLI or executable to launch"))?;
    let project = path.replace('/', "\\");
    let mut args: Vec<String> = vec![project.clone()];
    if let Some(file) = file {
        let file_path = if Path::new(&file).is_absolute() {
            file.replace('/', "\\")
        } else {
            target.join(file.replace('/', "\\")).to_string_lossy().into_owned()
        };
        if studio.family == StudioFamily::Vscode {
            if let Some(line) = line.filter(|n| *n > 0) {
                args.push("-g".into());
                args.push(format!("{file_path}:{line}"));
            } else {
                args.push(file_path);
            }
        } else {
            args.push(file_path);
        }
    }
    spawn_editor(&program, &args)
}

fn spawn_editor(program: &str, args: &[String]) -> AppResult<()> {
    let script = program.to_ascii_lowercase().ends_with(".cmd") || program.to_ascii_lowercase().ends_with(".bat");
    let mut cmd = if script {
        let mut c = std::process::Command::new("cmd.exe");
        c.arg("/D").arg("/C").arg(program).args(args);
        c
    } else {
        let mut c = std::process::Command::new(program);
        c.args(args);
        c
    };
    #[cfg(windows)]
    {
        crate::services::winproc::hide_console_ex(
            &mut cmd,
            crate::services::winproc::CREATE_NEW_PROCESS_GROUP,
        );
    }
    cmd.spawn().map_err(|e| AppError::msg(e.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_mix_exs_app_and_deps() {
        let sample = r#"
        def project do
          [
            app: :hello_elin,
            elixir: "~> 1.15",
            deps: [
              {:phoenix, "~> 1.7.0"},
              {:jason, "~> 1.4"}
            ]
          ]
        end
        "#;
        assert_eq!(capture_app_name(sample).as_deref(), Some("hello_elin"));
        let deps = parse_deps(sample);
        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0].name, "phoenix");
        assert_eq!(capture_elixir_req(sample).as_deref(), Some("~> 1.15"));
    }

    #[test]
    fn find_mix_walks_up() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("elin-mixwalk-{nanos}"));
        fs::create_dir_all(dir.join("lib").join("nested")).unwrap();
        fs::write(dir.join("mix.exs"), "defmodule X.MixProject do\n  def project, do: [app: :x, elixir: \"~> 1.15\", deps: []]\nend\n").unwrap();
        let found = find_mix_exs(&dir.join("lib").join("nested")).unwrap();
        assert_eq!(found, dir.join("mix.exs"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn pin_key_normalizes_slashes() {
        assert_eq!(pin_key(r"D:\a\b\"), pin_key(r"D:/a/b"));
    }
}
