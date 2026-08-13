//! Git status, diffs, and commit — via the probed `git.exe`. No push.

use crate::error::{AppError, AppResult};
use crate::services::analyze::ModuleGraph;
use crate::services::probe::probe_machine;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitFile {
    pub path: String,
    pub status: String,
    #[serde(default)]
    pub added: u32,
    #[serde(default)]
    pub deleted: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DepChange {
    pub name: String,
    pub kind: String,
    pub from: Option<String>,
    pub to: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitSnapshot {
    pub repo: Option<String>,
    pub branch: Option<String>,
    pub identity_ok: bool,
    pub identity_hint: Option<String>,
    pub files: Vec<GitFile>,
    pub dep_changes: Vec<DepChange>,
}

const BLOCKED_COMMIT: &[&str] = &[
    "_build/",
    "deps/",
    ".elixir_ls/",
    "node_modules/",
    ".env",
    "id_rsa",
    ".pem",
    "secrets",
];

pub fn snapshot(project_path: &str) -> GitSnapshot {
    let root = PathBuf::from(project_path);
    let Some(git) = git_exe() else {
        return GitSnapshot {
            repo: None,
            branch: None,
            identity_ok: false,
            identity_hint: Some("Git was not found. Doctor can add it to PATH.".into()),
            files: vec![],
            dep_changes: vec![],
        };
    };
    let Ok(repo) = git_out(&git, &root, &["rev-parse", "--show-toplevel"]) else {
        return GitSnapshot {
            repo: None,
            branch: None,
            identity_ok: false,
            identity_hint: None,
            files: vec![],
            dep_changes: vec![],
        };
    };
    let repo = repo.trim().replace('/', "\\");
    let repo_path = PathBuf::from(&repo);
    let branch = git_out(&git, &repo_path, &["rev-parse", "--abbrev-ref", "HEAD"])
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let name = git_out(&git, &repo_path, &["config", "user.name"])
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let email = git_out(&git, &repo_path, &["config", "user.email"])
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let identity_ok = name.is_some() && email.is_some();
    let identity_hint = if identity_ok {
        None
    } else {
        Some("Set git user.name and user.email before committing.".into())
    };

    let porcelain = git_out(&git, &repo_path, &["status", "--porcelain=v1"]).unwrap_or_default();
    let mut files = parse_porcelain(&porcelain);
    if let Ok(numstat) = git_out(&git, &repo_path, &["diff", "HEAD", "--numstat"]) {
        apply_numstat(&mut files, &numstat);
    }
    let dep_changes = lock_diff(&git, &repo_path);
    GitSnapshot {
        repo: Some(repo),
        branch,
        identity_ok,
        identity_hint,
        files,
        dep_changes,
    }
}

pub fn overlay(graph: &mut ModuleGraph, git: &GitSnapshot) {
    if git.repo.is_none() {
        return;
    }
    let by_path: BTreeMap<String, &GitFile> = git
        .files
        .iter()
        .map(|f| (normalize(&f.path), f))
        .collect();
    let mut known_paths: BTreeSet<String> = BTreeSet::new();
    for node in &mut graph.nodes {
        if let Some(path) = &node.path {
            known_paths.insert(normalize(path));
            if let Some(file) = by_path.get(&normalize(path)) {
                node.git = Some(status_label(&file.status));
            } else {
                node.git = Some("unchanged".into());
            }
        }
    }

    // Deleted files: try to recover the module name from the relative path.
    for file in &git.files {
        if !file.status.contains('D') {
            continue;
        }
        let key = normalize(&file.path);
        if known_paths.contains(&key) {
            continue;
        }
        let stem = Path::new(&file.path)
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| file.path.clone());
        let id = format!("(deleted) {stem}");
        graph.nodes.push(crate::services::analyze::GraphNode {
            id: id.clone(),
            label: stem,
            path: Some(file.path.clone()),
            kind: "deleted".into(),
            git: Some("deleted".into()),
            role: "deleted".into(),
            ..Default::default()
        });
    }

    let changed: BTreeSet<String> = graph
        .nodes
        .iter()
        .filter(|n| matches!(n.git.as_deref(), Some("added" | "modified" | "untracked")))
        .map(|n| n.id.clone())
        .collect();
    for edge in &mut graph.edges {
        edge.is_new = changed.contains(&edge.from);
    }
}

pub fn commit(project_path: &str, message: &str, paths: &[String]) -> AppResult<String> {
    let message = message.trim();
    if message.is_empty() {
        return Err(AppError::msg("Write a commit message."));
    }
    let git = git_exe().ok_or_else(|| AppError::msg("Git was not found."))?;
    let snap = snapshot(project_path);
    let repo = snap
        .repo
        .as_ref()
        .ok_or_else(|| AppError::msg("This folder is not a git repository."))?;
    if !snap.identity_ok {
        return Err(AppError::msg(
            snap.identity_hint
                .unwrap_or_else(|| "Git user.name / user.email are not set.".into()),
        ));
    }
    let repo_path = PathBuf::from(repo);
    let mut allowed = Vec::new();
    for path in paths {
        let norm = normalize(path);
        if is_blocked(&norm) {
            return Err(AppError::msg(format!(
                "Refusing to commit `{path}` (build artifacts, deps, or secrets)."
            )));
        }
        allowed.push(norm);
    }
    if allowed.is_empty() {
        return Err(AppError::msg("Pick at least one file to commit."));
    }
    let mut add_args = vec!["add".to_string(), "--".to_string()];
    add_args.extend(allowed);
    let add_refs: Vec<&str> = add_args.iter().map(String::as_str).collect();
    let add_out = git_run(&git, &repo_path, &add_refs)?;
    if !add_out.status.success() {
        return Err(AppError::msg(crate::services::winproc::output_text(&add_out)));
    }
    let commit_out = git_run(&git, &repo_path, &["commit", "-m", message])?;
    let text = crate::services::winproc::output_text(&commit_out);
    if !commit_out.status.success() {
        return Err(AppError::msg(text));
    }
    Ok(text)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LicenseOpt {
    pub id: String,
    pub name: String,
}

pub fn license_options() -> Vec<LicenseOpt> {
    [
        ("none", "None"),
        ("MIT", "MIT"),
        ("Apache-2.0", "Apache-2.0"),
        ("BSD-3-Clause", "BSD-3-Clause"),
        ("GPL-3.0", "GPL-3.0"),
        ("MPL-2.0", "MPL-2.0"),
        ("Unlicense", "Unlicense"),
    ]
    .into_iter()
    .map(|(id, name)| LicenseOpt {
        id: id.into(),
        name: name.into(),
    })
    .collect()
}

/// `git init` plus a Mix-aware `.gitignore` and an optional LICENSE.
pub fn init_repo(project_path: &str, license: &str) -> AppResult<GitSnapshot> {
    let root = PathBuf::from(project_path);
    if !root.join("mix.exs").is_file() {
        return Err(AppError::msg("No mix.exs in that folder."));
    }
    let git = git_exe().ok_or_else(|| AppError::msg("Git was not found. Doctor can add it to PATH."))?;
    if snapshot(project_path).repo.is_some() {
        return Err(AppError::msg("This folder is already a git repository."));
    }
    let init = git_run(&git, &root, &["init"])?;
    if !init.status.success() {
        return Err(AppError::msg(crate::services::winproc::output_text(&init)));
    }
    let gi = root.join(".gitignore");
    if !gi.exists() {
        fs::write(&gi, ELIXIR_GITIGNORE)?;
    } else {
        let existing = fs::read_to_string(&gi).unwrap_or_default();
        if !existing.contains("_build") {
            let mut next = existing;
            if !next.ends_with('\n') && !next.is_empty() {
                next.push('\n');
            }
            next.push_str("\n# Elixir (added by Elin)\n_build/\ndeps/\n");
            fs::write(&gi, next)?;
        }
    }
    if license != "none" && !license.is_empty() {
        let license_path = root.join("LICENSE");
        if !license_path.exists() {
            if let Some(body) = license_body(license) {
                fs::write(license_path, body)?;
            }
        }
    }
    Ok(snapshot(project_path))
}

const ELIXIR_GITIGNORE: &str = r#"# Mix / Elixir
/_build/
/cover/
/deps/
/doc/
/.fetch
erl_crash.dump
*.ez
*.beam
/tmp/
/config/*.secret.exs
.elixir_ls/
.elixir-tools/
.lexical/

# Assets
/assets/node_modules/
/priv/static/assets/
/priv/static/cache_manifest.json

# OS / editors
.DS_Store
Thumbs.db
.idea/
.vscode/
*.swp
"#;

fn license_body(id: &str) -> Option<String> {
    let year = 2026;
    Some(match id {
        "MIT" => format!(
            "MIT License\n\nCopyright (c) {year}\n\nPermission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the \"Software\"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:\n\nThe above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.\n\nTHE SOFTWARE IS PROVIDED \"AS IS\", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.\n"
        ),
        "BSD-3-Clause" => format!(
            "Copyright (c) {year}\nAll rights reserved.\n\nRedistribution and use in source and binary forms, with or without modification, are permitted provided that the following conditions are met:\n\n1. Redistributions of source code must retain the above copyright notice, this list of conditions and the following disclaimer.\n2. Redistributions in binary form must reproduce the above copyright notice, this list of conditions and the following disclaimer in the documentation and/or other materials provided with the distribution.\n3. Neither the name of the copyright holder nor the names of its contributors may be used to endorse or promote products derived from this software without specific prior written permission.\n\nTHIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS \"AS IS\" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.\n"
        ),
        "Unlicense" => "This is free and unencumbered software released into the public domain.\n\nAnyone is free to copy, modify, publish, use, compile, sell, or distribute this software, either in source code form or as a compiled binary, for any purpose, commercial or non-commercial, and by any means.\n\nIn jurisdictions that recognize copyright laws, the author or authors of this software dedicate any and all copyright interest in the software to the public domain. We make this dedication for the benefit of the public at large and to the detriment of our heirs and successors. We intend this dedication to be an overt act of relinquishment in perpetuity of all present and future rights to this software under copyright law.\n\nTHE SOFTWARE IS PROVIDED \"AS IS\", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.\n\nFor more information, please refer to <https://unlicense.org>\n".into(),
        "Apache-2.0" => format!(
            "Copyright {year}\n\nLicensed under the Apache License, Version 2.0 (the \"License\");\nyou may not use this file except in compliance with the License.\nYou may obtain a copy of the License at\n\n    http://www.apache.org/licenses/LICENSE-2.0\n\nUnless required by applicable law or agreed to in writing, software\ndistributed under the License is distributed on an \"AS IS\" BASIS,\nWITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.\nSee the License for the specific language governing permissions and\nlimitations under the License.\n"
        ),
        "GPL-3.0" => format!(
            "Copyright (C) {year}\n\nThis program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.\n\nThis program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.\n\nYou should have received a copy of the GNU General Public License along with this program. If not, see <https://www.gnu.org/licenses/>.\n"
        ),
        "MPL-2.0" => "This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.\n".into(),
        _ => return None,
    })
}

pub fn parse_porcelain(text: &str) -> Vec<GitFile> {
    let mut files = Vec::new();
    for line in text.lines() {
        let line = line.trim_end();
        if line.chars().count() < 4 {
            continue;
        }
        let mut chars = line.chars();
        let a = chars.next().unwrap_or(' ');
        let b = chars.next().unwrap_or(' ');
        let status: String = format!("{a}{b}").trim().to_string();
        let rest: String = chars.collect::<String>().trim().trim_matches('"').to_string();
        let path = if let Some((from, to)) = rest.split_once(" -> ") {
            let _ = from;
            to.trim().trim_matches('"').replace('\\', "/")
        } else {
            rest.replace('\\', "/")
        };
        if path.is_empty() {
            continue;
        }
        files.push(GitFile {
            path,
            status: if status.is_empty() { "M".into() } else { status },
            added: 0,
            deleted: 0,
        });
    }
    files
}

pub fn parse_numstat(text: &str) -> Vec<(u32, u32, String)> {
    let mut rows = Vec::new();
    for line in text.lines() {
        let mut parts = line.split('\t');
        let added = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        let deleted = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        let Some(path) = parts.next() else {
            continue;
        };
        rows.push((added, deleted, path.replace('\\', "/")));
    }
    rows
}

fn apply_numstat(files: &mut [GitFile], numstat: &str) {
    let stats = parse_numstat(numstat);
    for (added, deleted, path) in stats {
        if let Some(file) = files.iter_mut().find(|f| normalize(&f.path) == normalize(&path)) {
            file.added = added;
            file.deleted = deleted;
        }
    }
}

pub fn parse_lock_names(text: &str) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    let Ok(re) = regex::Regex::new(r#""([a-zA-Z0-9_]+)":\s*\{:hex,\s*:[a-zA-Z0-9_]+,\s*"([^"]+)""#) else {
        return map;
    };
    for cap in re.captures_iter(text) {
        map.insert(cap[1].to_string(), cap[2].to_string());
    }
    map
}

pub fn diff_locks(old: &str, new: &str) -> Vec<DepChange> {
    let before = parse_lock_names(old);
    let after = parse_lock_names(new);
    let mut changes = Vec::new();
    for (name, ver) in &after {
        match before.get(name) {
            None => changes.push(DepChange {
                name: name.clone(),
                kind: "added".into(),
                from: None,
                to: Some(ver.clone()),
            }),
            Some(old_ver) if old_ver != ver => changes.push(DepChange {
                name: name.clone(),
                kind: "changed".into(),
                from: Some(old_ver.clone()),
                to: Some(ver.clone()),
            }),
            _ => {}
        }
    }
    for (name, ver) in &before {
        if !after.contains_key(name) {
            changes.push(DepChange {
                name: name.clone(),
                kind: "removed".into(),
                from: Some(ver.clone()),
                to: None,
            });
        }
    }
    changes
}

fn lock_diff(git: &Path, repo: &Path) -> Vec<DepChange> {
    let Ok(old) = git_out(git, repo, &["show", "HEAD:mix.lock"]) else {
        return Vec::new();
    };
    let new = std::fs::read_to_string(repo.join("mix.lock")).unwrap_or_default();
    if new.is_empty() {
        return Vec::new();
    }
    diff_locks(&old, &new)
}

fn status_label(code: &str) -> String {
    if code.contains('?') {
        "untracked".into()
    } else if code.contains('A') {
        "added".into()
    } else if code.contains('D') {
        "deleted".into()
    } else if code.contains('R') {
        "renamed".into()
    } else {
        "modified".into()
    }
}

fn is_blocked(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    BLOCKED_COMMIT.iter().any(|b| lower.contains(b))
}

fn normalize(path: &str) -> String {
    path.replace('\\', "/").trim_start_matches("./").to_string()
}

fn git_exe() -> Option<PathBuf> {
    probe_machine()
        .ok()
        .and_then(|p| p.git.map(|h| PathBuf::from(h.path)))
        .or_else(|| which::which("git").ok())
}

fn git_out(git: &Path, cwd: &Path, args: &[&str]) -> AppResult<String> {
    let output = git_run(git, cwd, args)?;
    if !output.status.success() {
        return Err(AppError::msg(crate::services::winproc::output_text(&output)));
    }
    // Keep leading spaces — porcelain uses them (` M path`). trim() ate the
    // first file's status column and turned `lib/foo.ex` into `ib/foo.ex`.
    Ok(crate::services::winproc::decode_bytes(&output.stdout)
        .trim_end()
        .to_string())
}

fn git_run(git: &Path, cwd: &Path, args: &[&str]) -> AppResult<std::process::Output> {
    let mut cmd = Command::new(git);
    cmd.args(args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    crate::services::winproc::hide_console(&mut cmd);
    Ok(cmd.output()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn porcelain_parses_modified_untracked_and_rename() {
        let text = " M lib/foo.ex\n?? lib/new.ex\nR  lib/old.ex -> lib/renamed.ex\n D lib/gone.ex\n";
        let files = parse_porcelain(text);
        assert_eq!(files.len(), 4);
        assert_eq!(files[0].path, "lib/foo.ex");
        assert_eq!(files[0].status, "M");
        assert_eq!(files[1].status, "??");
        assert_eq!(files[2].path, "lib/renamed.ex");
        assert_eq!(files[3].status, "D");
    }

    #[test]
    fn porcelain_survives_trimmed_leading_space() {
        // decode_console used to trim the whole blob, turning ` M lib/a.ex` into `M lib/a.ex`.
        let files = parse_porcelain("M lib/hello_elin.ex\n?? mix.exs\n");
        assert_eq!(files[0].path, "lib/hello_elin.ex");
        assert_eq!(files[1].path, "mix.exs");
    }

    #[test]
    fn numstat_parses_counts() {
        let rows = parse_numstat("12\t3\tlib/foo.ex\n0\t8\tmix.lock\n");
        assert_eq!(rows[0], (12, 3, "lib/foo.ex".into()));
        assert_eq!(rows[1], (0, 8, "mix.lock".into()));
    }

    #[test]
    fn lock_diff_detects_added_changed_removed() {
        let old = r#""jason": {:hex, :jason, "1.4.0"}
"phoenix": {:hex, :phoenix, "1.7.0"}
"#;
        let new = r#""jason": {:hex, :jason, "1.4.4"}
"plug": {:hex, :plug, "1.16.0"}
"#;
        let changes = diff_locks(old, new);
        assert!(changes.iter().any(|c| c.name == "jason" && c.kind == "changed"));
        assert!(changes.iter().any(|c| c.name == "plug" && c.kind == "added"));
        assert!(changes.iter().any(|c| c.name == "phoenix" && c.kind == "removed"));
    }

    #[test]
    fn blocked_paths_include_build_and_secrets() {
        assert!(is_blocked("deps/phoenix/mix.exs"));
        assert!(is_blocked("_build/dev/lib/app"));
        assert!(is_blocked(".env"));
        assert!(!is_blocked("lib/foo.ex"));
    }

    #[test]
    fn overlay_marks_new_edges_from_modified_files() {
        let mut graph = ModuleGraph {
            nodes: vec![
                crate::services::analyze::GraphNode {
                    id: "A".into(),
                    label: "A".into(),
                    path: Some("lib/a.ex".into()),
                    kind: "lib".into(),
                    wired: true,
                    ..Default::default()
                },
                crate::services::analyze::GraphNode {
                    id: "B".into(),
                    label: "B".into(),
                    path: Some("lib/b.ex".into()),
                    kind: "lib".into(),
                    wired: true,
                    ..Default::default()
                },
            ],
            edges: vec![crate::services::analyze::GraphEdge {
                from: "A".into(),
                to: "B".into(),
                kind: "alias".into(),
                is_new: false,
            }],
            ..Default::default()
        };
        let git = GitSnapshot {
            repo: Some("x".into()),
            branch: Some("main".into()),
            identity_ok: true,
            identity_hint: None,
            files: vec![GitFile {
                path: "lib/a.ex".into(),
                status: "M".into(),
                added: 2,
                deleted: 0,
            }],
            dep_changes: vec![],
        };
        overlay(&mut graph, &git);
        assert_eq!(graph.nodes[0].git.as_deref(), Some("modified"));
        assert_eq!(graph.nodes[1].git.as_deref(), Some("unchanged"));
        assert!(graph.edges[0].is_new);
    }

    #[test]
    fn license_picker_has_short_ids() {
        let ids: Vec<String> = license_options().into_iter().map(|l| l.id).collect();
        assert!(ids.iter().any(|id| id == "MIT"));
        assert!(ids.iter().any(|id| id == "none"));
        assert!(license_body("MIT").unwrap().contains("Permission is hereby granted"));
    }
}
