//! Same-binary CLI. `elin` with no args still opens the GUI.

use crate::error::AppResult;
use crate::instance;
use crate::services::env;
use crate::services::kits;
use crate::services::mixcmd::mix_in_project;
use crate::services::projects;
use crate::services::scan;
use crate::term;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Help,
    Add { path: Option<String> },
    List,
    Open { path: Option<String> },
    Scan { path: Option<String>, full: bool },
    Format { path: Option<String>, check: bool },
    KitList { path: Option<String> },
    KitAdd { id: String, path: Option<String> },
    KitRemove { id: String, path: Option<String> },
    Status { path: Option<String> },
    Path,
}

pub fn is_cli(args: &[String]) -> bool {
    match args.first().map(String::as_str) {
        None => false,
        Some("--webview-options") | Some("--tauri") => false,
        Some(flag) if flag.starts_with("--crash") => false,
        Some(_) => true,
    }
}

pub fn parse(args: &[String]) -> Result<Command, String> {
    let mut rest = args.iter().map(String::as_str).peekable();
    let Some(cmd) = rest.next() else {
        return Ok(Command::Help);
    };
    match cmd {
        "-h" | "--help" | "help" => Ok(Command::Help),
        "add" => Ok(Command::Add {
            path: rest.next().map(str::to_string),
        }),
        "list" => Ok(Command::List),
        "open" => Ok(Command::Open {
            path: rest.next().map(str::to_string),
        }),
        "scan" => {
            let mut path = None;
            let mut full = false;
            for arg in rest {
                match arg {
                    "--full" => full = true,
                    "--quick" => full = false,
                    other if !other.starts_with('-') => path = Some(other.to_string()),
                    other => return Err(format!("Unknown scan flag: {other}")),
                }
            }
            Ok(Command::Scan { path, full })
        }
        "format" => {
            let mut path = None;
            let mut check = false;
            for arg in rest {
                match arg {
                    "--check" => check = true,
                    other if !other.starts_with('-') => path = Some(other.to_string()),
                    other => return Err(format!("Unknown format flag: {other}")),
                }
            }
            Ok(Command::Format { path, check })
        }
        "kit" => match rest.next() {
            None | Some("list") => Ok(Command::KitList {
                path: rest.next().map(str::to_string),
            }),
            Some("add") => {
                let id = rest
                    .next()
                    .ok_or_else(|| "Usage: elin kit add <id> [path]".to_string())?
                    .to_string();
                Ok(Command::KitAdd {
                    id,
                    path: rest.next().map(str::to_string),
                })
            }
            Some("remove") => {
                let id = rest
                    .next()
                    .ok_or_else(|| "Usage: elin kit remove <id> [path]".to_string())?
                    .to_string();
                Ok(Command::KitRemove {
                    id,
                    path: rest.next().map(str::to_string),
                })
            }
            Some(other) => Err(format!("Unknown kit action `{other}`. Use list, add, or remove.")),
        },
        "status" => Ok(Command::Status {
            path: rest.next().map(str::to_string),
        }),
        "path" => Ok(Command::Path),
        other => Err(format!("Unknown command `{other}`. Try `elin --help`.")),
    }
}

pub fn help_text() -> &'static str {
    "elin  elixir companion\n\n\
     elin                 Open the app (or focus it if already running)\n\
     elin add [path]      Remember this Mix project\n\
     elin list            Print remembered projects\n\
     elin open [path]     Open the project in the app\n\
     elin scan [path]     Analyze modules, git, and enabled Mix tools\n\
     elin scan --full     Also run Dialyzer and tests\n\
     elin format [path]   Run mix format (--check to only report)\n\
     elin kit list        Show kits for this project\n\
     elin kit add <id>    Add a kit to mix.exs\n\
     elin kit remove <id> Remove a kit from mix.exs\n\
     elin status [path]   Pin, git branch, dirty files\n\
     elin path            Put this elin.exe on the user PATH\n"
}

fn print_help() {
    term::banner();
    let rows: &[(&str, &str)] = &[
        ("elin", "open the app — or focus it if it is already running"),
        ("add [path]", "remember this Mix project"),
        ("list", "print remembered projects"),
        ("open [path]", "open the project workspace"),
        ("scan [path]", "modules, git, and enabled Mix tools"),
        ("scan --full", "also Dialyzer and tests"),
        ("format [path]", "mix format  (--check to only report)"),
        ("kit list", "kits for this project"),
        ("kit add <id>", "add a kit to mix.exs"),
        ("kit remove <id>", "remove a kit from mix.exs"),
        ("status [path]", "pin, git branch, dirty files"),
        ("path", "put this elin.exe on the user PATH"),
    ];
    println!("  {}", term::dim("USAGE"));
    for (cmd, hint) in rows {
        println!("    {:<18} {}", term::bold(cmd), term::dim(hint));
    }
    println!();
}

pub fn run(args: &[String]) -> i32 {
    term::enable();
    match parse(args) {
        Ok(cmd) => match execute(cmd) {
            Ok(()) => 0,
            Err(err) => {
                term::err(&err.to_string());
                1
            }
        },
        Err(err) => {
            term::err(&err);
            println!();
            print_help();
            2
        }
    }
}

fn execute(cmd: Command) -> AppResult<()> {
    match cmd {
        Command::Help => {
            print_help();
            Ok(())
        }
        Command::Add { path } => {
            let project = projects::add_project(&resolve_path(path)?)?;
            term::ok(&format!(
                "{}  {}",
                term::bold(&project.name),
                term::dim(&project.path)
            ));
            Ok(())
        }
        Command::List => {
            let list = projects::remembered();
            if list.is_empty() {
                term::info("no remembered Mix projects — run `elin add` inside one");
                return Ok(());
            }
            println!("  {}", term::dim("PROJECTS"));
            for project in list {
                let mark = if project.starred {
                    term::yellow("★")
                } else {
                    term::dim("·")
                };
                println!(
                    "  {mark} {:<16} {}",
                    term::bold(&project.name),
                    term::cyan(&project.path)
                );
            }
            Ok(())
        }
        Command::Open { path } => {
            let project = projects::add_project(&resolve_path(path)?)?;
            projects::write_open_request(&project.path)?;
            spawn_gui()?;
            term::ok(&format!("opening {} in Elin", term::bold(&project.name)));
            Ok(())
        }
        Command::Scan { path, full } => {
            let root = project_root(path)?;
            let report = scan::run_scan(&root, full, true)?;
            println!(
                "  {}  {}  {}",
                term::bold(&report.path),
                term::dim("·"),
                if report.findings.is_empty() {
                    term::green("clean")
                } else {
                    term::yellow(&format!("{} findings", report.findings.len()))
                }
            );
            for layer in &report.layers {
                let mark = if !layer.ran {
                    term::skip_mark()
                } else if layer.ok {
                    term::ok_mark()
                } else {
                    term::fail_mark()
                };
                println!(
                    "    {mark}  {:<12} {}",
                    layer.name,
                    term::dim(&layer.detail)
                );
            }
            for finding in &report.findings {
                let loc = match (&finding.file, finding.line) {
                    (Some(f), Some(n)) => format!("{f}:{n}"),
                    (Some(f), None) => f.clone(),
                    _ => finding.tool.clone(),
                };
                let sev = match finding.severity.as_str() {
                    "error" => term::red(&finding.severity),
                    "warn" | "warning" => term::yellow(&finding.severity),
                    _ => term::dim(&finding.severity),
                };
                println!("    {sev}  {}  {}", term::cyan(&loc), finding.message);
            }
            Ok(())
        }
        Command::Format { path, check } => {
            let root = project_root(path)?;
            let args: Vec<&str> = if check {
                vec!["format", "--check-formatted"]
            } else {
                vec!["format"]
            };
            let out = mix_in_project(Path::new(&root), &args, Duration::from_secs(60))?;
            if out.trim().is_empty() {
                term::ok(if check {
                    "formatted (check passed)"
                } else {
                    "formatted"
                });
            } else {
                println!("{out}");
            }
            Ok(())
        }
        Command::KitList { path } => {
            let root = project_root(path)?;
            let mix = PathBuf::from(&root).join("mix.exs");
            let project = projects::parse_project(&mix)
                .ok_or_else(|| crate::error::AppError::msg("No mix.exs"))?;
            println!("  {}", term::dim("KITS"));
            for status in kits::status_for(&project) {
                let mark = if status.installed {
                    term::ok_mark()
                } else {
                    term::skip_mark()
                };
                println!(
                    "    {mark}  {:<16} {}",
                    status.kit.id,
                    term::dim(&status.kit.summary)
                );
            }
            Ok(())
        }
        Command::KitAdd { id, path } => {
            let root = project_root(path)?;
            let log = kits::apply_kits(&root, &[id], true)?;
            term::ok(&log);
            Ok(())
        }
        Command::KitRemove { id, path } => {
            let root = project_root(path)?;
            term::ok(&kits::remove_kit(&root, &id)?);
            Ok(())
        }
        Command::Status { path } => {
            let root = project_root(path)?;
            let mix = PathBuf::from(&root).join("mix.exs");
            let project = projects::parse_project(&mix)
                .ok_or_else(|| crate::error::AppError::msg("No mix.exs"))?;
            let git = crate::services::git::snapshot(&root);
            println!("  {}", term::cyan(&project.path));
            match (&project.resolved_elixir, &project.resolved_otp) {
                (Some(e), Some(o)) => println!(
                    "  {}  elixir {}  otp {}",
                    term::ok_mark(),
                    term::violet(e),
                    term::yellow(o)
                ),
                _ => println!("  {}  elixir  {}", term::warn_mark(), term::dim("no pin yet")),
            }
            match git.branch {
                Some(b) => println!(
                    "  {}  git {}  {} dirty",
                    term::ok_mark(),
                    term::bold(&b),
                    git.files.len()
                ),
                None => println!("  {}  git  {}", term::skip_mark(), term::dim("not a repository")),
            }
            Ok(())
        }
        Command::Path => {
            term::ok(&env::add_elin_to_path()?);
            Ok(())
        }
    }
}

fn resolve_path(path: Option<String>) -> AppResult<String> {
    match path {
        Some(p) => Ok(p),
        None => Ok(std::env::current_dir()?.to_string_lossy().into()),
    }
}

fn project_root(path: Option<String>) -> AppResult<String> {
    let start = resolve_path(path)?;
    let mix = projects::find_mix_exs(Path::new(&start))?;
    Ok(mix.parent().unwrap_or(Path::new(&start)).to_string_lossy().into())
}

fn spawn_gui() -> AppResult<()> {
    if instance::is_running() {
        let focused = instance::focus();
        if focused {
            term::info("Elin is already running — brought it to the front");
        } else {
            term::info("Elin is already running (in the tray — click the flask)");
        }
        return Ok(());
    }
    let exe = std::env::current_exe()?;
    let mut cmd = std::process::Command::new(exe);
    cmd.stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000 | 0x0000_0008);
    }
    cmd.spawn()?;
    Ok(())
}

/// Attach a console so println works when the exe is WINDOWS subsystem.
/// `AttachConsole` alone does not bind Rust stdout; we also point STD_* at CONOUT$.
/// If stdout is already a pipe (studio console), leave it alone — otherwise help
/// text vanishes into CONOUT$ and the caller captures nothing.
pub fn attach_console() {
    #[cfg(windows)]
    {
        if stdout_already_captured() {
            crate::term::enable();
            return;
        }
        use std::os::windows::io::AsRawHandle;
        unsafe {
            #[link(name = "kernel32")]
            extern "system" {
                fn AttachConsole(pid: u32) -> i32;
                fn AllocConsole() -> i32;
                fn SetStdHandle(n: u32, h: *mut std::ffi::c_void) -> i32;
            }
            const ATTACH_PARENT: u32 = 0xFFFFFFFF;
            const STD_OUTPUT_HANDLE: u32 = 0xFFFFFFF5;
            const STD_ERROR_HANDLE: u32 = 0xFFFFFFF4;
            if AttachConsole(ATTACH_PARENT) == 0 {
                let _ = AllocConsole();
            }
            if let Ok(out) = std::fs::OpenOptions::new().read(true).write(true).open("CONOUT$") {
                let handle = out.as_raw_handle();
                SetStdHandle(STD_OUTPUT_HANDLE, handle);
                SetStdHandle(STD_ERROR_HANDLE, handle);
                std::mem::forget(out);
            }
        }
        crate::term::enable();
    }
}

#[cfg(windows)]
fn stdout_already_captured() -> bool {
    const STD_OUTPUT_HANDLE: u32 = 0xFFFFFFF5;
    const FILE_TYPE_DISK: u32 = 1;
    const FILE_TYPE_PIPE: u32 = 3;
    #[link(name = "kernel32")]
    extern "system" {
        fn GetStdHandle(n: u32) -> *mut std::ffi::c_void;
        fn GetFileType(h: *mut std::ffi::c_void) -> u32;
    }
    unsafe {
        let handle = GetStdHandle(STD_OUTPUT_HANDLE);
        if handle.is_null() || handle == (-1isize as *mut std::ffi::c_void) {
            return false;
        }
        matches!(GetFileType(handle), FILE_TYPE_DISK | FILE_TYPE_PIPE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn empty_args_are_gui() {
        assert!(!is_cli(&[]));
        assert!(is_cli(&args(&["add"])));
        assert!(is_cli(&args(&["--help"])));
    }

    #[test]
    fn parses_scan_full_and_path() {
        let cmd = parse(&args(&["scan", "--full", r"D:\code\app"])).unwrap();
        assert_eq!(
            cmd,
            Command::Scan {
                path: Some(r"D:\code\app".into()),
                full: true
            }
        );
    }

    #[test]
    fn parses_kit_add() {
        let cmd = parse(&args(&["kit", "add", "credo"])).unwrap();
        assert_eq!(
            cmd,
            Command::KitAdd {
                id: "credo".into(),
                path: None
            }
        );
    }

    #[test]
    fn unknown_command_errors() {
        assert!(parse(&args(&["push"])).is_err());
    }

    #[test]
    fn format_check_flag() {
        let cmd = parse(&args(&["format", "--check"])).unwrap();
        assert_eq!(
            cmd,
            Command::Format {
                path: None,
                check: true
            }
        );
    }

    #[test]
    fn parses_path_command() {
        assert_eq!(parse(&args(&["path"])).unwrap(), Command::Path);
    }

    #[test]
    fn help_lists_core_commands() {
        let h = help_text();
        assert!(h.contains("elin open"));
        assert!(h.contains("scan"));
        assert!(h.contains("already running"));
    }
}
