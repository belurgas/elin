//! Project Studio IPC: remembered Mix projects, graph, git, kits, and scan.

use super::{blocking, blocking_result};
use crate::desktop::{self, ToastPayload};
use crate::error::{AppError, AppResult};
use crate::services::catalog::fetch_catalog;
use crate::services::git::{self, GitSnapshot};
use crate::services::install::{install_pair, pair_satisfying, pick_from_catalog};
use crate::services::kits::{self, Kit, KitStatus};
use crate::services::projects::{
    add_project as remember_project, cancel_scan, deep_scan, enrich, module_graph, open_in_studio,
    parse_project, quick_scan, remembered, set_pin, take_open_request, toggle_star, touch_recent,
    MixProject, ModuleGraph,
};
use crate::services::scan::{self, ScanReport};
use crate::services::studios::Studio;
use serde::Serialize;
use tauri::{AppHandle, Emitter};

#[tauri::command]
pub async fn list_projects() -> AppResult<Vec<MixProject>> {
    blocking(remembered).await
}

#[tauri::command]
pub async fn scan_projects_quick() -> AppResult<Vec<MixProject>> {
    blocking_result(quick_scan).await
}

#[tauri::command]
pub async fn scan_projects_deep(app: AppHandle, roots: Vec<String>) -> AppResult<Vec<MixProject>> {
    let handle = app.clone();
    let projects = blocking_result(move || deep_scan(handle, roots)).await?;
    desktop::emit_toast(
        &app,
        ToastPayload {
            id: "scan".into(),
            title: "Project scan finished".into(),
            body: format!("Found {} Mix project(s).", projects.len()),
            kind: "ok".into(),
            page: Some("projects".into()),
        },
    );
    Ok(projects)
}

#[tauri::command]
pub fn cancel_project_scan() {
    cancel_scan();
}

#[tauri::command]
pub async fn inspect_project(path: String) -> AppResult<MixProject> {
    blocking_result(move || {
        touch_recent(&path);
        remember_project(&path)
    })
    .await
}

#[tauri::command]
pub async fn install_project_toolchain(app: AppHandle, path: String) -> AppResult<MixProject> {
    let mix = std::path::PathBuf::from(&path).join("mix.exs");
    let project = parse_project(&mix).ok_or_else(|| AppError::msg("No mix.exs in that folder"))?;
    let req = project.elixir_req.clone().ok_or_else(|| {
        AppError::msg("This mix.exs has no elixir: requirement. Pin an installed version instead.")
    })?;

    if let Some(existing) = pair_satisfying(&req) {
        set_pin(&path, &existing.elixir, &existing.otp)?;
        desktop::emit_toast(
            &app,
            ToastPayload {
                id: "project-pin".into(),
                title: "Project toolchain ready".into(),
                body: format!(
                    "Elixir {} + OTP {} is already installed. Pinned for this project only.",
                    existing.elixir, existing.otp
                ),
                kind: "ok".into(),
                page: Some("projects".into()),
            },
        );
        return Ok(enrich(project));
    }

    let catalog = fetch_catalog(false, false).await?;
    let (elixir, otp) = pick_from_catalog(&req, &catalog)?;
    install_pair(app.clone(), elixir.clone(), otp.clone(), false, true).await?;
    set_pin(&path, &elixir, &otp)?;
    desktop::emit_toast(
        &app,
        ToastPayload {
            id: "project-install".into(),
            title: "Project toolchain installed".into(),
            body: format!(
                "Elixir {elixir} + OTP {otp} is pinned to this project. Default PATH is unchanged."
            ),
            kind: "ok".into(),
            page: Some("projects".into()),
        },
    );
    parse_project(&mix)
        .ok_or_else(|| AppError::msg("Installed, but mix.exs could not be re-read."))
}

#[tauri::command]
pub async fn pin_project_toolchain(
    path: String,
    elixir: String,
    otp: String,
) -> AppResult<MixProject> {
    blocking_result(move || {
        set_pin(&path, &elixir, &otp)?;
        let mix = std::path::PathBuf::from(&path).join("mix.exs");
        parse_project(&mix).ok_or_else(|| AppError::msg("No mix.exs in that folder"))
    })
    .await
}

#[tauri::command]
pub async fn project_graph(path: String) -> AppResult<ModuleGraph> {
    blocking_result(move || module_graph(path)).await
}

#[tauri::command]
pub fn open_project_in_studio(
    studio: Studio,
    path: String,
    file: Option<String>,
    line: Option<u32>,
) -> AppResult<()> {
    touch_recent(&path);
    open_in_studio(studio, path, file, line)
}

#[tauri::command]
pub async fn add_project(path: String) -> AppResult<MixProject> {
    blocking_result(move || remember_project(&path)).await
}

#[tauri::command]
pub async fn star_project(path: String) -> AppResult<MixProject> {
    blocking_result(move || toggle_star(&path)).await
}

#[tauri::command]
pub async fn project_git(path: String) -> GitSnapshot {
    blocking(move || git::snapshot(&path))
        .await
        .unwrap_or_default()
}

#[tauri::command]
pub async fn project_commit(path: String, message: String, files: Vec<String>) -> AppResult<String> {
    blocking_result(move || git::commit(&path, &message, &files)).await
}

#[tauri::command]
pub async fn project_scan(path: String, full: bool, mix_layers: bool) -> AppResult<ScanReport> {
    blocking_result(move || scan::run_scan(&path, full, mix_layers)).await
}

#[tauri::command]
pub async fn project_format(path: String, check: bool) -> AppResult<String> {
    blocking_result(move || {
        let args: Vec<&str> = if check {
            vec!["format", "--check-formatted"]
        } else {
            vec!["format"]
        };
        crate::services::mixcmd::mix_in_project(
            std::path::Path::new(&path),
            &args,
            std::time::Duration::from_secs(60),
        )
    })
    .await
}

#[tauri::command]
pub async fn list_kits(path: String) -> AppResult<Vec<KitStatus>> {
    blocking_result(move || {
        let mix = std::path::PathBuf::from(&path).join("mix.exs");
        let project =
            parse_project(&mix).ok_or_else(|| AppError::msg("No mix.exs in that folder"))?;
        Ok(kits::status_for(&project))
    })
    .await
}

#[tauri::command]
pub fn kit_catalog() -> Vec<Kit> {
    kits::catalog()
}

#[tauri::command]
pub async fn apply_project_kits(path: String, ids: Vec<String>) -> AppResult<String> {
    blocking_result(move || kits::apply_kits(&path, &ids, true)).await
}

#[tauri::command]
pub async fn remove_project_kit(path: String, id: String) -> AppResult<String> {
    blocking_result(move || kits::remove_kit(&path, &id)).await
}

#[tauri::command]
pub async fn write_kit_config(path: String, id: String) -> AppResult<String> {
    blocking_result(move || kits::write_kit_config(&path, &id)).await
}

#[tauri::command]
pub async fn set_credo_strict(path: String, strict: bool) -> AppResult<String> {
    blocking_result(move || kits::set_credo_strict(&path, strict)).await
}

/// Consume a pending `elin open` request. Parse first, then delete the file.
#[tauri::command]
pub fn take_open_project() -> Option<String> {
    take_open_request()
}

/// Open (or focus) the large per-project workspace window.
///
/// Must be async. A sync command runs on the UI thread; creating a WebView2
/// window from there deadlocks every webview in the process (main, tray, and
/// the new workspace) so later IPC never returns.
#[tauri::command]
pub async fn open_project_workspace(app: AppHandle, path: String) -> AppResult<()> {
    let check = path.clone();
    blocking_result(move || {
        if std::path::Path::new(&check).join("mix.exs").is_file() {
            Ok(())
        } else {
            Err(AppError::msg("No mix.exs in that folder."))
        }
    })
    .await?;

    let handle = app.clone();
    app.run_on_main_thread(move || {
        if let Err(err) = crate::services::workspace::open(&handle, &path) {
            desktop::emit_toast(
                &handle,
                ToastPayload {
                    id: "workspace".into(),
                    title: "Workspace failed".into(),
                    body: err.to_string(),
                    kind: "error".into(),
                    page: Some("projects".into()),
                },
            );
        }
    })
    .map_err(|e| AppError::msg(e.to_string()))
}

/// Path for the calling workspace window. Used when the init script did not run.
#[tauri::command]
pub fn workspace_context(window: tauri::WebviewWindow) -> Option<String> {
    crate::services::workspace::path_for_label(window.label())
}

#[tauri::command]
pub async fn project_mix(app: AppHandle, path: String, task: String, session: Option<String>) -> AppResult<String> {
    let handle = app.clone();
    let task_name = task.clone();
    let sid = session.unwrap_or_else(|| "main".into());
    blocking_result(move || {
        let (args, secs) = mix_args(&task)?;
        crate::services::mixcmd::mix_with_lines(
            std::path::Path::new(&path),
            args,
            std::time::Duration::from_secs(secs),
            |line| {
                let _ = handle.emit(
                    "mix-line",
                    MixLine {
                        session: sid.clone(),
                        task: task_name.clone(),
                        line: line.to_string(),
                    },
                );
            },
        )
    })
    .await
}

#[tauri::command]
pub async fn project_shell(app: AppHandle, path: String, session: String, command: String) -> AppResult<String> {
    let handle = app.clone();
    let sid = session.clone();
    let shown = command.clone();
    blocking_result(move || {
        crate::services::mixcmd::shell_in_project(
            std::path::Path::new(&path),
            &command,
            std::time::Duration::from_secs(180),
            |line| {
                let _ = handle.emit(
                    "mix-line",
                    MixLine {
                        session: sid.clone(),
                        task: shown.clone(),
                        line: line.to_string(),
                    },
                );
            },
        )
    })
    .await
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct MixLine {
    session: String,
    task: String,
    line: String,
}

fn mix_args(task: &str) -> AppResult<(&'static [&'static str], u64)> {
    match task {
        "compile" => Ok((&["compile"], 180)),
        "test" => Ok((&["test"], 300)),
        "format" => Ok((&["format"], 60)),
        "format.check" => Ok((&["format", "--check-formatted"], 60)),
        "deps.get" => Ok((&["deps.get"], 180)),
        _ => Err(AppError::msg(format!("Unknown Mix task `{task}`."))),
    }
}

#[tauri::command]
pub async fn add_hex_dep(path: String, name: String, requirement: String) -> AppResult<String> {
    blocking_result(move || crate::services::workspace::add_hex_dep(&path, &name, &requirement)).await
}

#[tauri::command]
pub async fn remove_hex_dep(path: String, name: String) -> AppResult<String> {
    blocking_result(move || crate::services::workspace::remove_hex_dep(&path, &name)).await
}

#[tauri::command]
pub fn workspace_watch_start(app: AppHandle, path: String) -> AppResult<()> {
    crate::services::watch::start(app, path)
}

#[tauri::command]
pub fn workspace_watch_stop(path: String) {
    crate::services::watch::stop(&path);
}

#[tauri::command]
pub fn git_licenses() -> Vec<crate::services::git::LicenseOpt> {
    crate::services::git::license_options()
}

#[tauri::command]
pub async fn git_init(path: String, license: String) -> AppResult<GitSnapshot> {
    blocking_result(move || crate::services::git::init_repo(&path, &license)).await
}

#[tauri::command]
pub async fn add_elin_comment(path: String, file: String, tag: String, value: String) -> AppResult<()> {
    blocking_result(move || crate::services::analyze::insert_comment(&path, &file, &tag, &value)).await
}
