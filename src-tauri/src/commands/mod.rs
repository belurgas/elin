//! Tauri IPC surface. Every command is a thin, documented wrapper around a
//! service so the frontend never talks to files or PATH directly.
//!
//! Project Studio commands live in [`studio`].

mod studio;
pub use studio::*;

use crate::desktop::{self, ToastPayload};
use crate::domain::{InstalledPair, VersionCatalog};
use crate::error::AppResult;
use crate::services::cache::{self, CacheStatus};
use crate::services::catalog::fetch_catalog;
use crate::services::doctor::{apply_fix, run_doctor, DoctorReport};
use crate::services::hexpm::{get_package, search_packages, HexPackage};
use crate::services::install::{activate_pair, install_pair, list_installed, remove_pair, InstallResult};
use crate::services::plugins::{install_plugin, neovim_snippet, status_for, PluginStatus};
use crate::services::probe::{probe_machine, StartupProbe};
use crate::services::runtime::{create_project, eval_snippet, SparkRequest, SparkResult};
use crate::services::studios::{detect_studios, studio_from_executable, Studio};
use serde::Serialize;
use tauri::{AppHandle, Emitter};

/// Run CPU/IO work off the UI thread.
pub(crate) async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> T + Send + 'static,
) -> AppResult<T> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| crate::error::AppError::msg(e.to_string()))
}

/// Same as [`blocking`], flattening an inner [`AppResult`].
pub(crate) async fn blocking_result<T: Send + 'static>(
    f: impl FnOnce() -> AppResult<T> + Send + 'static,
) -> AppResult<T> {
    blocking(f).await?
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostInfo {
    pub os: String,
    pub arch: String,
    pub home: Option<String>,
    pub installs_dir: String,
    pub version: String,
    pub repo: String,
}

#[tauri::command]
pub async fn get_host_info() -> AppResult<HostInfo> {
    Ok(HostInfo {
        os: std::env::consts::OS.into(),
        arch: std::env::consts::ARCH.into(),
        home: dirs::home_dir().map(|p| p.to_string_lossy().into()),
        installs_dir: crate::services::env::managed_root()?
            .to_string_lossy()
            .into(),
        version: crate::services::update::current_version(),
        repo: crate::services::update::REPO.into(),
    })
}

#[tauri::command]
pub async fn fetch_version_catalog(include_prerelease: bool, force: bool) -> AppResult<VersionCatalog> {
    fetch_catalog(include_prerelease, force).await
}

#[tauri::command]
pub async fn install_toolchain(
    app: AppHandle,
    elixir: String,
    otp: String,
    add_to_path: bool,
    install_hex: bool,
) -> AppResult<InstallResult> {
    let result = install_pair(app.clone(), elixir.clone(), otp.clone(), add_to_path, install_hex).await?;
    desktop::emit_toast(
        &app,
        ToastPayload {
            id: "installed".into(),
            title: "Toolchain ready".into(),
            body: format!("Elixir {elixir} + OTP {otp} is on this machine."),
            kind: "ok".into(),
            page: Some("doctor".into()),
        },
    );
    Ok(result)
}

#[tauri::command]
pub async fn list_toolchains() -> AppResult<Vec<InstalledPair>> {
    tauri::async_runtime::spawn_blocking(list_installed)
        .await
        .map_err(|e| crate::error::AppError::msg(e.to_string()))?
}

#[tauri::command]
pub fn activate_toolchain(app: AppHandle, elixir: String, otp: String) -> AppResult<InstalledPair> {
    let pair = activate_pair(&elixir, &otp)?;
    desktop::emit_toast(
        &app,
        ToastPayload {
            id: "path".into(),
            title: "PATH updated".into(),
            body: "Open a new terminal so `elixir` picks up the active version.".into(),
            kind: "ok".into(),
            page: Some("toolchain".into()),
        },
    );
    Ok(pair)
}

#[tauri::command]
pub fn remove_toolchain(elixir: String, otp: String) -> AppResult<()> {
    remove_pair(&elixir, &otp)
}

#[tauri::command]
pub async fn scan_studios() -> Vec<Studio> {
    tauri::async_runtime::spawn_blocking(detect_studios)
        .await
        .unwrap_or_default()
}

#[tauri::command]
pub fn import_studio(path: String) -> AppResult<Studio> {
    studio_from_executable(path)
}

#[tauri::command]
pub async fn list_plugins(studios: Vec<Studio>) -> Vec<PluginStatus> {
    tauri::async_runtime::spawn_blocking(move || status_for(&studios))
        .await
        .unwrap_or_default()
}

#[tauri::command]
pub fn install_studio_plugin(app: AppHandle, studio: Studio, marketplace_id: String) -> AppResult<String> {
    let out = install_plugin(&studio, &marketplace_id)?;
    desktop::emit_toast(
        &app,
        ToastPayload {
            id: "plugin".into(),
            title: format!("Installed into {}", studio.name),
            body: marketplace_id,
            kind: "ok".into(),
            page: Some("plugins".into()),
        },
    );
    Ok(out)
}

#[tauri::command]
pub fn get_neovim_snippet() -> String {
    neovim_snippet().to_string()
}

#[tauri::command]
pub async fn doctor_report() -> AppResult<DoctorReport> {
    tauri::async_runtime::spawn_blocking(run_doctor)
        .await
        .map_err(|e| crate::error::AppError::msg(e.to_string()))?
}

#[tauri::command]
pub async fn doctor_fix(app: AppHandle, fix_id: String) -> AppResult<String> {
    let msg = tauri::async_runtime::spawn_blocking(move || apply_fix(&fix_id))
        .await
        .map_err(|e| crate::error::AppError::msg(e.to_string()))??;
    desktop::emit_toast(
        &app,
        ToastPayload {
            id: "fix".into(),
            title: "Doctor applied a fix".into(),
            body: msg.clone(),
            kind: "ok".into(),
            page: Some("doctor".into()),
        },
    );
    Ok(msg)
}

#[tauri::command]
pub fn spark_create(app: AppHandle, request: SparkRequest) -> AppResult<SparkResult> {
    let result = create_project(request)?;
    desktop::emit_toast(
        &app,
        ToastPayload {
            id: "project".into(),
            title: "Project created".into(),
            body: result.path.clone(),
            kind: "ok".into(),
            page: Some("projects".into()),
        },
    );
    Ok(result)
}

#[tauri::command]
pub fn playground_eval(code: String) -> AppResult<String> {
    eval_snippet(code)
}

#[tauri::command]
pub async fn hex_search(query: String, force: bool) -> AppResult<Vec<HexPackage>> {
    search_packages(query, force).await
}

#[tauri::command]
pub async fn hex_package(name: String) -> AppResult<HexPackage> {
    get_package(name).await
}

#[tauri::command]
pub fn open_path(path: String) -> AppResult<()> {
    #[cfg(windows)]
    {
        let native = path.replace('/', "\\");
        let target = std::path::PathBuf::from(&native);
        let mut cmd = std::process::Command::new("explorer");
        if target.is_file() {
            cmd.arg(format!("/select,{native}"));
        } else {
            cmd.arg(&native);
        }
        cmd.spawn()
            .map_err(|e| crate::error::AppError::msg(e.to_string()))?;
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        Err(crate::error::AppError::msg(
            "Opening folders from Elin is implemented on Windows in this build.",
        ))
    }
}

#[tauri::command]
pub async fn startup_probe() -> AppResult<StartupProbe> {
    tauri::async_runtime::spawn_blocking(probe_machine)
        .await
        .map_err(|e| crate::error::AppError::msg(e.to_string()))?
}

#[tauri::command]
pub fn cache_status() -> CacheStatus {
    cache::status()
}

#[tauri::command]
pub fn cache_clear() {
    cache::clear();
}

#[tauri::command]
pub fn add_bin_to_path(name: String) -> AppResult<String> {
    crate::services::probe::add_hit_to_path(&name)
}

#[tauri::command]
pub fn add_elin_to_path() -> AppResult<String> {
    crate::services::env::add_elin_to_path()
}

#[tauri::command]
pub async fn check_app_update(force: bool) -> AppResult<crate::services::update::AppUpdate> {
    crate::services::update::check(force).await
}

#[tauri::command]
pub async fn download_app_update(app: AppHandle, force: bool) -> AppResult<String> {
    let path = crate::services::update::download(&app, force).await?;
    Ok(path.to_string_lossy().into())
}

#[tauri::command]
pub fn install_app_update(app: AppHandle, path: String) -> AppResult<()> {
    crate::services::update::launch_installer(&app, std::path::Path::new(&path))
}

#[tauri::command]
pub fn show_toast(app: AppHandle, toast: ToastPayload) {
    desktop::emit_toast(&app, toast);
}

#[tauri::command]
pub fn hide_toast_window(app: AppHandle) {
    desktop::hide_toast(&app);
}

#[tauri::command]
pub fn focus_main(app: AppHandle) -> AppResult<()> {
    desktop::show_main(&app).map_err(|e| crate::error::AppError::msg(e.to_string()))
}

#[tauri::command]
pub fn last_toast() -> Option<ToastPayload> {
    desktop::last_toast()
}

#[tauri::command]
pub fn open_page(app: AppHandle, page: String) -> AppResult<()> {
    let _ = app.emit("elin-open", &page);
    desktop::hide_toast(&app);
    desktop::show_main(&app).map_err(|e| crate::error::AppError::msg(e.to_string()))
}

#[tauri::command]
pub fn quit_app(app: AppHandle) {
    app.exit(0);
}
