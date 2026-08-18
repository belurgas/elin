//! Plugin Hub: curated Elixir add-ons per editor family, plus one-click
//! install for VS Code-compatible CLIs.

use crate::error::{AppError, AppResult};
use crate::services::studios::{Studio, StudioFamily};
use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Plugin {
    pub id: String,
    pub name: String,
    pub publisher: String,
    pub family: StudioFamily,
    pub marketplace_id: Option<String>,
    pub url: String,
    pub summary: String,
    pub why: String,
    pub recommended: bool,
    pub beginner: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginStatus {
    pub plugin: Plugin,
    pub installed_in: Vec<String>,
}

fn plug(
    id: &str,
    name: &str,
    publisher: &str,
    family: StudioFamily,
    marketplace_id: Option<&str>,
    url: &str,
    summary: &str,
    recommended: bool,
    beginner: bool,
) -> Plugin {
    Plugin {
        id: id.into(),
        name: name.into(),
        publisher: publisher.into(),
        family,
        marketplace_id: marketplace_id.map(str::to_string),
        url: url.into(),
        summary: summary.into(),
        why: why_for(id).into(),
        recommended,
        beginner,
    }
}

/// Curated catalog. IDs are stable so the UI can remember selections.
pub fn catalog() -> Vec<Plugin> {
    vec![
        plug(
            "elixir-ls",
            "ElixirLS",
            "Elixir Language Server",
            StudioFamily::Vscode,
            Some("JakeBecker.elixir-ls"),
            "https://marketplace.visualstudio.com/items?itemName=JakeBecker.elixir-ls",
            "The language server: completion, go-to-definition, formatting, Dialyzer.",
            true,
            true,
        ),
        plug(
            "vscode-elixir-ls-official",
            "ElixirLS (elixir-lsp)",
            "elixir-lsp",
            StudioFamily::Vscode,
            Some("elixir-lsp.elixir-ls"),
            "https://marketplace.visualstudio.com/items?itemName=elixir-lsp.elixir-ls",
            "Alternate Marketplace listing of ElixirLS. Install one ElixirLS, not both.",
            false,
            false,
        ),
        plug(
            "elixir-snippets",
            "Elixir Snippets",
            "florinpatrascu",
            StudioFamily::Vscode,
            Some("florinpatrascu.vscode-elixir-snippets"),
            "https://marketplace.visualstudio.com/items?itemName=florinpatrascu.vscode-elixir-snippets",
            "Handy snippets for modules, genserver, tests, and Phoenix.",
            true,
            true,
        ),
        plug(
            "credo",
            "Credo",
            "pantajoe",
            StudioFamily::Vscode,
            Some("pantajoe.vscode-elixir-credo"),
            "https://marketplace.visualstudio.com/items?itemName=pantajoe.vscode-elixir-credo",
            "Inline Credo linting so style issues show up while you type.",
            true,
            false,
        ),
        plug(
            "erlang-vscode",
            "Erlang",
            "pgourlain",
            StudioFamily::Vscode,
            Some("pgourlain.erlang"),
            "https://marketplace.visualstudio.com/items?itemName=pgourlain.erlang",
            "Syntax and tools for the rare `.erl` file you will open next to Elixir.",
            false,
            false,
        ),
        plug(
            "jetbrains-elixir",
            "Elixir plugin",
            "KronicDeth",
            StudioFamily::Jetbrains,
            None,
            "https://plugins.jetbrains.com/plugin/7522-elixir",
            "SDK detection, Mix, ElixirLS integration, and Phoenix support in IntelliJ.",
            true,
            true,
        ),
        plug(
            "elixir-tools-nvim",
            "elixir-tools.nvim",
            "elixir-tools",
            StudioFamily::Neovim,
            None,
            "https://github.com/elixir-tools/elixir-tools.nvim",
            "Next-LS / ElixirLS, Mix commands, and projectionist-style navigation.",
            true,
            true,
        ),
        plug(
            "zed-elixir",
            "Elixir extension",
            "Zed",
            StudioFamily::Zed,
            None,
            "https://zed.dev/extensions?query=elixir",
            "Official Elixir grammar + LSP wiring inside Zed.",
            true,
            true,
        ),
        plug(
            "emacs-elixir",
            "elixir-mode + eglot",
            "elixir-editors",
            StudioFamily::Emacs,
            None,
            "https://github.com/elixir-editors/emacs-elixir",
            "Major mode plus ElixirLS through eglot or lsp-mode.",
            true,
            true,
        ),
        plug(
            "sublime-elixir",
            "Elixir (Package Control)",
            "elixir-editors",
            StudioFamily::Sublime,
            None,
            "https://packagecontrol.io/packages/Elixir",
            "Syntax highlighting and snippets via Package Control.",
            true,
            true,
        ),
    ]
}

fn why_for(id: &str) -> &'static str {
    match id {
        "elixir-ls" => "This is the one plugin a beginner actually needs: jump-to-def, format on save, Dialyzer.",
        "vscode-elixir-ls-official" => "Same language server, different Marketplace listing. Install only one ElixirLS.",
        "elixir-snippets" => "Saves typing for defmodule, GenServer, ExUnit, and Phoenix plugs.",
        "credo" => "Shows Credo notes inline so style issues do not wait for CI.",
        "erlang-vscode" => "Useful the day you open a .erl file next to your Mix project.",
        "jetbrains-elixir" => "SDK + Mix + ElixirLS inside IntelliJ / WebStorm.",
        "elixir-tools-nvim" => "The Neovim stack: Next-LS or ElixirLS, Mix, projectionist.",
        "zed-elixir" => "Grammar + LSP so Zed is not a plain text editor for .ex files.",
        "emacs-elixir" => "elixir-mode plus eglot talking to ElixirLS.",
        "sublime-elixir" => "Syntax and snippets via Package Control.",
        _ => "Helps Elixir development in this editor family.",
    }
}

/// Merge the catalog with on-disk extension folders (no CLI — those hang for seconds).
pub fn status_for(studios: &[Studio]) -> Vec<PluginStatus> {
    catalog()
        .into_iter()
        .map(|plugin| {
            let installed_in = studios
                .iter()
                .filter(|studio| studio.detected && studio.family == plugin.family)
                .filter(|studio| {
                    plugin
                        .marketplace_id
                        .as_ref()
                        .map(|id| folder_has_extension(studio, id))
                        .unwrap_or(false)
                })
                .map(|s| s.id.clone())
                .collect();
            PluginStatus {
                plugin,
                installed_in,
            }
        })
        .collect()
}

fn folder_has_extension(studio: &Studio, marketplace_id: &str) -> bool {
    let Some(home) = dirs::home_dir() else {
        return false;
    };
    let needle = marketplace_id.to_lowercase();
    let dirs = match studio.id.as_str() {
        "vscode" => vec![home.join(r".vscode\extensions")],
        "cursor" => vec![home.join(r".cursor\extensions")],
        "vscodium" => vec![home.join(r".vscode-oss\extensions")],
        "windsurf" => vec![
            home.join(r".windsurf\extensions"),
            home.join(r".codeium\windsurf\extensions"),
        ],
        "antigravity" => vec![
            home.join(r".antigravity\extensions"),
            home.join(r".antigravity-editor\extensions"),
        ],
        _ if studio.family == StudioFamily::Vscode => vec![
            home.join(r".vscode\extensions"),
            home.join(r".cursor\extensions"),
        ],
        _ => vec![],
    };
    for dir in dirs {
        let Ok(entries) = std::fs::read_dir(dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_lowercase();
            if name.starts_with(&needle) || name.contains(&needle.replace('.', "-")) {
                return true;
            }
        }
    }
    false
}

/// Install a VS Code-family extension into the selected studio CLI.
pub fn install_plugin(studio: &Studio, marketplace_id: &str) -> AppResult<String> {
    if studio.family != StudioFamily::Vscode {
        return Err(AppError::msg(
            "One-click install is available for VS Code-compatible editors. For others, Elin opens the plugin page.",
        ));
    }
    let cli = studio
        .cli
        .as_ref()
        .ok_or_else(|| AppError::msg(format!("{} was detected but its CLI was not found", studio.name)))?;
    let script = crate::services::winproc::is_shell_script(std::path::Path::new(cli));
    let mut cmd = if script {
        let mut c = Command::new("cmd.exe");
        c.arg("/D")
            .arg("/C")
            .arg(cli)
            .args(["--install-extension", marketplace_id, "--force"]);
        c
    } else {
        let mut c = Command::new(cli);
        c.args(["--install-extension", marketplace_id, "--force"]);
        c
    };
    crate::services::winproc::hide_console(&mut cmd);
    let output = cmd.output()?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if !output.status.success() {
        return Err(AppError::msg(format!(
            "Extension install failed: {}",
            stderr.trim().if_empty_text(stdout.trim())
        )));
    }
    Ok(format!("{stdout}{stderr}"))
}

pub fn neovim_snippet() -> &'static str {
    r#"-- elixir-tools.nvim (paste into your Neovim config)
{
  "elixir-tools/elixir-tools.nvim",
  version = "*",
  event = { "BufReadPre", "BufNewFile" },
  config = function()
    require("elixir").setup({
      nextls = { enable = true },
      elixirls = { enable = true },
      projectionist = { enable = true },
    })
  end,
  dependencies = { "nvim-lua/plenary.nvim" },
}
"#
}

trait IfEmptyText {
    fn if_empty_text<'a>(&'a self, other: &'a str) -> &'a str;
}

impl IfEmptyText for str {
    fn if_empty_text<'a>(&'a self, other: &'a str) -> &'a str {
        if self.is_empty() { other } else { self }
    }
}
