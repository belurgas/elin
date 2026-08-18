//! Studio Scout: detect editors that can host Elixir plugins, extract their
//! icons, and let the user pick extra executables.

use crate::error::{AppError, AppResult};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// How an editor can receive Elixir tooling.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum StudioFamily {
    Vscode,
    Jetbrains,
    Neovim,
    Zed,
    Emacs,
    Sublime,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Studio {
    pub id: String,
    pub name: String,
    pub family: StudioFamily,
    pub executable: Option<String>,
    pub cli: Option<String>,
    pub detected: bool,
    pub plugin_capable: bool,
    pub icon_data_url: Option<String>,
    pub notes: String,
}

struct Recipe {
    id: &'static str,
    name: &'static str,
    family: StudioFamily,
    notes: &'static str,
    exe_names: &'static [&'static str],
    extra_paths: fn() -> Vec<PathBuf>,
    cli_names: &'static [&'static str],
}

fn local_app_data() -> PathBuf {
    dirs::data_local_dir().unwrap_or_else(|| PathBuf::from(r"C:\Users\Default\AppData\Local"))
}

fn program_files() -> PathBuf {
    PathBuf::from(std::env::var("ProgramFiles").unwrap_or_else(|_| r"C:\Program Files".into()))
}

fn program_files_x86() -> PathBuf {
    PathBuf::from(
        std::env::var("ProgramFiles(x86)").unwrap_or_else(|_| r"C:\Program Files (x86)".into()),
    )
}

fn recipes() -> Vec<Recipe> {
    vec![
        Recipe {
            id: "vscode",
            name: "Visual Studio Code",
            family: StudioFamily::Vscode,
            notes: "Best-in-class ElixirLS experience. One-click plugin install is supported.",
            exe_names: &["Code.exe", "code.exe", "code"],
            extra_paths: || {
                let mut p = vec![
                    local_app_data().join(r"Programs\Microsoft VS Code\Code.exe"),
                    local_app_data().join(r"Programs\Microsoft VS Code Insiders\Code - Insiders.exe"),
                    program_files().join(r"Microsoft VS Code\Code.exe"),
                    program_files_x86().join(r"Microsoft VS Code\Code.exe"),
                    local_app_data().join(r"Microsoft\WindowsApps\Code.exe"),
                    dirs::home_dir().unwrap_or_default().join(r"scoop\apps\vscode\current\Code.exe"),
                ];
                p.extend(unix_bins("Visual Studio Code", "code", "Electron"));
                p
            },
            cli_names: &["code.cmd", "code", "code.exe"],
        },
        Recipe {
            id: "cursor",
            name: "Cursor",
            family: StudioFamily::Vscode,
            notes: "VS Code-compatible. ElixirLS installs through the Cursor CLI (`cursor --install-extension`).",
            exe_names: &["Cursor.exe", "cursor.exe", "cursor"],
            extra_paths: || {
                let mut p = vec![
                    local_app_data().join(r"Programs\cursor\Cursor.exe"),
                    local_app_data().join(r"Programs\Cursor\Cursor.exe"),
                    program_files().join(r"Cursor\Cursor.exe"),
                ];
                p.extend(unix_bins("Cursor", "cursor", "Cursor"));
                p
            },
            cli_names: &["cursor.cmd", "cursor", "cursor.exe"],
        },
        Recipe {
            id: "vscodium",
            name: "VSCodium",
            family: StudioFamily::Vscode,
            notes: "Open-source VS Code build. Uses the codium CLI for extensions.",
            exe_names: &["VSCodium.exe", "codium"],
            extra_paths: || {
                let mut p = vec![
                    local_app_data().join(r"Programs\VSCodium\VSCodium.exe"),
                    program_files().join(r"VSCodium\VSCodium.exe"),
                ];
                p.extend(unix_bins("VSCodium", "codium", "Electron"));
                p
            },
            cli_names: &["codium.cmd", "codium"],
        },
        Recipe {
            id: "windsurf",
            name: "Windsurf",
            family: StudioFamily::Vscode,
            notes: "Codeium editor. VS Code extensions, including ElixirLS, are supported.",
            exe_names: &["Windsurf.exe", "windsurf"],
            extra_paths: || {
                let mut p = vec![local_app_data().join(r"Programs\Windsurf\Windsurf.exe")];
                p.extend(unix_bins("Windsurf", "windsurf", "Windsurf"));
                p
            },
            cli_names: &["windsurf.cmd", "windsurf"],
        },
        Recipe {
            id: "antigravity",
            name: "Antigravity",
            family: StudioFamily::Vscode,
            notes: "VS Code-compatible editor. Extensions install via its CLI when present.",
            exe_names: &["Antigravity.exe"],
            extra_paths: || vec![local_app_data().join(r"Programs\Antigravity\Antigravity.exe")],
            cli_names: &["antigravity"],
        },
        Recipe {
            id: "intellij",
            name: "IntelliJ IDEA",
            family: StudioFamily::Jetbrains,
            notes: "Install the Elixir plugin from JetBrains Marketplace. Elin will open the page.",
            exe_names: &["idea64.exe", "idea.exe", "idea"],
            extra_paths: || jetbrains_paths("IntelliJ"),
            cli_names: &["idea64.exe"],
        },
        Recipe {
            id: "webstorm",
            name: "WebStorm",
            family: StudioFamily::Jetbrains,
            notes: "The Elixir plugin is available on JetBrains Marketplace.",
            exe_names: &["webstorm64.exe", "webstorm"],
            extra_paths: || jetbrains_paths("WebStorm"),
            cli_names: &[],
        },
        Recipe {
            id: "neovim",
            name: "Neovim",
            family: StudioFamily::Neovim,
            notes: "Use elixir-tools.nvim or Mason's elixir-ls. Elin copies a starter snippet.",
            exe_names: &["nvim.exe", "nvim"],
            extra_paths: || {
                let mut p = vec![
                    program_files().join(r"Neovim\bin\nvim.exe"),
                    local_app_data().join(r"Programs\Neovim\bin\nvim.exe"),
                ];
                p.extend(unix_bins("nvim", "nvim", "nvim"));
                p
            },
            cli_names: &["nvim"],
        },
        Recipe {
            id: "zed",
            name: "Zed",
            family: StudioFamily::Zed,
            notes: "Zed has a first-party Elixir extension. Elin can open the extensions UI.",
            exe_names: &["Zed.exe", "zed"],
            extra_paths: || {
                let mut p = vec![local_app_data().join(r"Programs\Zed\Zed.exe")];
                p.extend(unix_bins("Zed", "zed", "zed"));
                p
            },
            cli_names: &["zed"],
        },
        Recipe {
            id: "sublime",
            name: "Sublime Text",
            family: StudioFamily::Sublime,
            notes: "Install the Elixir package via Package Control.",
            exe_names: &["sublime_text.exe", "subl", "sublime_text"],
            extra_paths: || {
                let mut p = vec![
                    program_files().join(r"Sublime Text\sublime_text.exe"),
                    program_files().join(r"Sublime Text 3\sublime_text.exe"),
                ];
                p.extend(unix_bins("Sublime Text", "subl", "sublime_text"));
                p
            },
            cli_names: &["subl"],
        },
        Recipe {
            id: "emacs",
            name: "Emacs",
            family: StudioFamily::Emacs,
            notes: "elixir-mode + eglot/lsp-mode talking to ElixirLS.",
            exe_names: &["emacs.exe", "runemacs.exe", "emacs"],
            extra_paths: || {
                let mut p = vec![program_files().join(r"Emacs\bin\runemacs.exe")];
                p.extend(unix_bins("Emacs", "emacs", "Emacs"));
                p
            },
            cli_names: &["emacs"],
        },
        Recipe {
            id: "helix",
            name: "Helix",
            family: StudioFamily::Other,
            notes: "Helix uses ElixirLS through languages.toml. Plugin-capable via LSP.",
            exe_names: &["hx.exe", "hx"],
            extra_paths: || vec![],
            cli_names: &["hx"],
        },
    ]
}

fn unix_bins(app: &str, cli: &str, macos_bin: &str) -> Vec<PathBuf> {
    let mut out = vec![
        PathBuf::from(format!("/Applications/{app}.app/Contents/MacOS/{macos_bin}")),
        PathBuf::from(format!("/Applications/{app}.app/Contents/Resources/app/bin/{cli}")),
        PathBuf::from(format!("/usr/local/bin/{cli}")),
        PathBuf::from(format!("/opt/homebrew/bin/{cli}")),
        PathBuf::from(format!("/usr/bin/{cli}")),
        PathBuf::from(format!("/snap/bin/{cli}")),
        dirs::home_dir().unwrap_or_default().join(".local").join("bin").join(cli),
    ];
    if let Some(home) = dirs::home_dir() {
        out.push(home.join("Applications").join(format!("{app}.app")).join("Contents").join("MacOS").join(macos_bin));
    }
    out
}

fn jetbrains_paths(product: &str) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let toolbox = local_app_data().join(r"Programs\JetBrains Toolbox\apps");
    if toolbox.exists() {
        for entry in walkdir::WalkDir::new(&toolbox).max_depth(6).into_iter().flatten() {
            let name = entry.file_name().to_string_lossy();
            if name.eq_ignore_ascii_case("idea64.exe")
                || name.eq_ignore_ascii_case("webstorm64.exe")
            {
                if entry.path().to_string_lossy().to_lowercase().contains(&product.to_lowercase())
                {
                    out.push(entry.path().to_path_buf());
                }
            }
        }
    }
    let _ = program_files_x86();
    out
}

/// Detect known studios and attach icons when possible.
pub fn detect_studios() -> Vec<Studio> {
    let mut studios: Vec<Studio> = recipes()
        .into_iter()
        .map(|recipe| {
            let (exe, cli) = locate_pair(&recipe);
            let detected = match recipe.id {
                "antigravity" | "windsurf" | "helix" => exe.is_some(),
                _ => exe.is_some() || cli.is_some(),
            };
            Studio {
                id: recipe.id.into(),
                name: recipe.name.into(),
                family: recipe.family,
                executable: exe.map(|p| p.to_string_lossy().into()),
                cli: cli.map(|p| p.to_string_lossy().into()),
                detected,
                plugin_capable: true,
                icon_data_url: None,
                notes: recipe.notes.into(),
            }
        })
        .collect();

    // Catch VS Code-family editors sitting under Local\Programs that recipes missed.
    discover_loose_exes(&mut studios);
    apply_registry_hits(&mut studios);

    let paths: Vec<PathBuf> = studios
        .iter()
        .filter_map(|s| s.executable.as_ref().map(PathBuf::from))
        .collect();
    let icons = extract_icons_timed(&paths, std::time::Duration::from_secs(3));
    for studio in &mut studios {
        if let Some(exe) = studio.executable.as_ref() {
            studio.icon_data_url = icons.get(exe).cloned();
        }
    }
    studios.sort_by(|a, b| b.detected.cmp(&a.detected).then(a.name.cmp(&b.name)));
    studios
}

fn locate_pair(recipe: &Recipe) -> (Option<PathBuf>, Option<PathBuf>) {
    let mut exe = locate_exe(recipe);
    let mut cli = exe
        .as_ref()
        .and_then(|p| locate_cli(p, recipe.cli_names))
        .or_else(|| which_first(recipe.cli_names));

    if exe.is_none() {
        if let Some(cli_path) = &cli {
            exe = exe_from_cli(cli_path, recipe.exe_names);
        }
    }
    if cli.is_none() {
        if let Some(exe_path) = &exe {
            cli = locate_cli(exe_path, recipe.cli_names);
        }
    }
    (exe, cli)
}

fn exe_from_cli(cli: &Path, exe_names: &[&str]) -> Option<PathBuf> {
    let mut dir = cli.parent()?;
    for _ in 0..6 {
        for name in exe_names {
            let candidate = dir.join(name);
            if candidate.exists() {
                return Some(candidate);
            }
        }
        dir = dir.parent()?;
    }
    None
}

fn discover_loose_exes(studios: &mut [Studio]) {
    let programs = local_app_data().join("Programs");
    if !programs.exists() {
        return;
    }
    for entry in walkdir::WalkDir::new(programs).max_depth(3).into_iter().flatten() {
        let name = entry.file_name().to_string_lossy();
        let path = entry.path();
        if !name.eq_ignore_ascii_case("Cursor.exe")
            && !name.eq_ignore_ascii_case("Code.exe")
            && !name.eq_ignore_ascii_case("Code - Insiders.exe")
            && !name.eq_ignore_ascii_case("VSCodium.exe")
        {
            continue;
        }
        let id = if name.to_lowercase().contains("cursor") {
            "cursor"
        } else if name.to_lowercase().contains("codium") {
            "vscodium"
        } else if name.to_lowercase().contains("insider") {
            "vscode"
        } else {
            "vscode"
        };
        if let Some(studio) = studios.iter_mut().find(|s| s.id == id) {
            if studio.executable.is_none() {
                studio.executable = Some(path.to_string_lossy().into());
                studio.detected = true;
            }
        }
    }
}

fn apply_registry_hits(studios: &mut [Studio]) {
    for (id, exe) in registry_install_locations() {
        if let Some(studio) = studios.iter_mut().find(|s| s.id == id) {
            if studio.executable.is_none() && exe.exists() {
                studio.executable = Some(exe.to_string_lossy().into());
                studio.detected = true;
            }
        }
    }
}

#[cfg(windows)]
fn registry_install_locations() -> Vec<(String, PathBuf)> {
    use winreg::enums::*;
    use winreg::RegKey;
    let mut out = Vec::new();
    let keys = [
        (RegKey::predef(HKEY_CURRENT_USER), r"Software\Microsoft\Windows\CurrentVersion\Uninstall"),
        (RegKey::predef(HKEY_LOCAL_MACHINE), r"Software\Microsoft\Windows\CurrentVersion\Uninstall"),
        (RegKey::predef(HKEY_LOCAL_MACHINE), r"Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall"),
    ];
    for (root, path) in keys {
        let Ok(key) = root.open_subkey(path) else {
            continue;
        };
        for name in key.enum_keys().flatten() {
            let Ok(sub) = key.open_subkey(&name) else {
                continue;
            };
            let display: String = sub.get_value("DisplayName").unwrap_or_default();
            let location: String = sub.get_value("InstallLocation").unwrap_or_default();
            let icon: String = sub.get_value("DisplayIcon").unwrap_or_default();
            let hay = format!("{display} {location} {icon}").to_lowercase();
            let id = if hay.contains("cursor") && !hay.contains("visual studio") {
                "cursor"
            } else if hay.contains("visual studio code") || hay.contains("vscode") {
                "vscode"
            } else if hay.contains("vscodium") {
                "vscodium"
            } else {
                continue;
            };
            let mut candidates = Vec::new();
            if !location.is_empty() {
                let dir = PathBuf::from(location.trim_matches('"'));
                candidates.push(dir.join("Code.exe"));
                candidates.push(dir.join("Cursor.exe"));
                candidates.push(dir.join("Code - Insiders.exe"));
                candidates.push(dir.join("VSCodium.exe"));
            }
            if !icon.is_empty() {
                let icon_path = PathBuf::from(icon.split(',').next().unwrap_or(&icon).trim_matches('"'));
                if icon_path.extension().and_then(|e| e.to_str()).map(|e| e.eq_ignore_ascii_case("exe")).unwrap_or(false) {
                    candidates.push(icon_path);
                }
            }
            if let Some(exe) = candidates.into_iter().find(|p| p.exists()) {
                out.push((id.into(), exe));
            }
        }
    }
    out
}

#[cfg(not(windows))]
fn registry_install_locations() -> Vec<(String, PathBuf)> {
    Vec::new()
}

fn locate_exe(recipe: &Recipe) -> Option<PathBuf> {
    for path in (recipe.extra_paths)() {
        if path.exists() {
            return Some(path);
        }
    }
    for name in recipe.exe_names {
        if let Ok(found) = which::which(name) {
            return Some(found);
        }
    }
    None
}

fn locate_cli(exe: &Path, cli_names: &[&str]) -> Option<PathBuf> {
    let parent = exe.parent()?;
    let candidates = [
        parent.join("bin"),
        parent.join(r"resources\app\bin"),
        parent.to_path_buf(),
    ];
    for dir in candidates {
        for name in cli_names {
            let path = dir.join(name);
            if path.exists() {
                return Some(path);
            }
        }
    }
    which_first(cli_names)
}

fn which_first(names: &[&str]) -> Option<PathBuf> {
    names.iter().find_map(|n| which::which(n).ok())
}

/// Register a user-picked executable as a custom studio.
pub fn studio_from_executable(path: String) -> AppResult<Studio> {
    let exe = PathBuf::from(&path);
    if !exe.exists() {
        return Err(AppError::msg("That executable does not exist"));
    }
    let name = exe
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "Custom editor".into());
    let family = infer_family(&name, &exe);
    let icon_data_url = extract_icon_data_url(&exe);
    Ok(Studio {
        id: format!("custom-{}", name.to_lowercase().replace(' ', "-")),
        name,
        family,
        executable: Some(path),
        cli: None,
        detected: true,
        plugin_capable: true,
        icon_data_url,
        notes: "Added by you. Elin will still recommend Elixir plugins for this family.".into(),
    })
}

fn infer_family(name: &str, path: &Path) -> StudioFamily {
    let hay = format!("{} {}", name, path.to_string_lossy()).to_lowercase();
    if hay.contains("code") || hay.contains("cursor") || hay.contains("codium") || hay.contains("windsurf")
    {
        StudioFamily::Vscode
    } else if hay.contains("idea") || hay.contains("jetbrains") || hay.contains("storm") {
        StudioFamily::Jetbrains
    } else if hay.contains("nvim") || hay.contains("vim") {
        StudioFamily::Neovim
    } else if hay.contains("zed") {
        StudioFamily::Zed
    } else if hay.contains("emacs") {
        StudioFamily::Emacs
    } else if hay.contains("sublime") {
        StudioFamily::Sublime
    } else {
        StudioFamily::Other
    }
}

/// Extract the associated Windows icon and return a PNG data URL.
pub fn extract_icon_data_url(exe: &Path) -> Option<String> {
    extract_icons_batch(&[exe.to_path_buf()])
        .into_iter()
        .next()
        .map(|(_, url)| url)
}

fn cache_file_for(exe: &Path) -> Option<PathBuf> {
    let cache_dir = dirs::cache_dir()?.join("elin").join("icons");
    let _ = fs::create_dir_all(&cache_dir);
    let key = exe.to_string_lossy().replace(['\\', '/', ':'], "_");
    Some(cache_dir.join(format!("{key}.b64")))
}

fn extract_icons_timed(paths: &[PathBuf], timeout: std::time::Duration) -> HashMap<String, String> {
    if paths.is_empty() {
        return HashMap::new();
    }
    let paths = paths.to_vec();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(extract_icons_batch(&paths));
    });
    rx.recv_timeout(timeout).unwrap_or_default()
}

fn extract_icons_batch(paths: &[PathBuf]) -> HashMap<String, String> {
    let mut out = HashMap::new();
    let mut missing = Vec::new();
    for path in paths {
        if let Some(cache) = cache_file_for(path) {
            if let Ok(existing) = fs::read_to_string(&cache) {
                if existing.starts_with("data:image") {
                    out.insert(path.to_string_lossy().into(), existing);
                    continue;
                }
            }
        }
        missing.push(path.clone());
    }
    if missing.is_empty() {
        return out;
    }

    #[cfg(not(windows))]
    {
        let _ = missing;
        return out;
    }

    #[cfg(windows)]
    {
    let list = missing
        .iter()
        .map(|p| format!("'{}'", p.to_string_lossy().replace('\'', "''")))
        .collect::<Vec<_>>()
        .join(",");
    let script = format!(
        r#"
Add-Type -AssemblyName System.Drawing
$paths = @({list})
foreach ($p in $paths) {{
  try {{
    $icon = [System.Drawing.Icon]::ExtractAssociatedIcon($p)
    if ($null -eq $icon) {{ continue }}
    $ms = New-Object System.IO.MemoryStream
    $icon.ToBitmap().Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
    Write-Output ($p + '|' + [Convert]::ToBase64String($ms.ToArray()))
  }} catch {{}}
}}
"#
    );
    if let Ok(output) = {
        let mut cmd = Command::new("powershell");
        cmd.args(["-NoProfile", "-STA", "-WindowStyle", "Hidden", "-Command", &script]);
        crate::services::winproc::hide_console(&mut cmd);
        cmd.output()
    }
    {
        if output.status.success() {
            for line in String::from_utf8_lossy(&output.stdout).lines() {
                if let Some((path, b64)) = line.split_once('|') {
                    if BASE64.decode(b64.trim()).is_ok() {
                        let data_url = format!("data:image/png;base64,{}", b64.trim());
                        if let Some(cache) = cache_file_for(Path::new(path)) {
                            let _ = fs::write(cache, &data_url);
                        }
                        out.insert(path.to_string(), data_url);
                    }
                }
            }
        }
    }
    out
    }
}
