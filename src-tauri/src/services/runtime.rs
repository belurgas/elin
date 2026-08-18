//! First Spark (new Mix/Phoenix projects) and the in-app IEx playground.

use crate::error::{AppError, AppResult};
use crate::services::install::list_installed;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SparkRequest {
    pub name: String,
    pub directory: String,
    pub template: String,
    #[serde(default)]
    pub kits: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SparkResult {
    pub path: String,
    pub output: String,
}

fn toolchain_bins() -> AppResult<(PathBuf, PathBuf)> {
    let installed = list_installed()?;
    let active = installed
        .iter()
        .find(|p| p.is_active)
        .or_else(|| installed.first())
        .ok_or_else(|| AppError::msg("Install Elixir first — Spark needs Mix."))?;
    Ok((
        PathBuf::from(&active.otp_path),
        PathBuf::from(&active.elixir_path),
    ))
}

fn mix_run(otp_bin: &Path, elixir_bin: &Path, cwd: Option<&Path>, args: &[&str]) -> AppResult<std::process::Output> {
    let mix = crate::services::host::mix_cmd(elixir_bin);
    let path = crate::services::winproc::isolated_path(otp_bin, elixir_bin);
    let home = crate::services::winproc::erlang_home(otp_bin);
    let child = crate::services::winproc::spawn_bat(&mix, args, &path, home.as_deref(), cwd)
        .map_err(|e| AppError::msg(e.to_string()))?;
    child.wait_with_output().map_err(|e| AppError::msg(e.to_string()))
}

/// Create a new Mix or Phoenix project in the chosen folder.
pub fn create_project(req: SparkRequest) -> AppResult<SparkResult> {
    let name = sanitize_app_name(&req.name)?;
    let parent = PathBuf::from(&req.directory);
    if !parent.exists() {
        std::fs::create_dir_all(&parent)?;
    }
    let (otp_bin, elixir_bin) = toolchain_bins()?;

    let mut args: Vec<&str> = Vec::new();
    match req.template.as_str() {
        "mix" => {
            args.extend(["new", name.as_str()]);
        }
        "mix-sup" => {
            args.extend(["new", name.as_str(), "--sup"]);
        }
        "phoenix" | "phoenix-live" => {
            let _ = mix_run(&otp_bin, &elixir_bin, None, &["archive.install", "hex", "phx_new", "--force"]);
            args.extend(["phx.new", name.as_str(), "--install"]);
            if req.template == "phoenix" {
                args.push("--no-live");
            }
        }
        other => {
            return Err(AppError::msg(format!("Unknown template: {other}")));
        }
    }

    let output = mix_run(&otp_bin, &elixir_bin, Some(&parent), &args)?;
    let text = crate::services::winproc::output_text(&output);
    if !output.status.success() {
        return Err(AppError::Install(text));
    }
    let path = parent.join(&name);
    let mut output_text = text;
    let phoenix = req.template.starts_with("phoenix");
    let kits = if req.kits.is_empty() {
        crate::services::kits::default_ids(phoenix)
    } else {
        req.kits.clone()
    };
    if !kits.is_empty() {
        match crate::services::kits::apply_kits(&path.to_string_lossy(), &kits, true) {
            Ok(kit_log) => {
                output_text.push_str("\n\n");
                output_text.push_str(&kit_log);
            }
            Err(err) => {
                output_text.push_str("\n\nkit apply: ");
                output_text.push_str(&err.to_string());
            }
        }
    }
    let _ = crate::services::projects::add_project(&path.to_string_lossy());
    Ok(SparkResult {
        path: path.to_string_lossy().into(),
        output: output_text,
    })
}

fn sanitize_app_name(name: &str) -> AppResult<String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(AppError::msg("Give the project a name, like hello_phoenix"));
    }
    if !trimmed.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(AppError::msg(
            "Use only letters, numbers, and underscores (Elixir module-friendly).",
        ));
    }
    Ok(trimmed.to_string())
}

/// Run a short Elixir snippet with a hard timeout so the UI cannot hang.
///
/// Code is written to a temp `.exs` file so `cmd.exe` never sees `& | ( ) "` from the snippet.
pub fn eval_snippet(code: String) -> AppResult<String> {
    if code.len() > 16_384 {
        return Err(AppError::msg("Snippet is too long (16 KB max)."));
    }
    let (otp_bin, elixir_bin) = toolchain_bins()?;
    let elixir = crate::services::host::elixir_cmd(&elixir_bin);
    if !elixir.exists() {
        return Err(AppError::msg("elixir was not found next to the toolchain."));
    }
    let dir = std::env::temp_dir().join("elin-play");
    std::fs::create_dir_all(&dir)?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let file = dir.join(format!("snippet-{}-{}.exs", std::process::id(), stamp));
    std::fs::write(&file, code)?;
    let path = crate::services::winproc::isolated_path(&otp_bin, &elixir_bin);
    let home = crate::services::winproc::erlang_home(&otp_bin);
    let file_arg = file.to_string_lossy().into_owned();
    let child = match crate::services::winproc::spawn_bat(&elixir, &[&file_arg], &path, home.as_deref(), None) {
        Ok(child) => child,
        Err(err) => {
            let _ = std::fs::remove_file(&file);
            return Err(err.into());
        }
    };
    let result = crate::services::mixcmd::wait_lines(
        child,
        std::time::Duration::from_secs(8),
        "The snippet ran longer than 8 seconds and was stopped.".into(),
        "The snippet failed.".into(),
        |_| {},
    );
    let _ = std::fs::remove_file(&file);
    result
}
