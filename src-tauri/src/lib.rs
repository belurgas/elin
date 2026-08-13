//! Elin — a beginner-friendly Elixir + Erlang toolchain companion.
//!
//! The crate is split into:
//! - [`domain`] — version numbers and the official compatibility table
//! - [`services`] — network, install, PATH, IDEs, plugins, doctor, Hex, Studio
//! - [`commands`] — the Tauri IPC surface the React UI calls
//! - [`cli`] — same-binary `elin` subcommands (no args still opens the GUI)
//! - [`desktop`] — custom tray + toast windows

mod commands;
mod desktop;
mod domain;
mod error;
mod instance;
mod services;
mod term;
pub mod cli;

use tauri::Manager;

/// Application entry used by both the desktop binary and the staticlib.
///
/// A second `elin` with no args focuses the existing window instead of
/// starting another GUI.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    if !instance::try_claim() {
        // Production: another Elin is already up — focus it and stop.
        // `tauri dev` relaunch races the previous process; exiting here leaves
        // Vite talking to a dead backend (Hex, playground, workspace all hang).
        if !cfg!(debug_assertions) && instance::focus() {
            return;
        }
    }
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .invoke_handler(tauri::generate_handler![
            commands::get_host_info,
            commands::fetch_version_catalog,
            commands::install_toolchain,
            commands::list_toolchains,
            commands::activate_toolchain,
            commands::remove_toolchain,
            commands::scan_studios,
            commands::import_studio,
            commands::list_plugins,
            commands::install_studio_plugin,
            commands::get_neovim_snippet,
            commands::doctor_report,
            commands::doctor_fix,
            commands::spark_create,
            commands::playground_eval,
            commands::hex_search,
            commands::hex_package,
            commands::open_path,
            commands::startup_probe,
            commands::cache_status,
            commands::cache_clear,
            commands::list_projects,
            commands::scan_projects_quick,
            commands::scan_projects_deep,
            commands::cancel_project_scan,
            commands::inspect_project,
            commands::install_project_toolchain,
            commands::pin_project_toolchain,
            commands::project_graph,
            commands::open_project_in_studio,
            commands::add_project,
            commands::star_project,
            commands::project_git,
            commands::project_commit,
            commands::project_scan,
            commands::project_format,
            commands::list_kits,
            commands::kit_catalog,
            commands::apply_project_kits,
            commands::remove_project_kit,
            commands::write_kit_config,
            commands::set_credo_strict,
            commands::take_open_project,
            commands::open_project_workspace,
            commands::workspace_context,
            commands::project_mix,
            commands::project_shell,
            commands::add_hex_dep,
            commands::remove_hex_dep,
            commands::workspace_watch_start,
            commands::workspace_watch_stop,
            commands::git_licenses,
            commands::git_init,
            commands::add_elin_comment,
            commands::add_elin_to_path,
            commands::add_bin_to_path,
            commands::check_app_update,
            commands::download_app_update,
            commands::install_app_update,
            commands::show_toast,
            commands::hide_toast_window,
            commands::last_toast,
            commands::open_page,
            commands::focus_main,
            commands::quit_app,
        ])
        .setup(|app| {
            crate::services::store::write_gui_pid();
            let handle = app.handle().clone();
            instance::spawn_wake_listener({
                let handle = handle.clone();
                move || {
                    let handle = handle.clone();
                    let shown = handle.clone();
                    let _ = handle.run_on_main_thread(move || {
                        let _ = desktop::show_main(&shown);
                    });
                }
            });
            if let Err(err) = desktop::setup_shell_windows(&handle) {
                eprintln!("shell windows: {err}");
            }
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_title("Elin");
            }
            if let Some(path) = crate::services::projects::take_open_request() {
                let _ = crate::services::workspace::open(&handle, &path);
            }
            Ok(())
        })
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::CloseRequested { api, .. } if window.label() == "main" => {
                api.prevent_close();
                desktop::hide_main_to_tray(window.app_handle());
            }
            tauri::WindowEvent::Focused(true) => {
                let label = window.label();
                if label != "tray" && label != "toast" {
                    desktop::hide_tray_on_blur(window.app_handle());
                }
                if label == "main" {
                    if let Some(path) = crate::services::projects::take_open_request() {
                        let _ = crate::services::workspace::open(window.app_handle(), &path);
                    }
                }
            }
            tauri::WindowEvent::Focused(false) if window.label() == "tray" => {
                desktop::hide_tray_on_blur(window.app_handle());
            }
            tauri::WindowEvent::Destroyed if window.label().starts_with("ws-") => {
                if let Some(path) = crate::services::workspace::path_for_label(window.label()) {
                    crate::services::watch::stop(&path);
                }
            }
            _ => {}
        })
        .run(tauri::generate_context!())
        .expect("error while running Elin");
}
