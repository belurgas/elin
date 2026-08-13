//! `elin scan` — Elin analyzer plus Mix tools. One report for CLI and UI.

mod parse;

use crate::error::{AppError, AppResult};
use crate::services::analyze::{analyze_path, ModuleGraph};
use crate::services::git::{self, GitSnapshot};
use crate::services::kits::{self, KitStatus};
use crate::services::mixcmd::mix_in_project;
use crate::services::projects::parse_project;
use parse::{
    parse_credo_json, parse_format_output, parse_mix_audit_json, parse_sobelow_json,
    parse_text_findings, truncate,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanFinding {
    pub layer: String,
    pub severity: String,
    pub file: Option<String>,
    pub line: Option<u32>,
    pub message: String,
    pub tool: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanLayer {
    pub id: String,
    pub name: String,
    pub ran: bool,
    pub ok: bool,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanReport {
    pub path: String,
    pub full: bool,
    pub layers: Vec<ScanLayer>,
    pub findings: Vec<ScanFinding>,
    pub graph: ModuleGraph,
    pub git: GitSnapshot,
    pub kits: Vec<KitStatus>,
}

/// Layer 0 (graph + git) always runs. Mix layers run when `mix_layers` is set.
/// `--full` adds Dialyzer and ExUnit when those kits are present.
pub fn run_scan(project_path: &str, full: bool, mix_layers: bool) -> AppResult<ScanReport> {
    let root = PathBuf::from(project_path);
    let mix = root.join("mix.exs");
    if !mix.is_file() {
        return Err(AppError::msg("No mix.exs in that folder."));
    }
    let project = parse_project(&mix).ok_or_else(|| AppError::msg("Could not read mix.exs"))?;
    let kits = kits::status_for(&project);
    let mut graph = analyze_path(&root);
    let git = git::snapshot(project_path);
    git::overlay(&mut graph, &git);

    let mut findings = Vec::new();
    let mut layers = Vec::new();

    let unwired: Vec<_> = graph
        .nodes
        .iter()
        .filter(|n| !n.wired && n.kind != "test" && n.git.as_deref() != Some("deleted"))
        .cloned()
        .collect();
    for node in &unwired {
        findings.push(ScanFinding {
            layer: "graph".into(),
            severity: "info".into(),
            file: node.path.clone(),
            line: None,
            message: format!("{} is not wired to other project modules", node.id),
            tool: "elin".into(),
        });
    }
    layers.push(ScanLayer {
        id: "graph".into(),
        name: "Module graph".into(),
        ran: true,
        ok: true,
        detail: format!(
            "{} modules, {} edges, {} git-dirty",
            graph.nodes.len(),
            graph.edges.len(),
            git.files.len()
        ),
    });

    if !mix_layers {
        return Ok(ScanReport {
            path: project.path,
            full,
            layers,
            findings,
            graph,
            git,
            kits,
        });
    }

    run_mix_layer(
        &root,
        "format",
        "Format",
        "format",
        &["format", "--check-formatted"],
        Duration::from_secs(60),
        &mut layers,
        &mut findings,
        parse_format_output,
    );

    if kit_on(&kits, "credo") {
        run_mix_layer(
            &root,
            "credo",
            "Credo",
            "credo",
            &["credo", "--format", "json", "--all"],
            Duration::from_secs(120),
            &mut layers,
            &mut findings,
            parse_credo_json,
        );
    } else {
        skipped(&mut layers, "credo", "Credo", "Credo is not in mix.exs");
    }

    run_mix_layer(
        &root,
        "hex_audit",
        "Hex audit",
        "hex.audit",
        &["hex.audit"],
        Duration::from_secs(60),
        &mut layers,
        &mut findings,
        |out| parse_text_findings("hex_audit", "hex.audit", out),
    );

    if kit_on(&kits, "mix_audit") {
        run_mix_layer(
            &root,
            "mix_audit",
            "MixAudit",
            "deps.audit",
            &["deps.audit", "--format", "json"],
            Duration::from_secs(120),
            &mut layers,
            &mut findings,
            parse_mix_audit_json,
        );
    } else {
        skipped(
            &mut layers,
            "mix_audit",
            "MixAudit",
            "mix_audit is not in mix.exs",
        );
    }

    if project.has_phoenix && kit_on(&kits, "sobelow") {
        run_mix_layer(
            &root,
            "sobelow",
            "Sobelow",
            "sobelow",
            &["sobelow", "--format", "json", "--quiet"],
            Duration::from_secs(120),
            &mut layers,
            &mut findings,
            parse_sobelow_json,
        );
    } else if project.has_phoenix {
        skipped(
            &mut layers,
            "sobelow",
            "Sobelow",
            "Sobelow is not in mix.exs",
        );
    }

    if full {
        if kit_on(&kits, "dialyxir") {
            run_mix_layer(
                &root,
                "dialyzer",
                "Dialyzer",
                "dialyzer",
                &["dialyzer", "--format", "short"],
                Duration::from_secs(600),
                &mut layers,
                &mut findings,
                |out| parse_text_findings("dialyzer", "dialyzer", out),
            );
        } else {
            skipped(
                &mut layers,
                "dialyzer",
                "Dialyzer",
                "dialyxir is not in mix.exs",
            );
        }
        run_mix_layer(
            &root,
            "test",
            "ExUnit",
            "test",
            &["test"],
            Duration::from_secs(300),
            &mut layers,
            &mut findings,
            |out| parse_text_findings("test", "test", out),
        );
    }

    Ok(ScanReport {
        path: project.path,
        full,
        layers,
        findings,
        graph,
        git,
        kits,
    })
}

fn kit_on(kits: &[KitStatus], id: &str) -> bool {
    kits.iter().any(|k| k.kit.id == id && k.installed)
}

fn skipped(layers: &mut Vec<ScanLayer>, id: &str, name: &str, detail: &str) {
    layers.push(ScanLayer {
        id: id.into(),
        name: name.into(),
        ran: false,
        ok: true,
        detail: detail.into(),
    });
}

fn run_mix_layer(
    root: &Path,
    id: &str,
    name: &str,
    tool: &str,
    args: &[&str],
    timeout: Duration,
    layers: &mut Vec<ScanLayer>,
    findings: &mut Vec<ScanFinding>,
    parse: fn(&str) -> Vec<ScanFinding>,
) {
    match mix_in_project(root, args, timeout) {
        Ok(out) => {
            let parsed = parse(&out);
            let failed = out.contains("** (")
                || args.contains(&"--check-formatted") && out.contains("would be formatted");
            let ok = parsed.iter().all(|f| f.severity != "error") && !failed;
            findings.extend(parsed);
            layers.push(ScanLayer {
                id: id.into(),
                name: name.into(),
                ran: true,
                ok,
                detail: if ok {
                    format!("{tool} finished")
                } else {
                    format!("{tool} reported issues")
                },
            });
        }
        Err(err) => {
            let msg = err.to_string();
            if msg.contains("Install Elixir") || msg.contains("No mix.exs") {
                skipped(layers, id, name, "No project toolchain — skipped Mix layer");
                return;
            }
            findings.extend(parse(&msg));
            layers.push(ScanLayer {
                id: id.into(),
                name: name.into(),
                ran: true,
                ok: false,
                detail: truncate(&msg, 240),
            });
        }
    }
}
