//! Quality kits: Hex deps + config templates Elin writes and later edits.
//!
//! mix.exs surgery lives in [`crate::services::mixexs`]. This module owns the
//! catalog, apply/remove, and config templates.

use crate::error::{AppError, AppResult};
use crate::services::mixexs::{insert_dep, remove_dep};
use crate::services::projects::{parse_project, MixProject};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Kit {
    pub id: String,
    pub name: String,
    pub summary: String,
    pub hex: Option<String>,
    pub requirement: String,
    pub mix_tuple: Option<String>,
    pub default_on: bool,
    pub phoenix_only: bool,
    pub advanced: bool,
    pub config_file: Option<String>,
    pub mix_task: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KitStatus {
    pub kit: Kit,
    pub installed: bool,
    pub config_present: bool,
    #[serde(default)]
    pub credo_strict: Option<bool>,
}

pub fn catalog() -> Vec<Kit> {
    vec![
        kit("format", "Format", "Official mix format. Always on.", None, "", None, true, false, false, Some(".formatter.exs"), Some("format")),
        kit("credo", "Credo", "Style and consistency — the Clippy analog.", Some("credo"), "~> 1.7", Some(r#"{:credo, "~> 1.7", only: [:dev, :test], runtime: false}"#), true, false, false, Some(".credo.exs"), Some("credo")),
        kit("hex_audit", "Hex audit", "Retired packages and Hex advisories. Built into Hex.", None, "", None, true, false, false, None, Some("hex.audit")),
        kit("mix_audit", "MixAudit", "CVE scan of mix.lock (elixir-security-advisories).", Some("mix_audit"), "~> 2.1", Some(r#"{:mix_audit, "~> 2.1", only: [:dev, :test], runtime: false}"#), true, false, false, None, Some("deps.audit")),
        kit("sobelow", "Sobelow", "Phoenix security (XSS, SQLi, leaked secrets).", Some("sobelow"), "~> 0.13", Some(r#"{:sobelow, "~> 0.13", only: [:dev, :test], runtime: false}"#), true, true, false, Some(".sobelow-conf"), Some("sobelow")),
        kit("dialyxir", "Dialyxir", "Success typing. First PLT is slow — opt-in only.", Some("dialyxir"), "~> 1.4", Some(r#"{:dialyxir, "~> 1.4", only: [:dev, :test], runtime: false}"#), false, false, true, None, Some("dialyzer")),
        kit("doctor", "Doctor (docs)", "Moduledoc and typespec coverage gates.", Some("doctor"), "~> 0.21", Some(r#"{:doctor, "~> 0.21", only: [:dev, :test], runtime: false}"#), false, false, true, Some(".doctor.exs"), Some("doctor")),
        kit("excoveralls", "ExCoveralls", "Test coverage threshold. Full scan only.", Some("excoveralls"), "~> 0.18", Some(r#"{:excoveralls, "~> 0.18", only: [:test], runtime: false}"#), false, false, true, None, Some("coveralls")),
        kit("stream_data", "StreamData", "Property-based tests. Adds the dep, not sample tests.", Some("stream_data"), "~> 1.1", Some(r#"{:stream_data, "~> 1.1", only: :test}"#), false, false, false, None, None),
        kit("mox", "Mox", "Behaviour mocks for tests.", Some("mox"), "~> 1.2", Some(r#"{:mox, "~> 1.2", only: :test}"#), false, false, false, None, None),
        kit("boundary", "Boundary", "Compile-time module walls. Advanced; pairs with # elin:boundary.", Some("boundary"), "~> 0.10", Some(r#"{:boundary, "~> 0.10", runtime: false}"#), false, false, true, None, None),
        kit("live_dashboard", "LiveDashboard", "Phoenix metrics. Elin adds the dep; you still wire the router.", Some("phoenix_live_dashboard"), "~> 0.8", Some(r#"{:phoenix_live_dashboard, "~> 0.8"}"#), false, true, false, None, None),
    ]
}

fn kit(
    id: &str,
    name: &str,
    summary: &str,
    hex: Option<&str>,
    requirement: &str,
    mix_tuple: Option<&str>,
    default_on: bool,
    phoenix_only: bool,
    advanced: bool,
    config_file: Option<&str>,
    mix_task: Option<&str>,
) -> Kit {
    Kit {
        id: id.into(),
        name: name.into(),
        summary: summary.into(),
        hex: hex.map(str::to_string),
        requirement: requirement.into(),
        mix_tuple: mix_tuple.map(str::to_string),
        default_on,
        phoenix_only,
        advanced,
        config_file: config_file.map(str::to_string),
        mix_task: mix_task.map(str::to_string),
    }
}

pub fn default_ids(phoenix: bool) -> Vec<String> {
    catalog()
        .into_iter()
        .filter(|k| k.default_on && (!k.phoenix_only || phoenix))
        .map(|k| k.id)
        .collect()
}

pub fn status_for(project: &MixProject) -> Vec<KitStatus> {
    let names: std::collections::BTreeSet<String> = project
        .deps
        .iter()
        .chain(project.locked.iter())
        .map(|d| d.name.clone())
        .collect();
    let root = PathBuf::from(&project.path);
    catalog()
        .into_iter()
        .map(|kit| {
            let installed = match kit.hex.as_deref() {
                None => true,
                Some(hex) => names.contains(hex),
            };
            let config_present = kit
                .config_file
                .as_ref()
                .map(|f| root.join(f).is_file())
                .unwrap_or(true);
            let credo_strict = if kit.id == "credo" {
                Some(read_credo_strict(&root.join(".credo.exs")))
            } else {
                None
            };
            KitStatus {
                kit,
                installed,
                config_present,
                credo_strict,
            }
        })
        .collect()
}

pub fn apply_kits(project_path: &str, ids: &[String], fetch: bool) -> AppResult<String> {
    let mix = PathBuf::from(project_path).join("mix.exs");
    let project = parse_project(&mix).ok_or_else(|| AppError::msg("No mix.exs in that folder"))?;
    let mut log = Vec::new();
    let mut mix_text = fs::read_to_string(&mix)?;
    let mut changed = false;
    let catalog = catalog();
    for id in ids {
        let Some(kit) = catalog.iter().find(|k| k.id == *id) else {
            log.push(format!("unknown kit `{id}` — skipped"));
            continue;
        };
        if kit.phoenix_only && !project.has_phoenix {
            log.push(format!("{} is for Phoenix projects — skipped", kit.name));
            continue;
        }
        if let Some(tuple) = &kit.mix_tuple {
            match insert_dep(&mix_text, tuple) {
                Ok(next) => {
                    if next != mix_text {
                        mix_text = next;
                        changed = true;
                        log.push(format!("added {} to mix.exs", kit.hex.as_deref().unwrap_or(&kit.id)));
                    } else {
                        log.push(format!("{} already in mix.exs", kit.hex.as_deref().unwrap_or(&kit.id)));
                    }
                }
                Err(err) => {
                    log.push(format!("could not patch mix.exs for {}: {err}", kit.name));
                    log.push(format!("paste this yourself:\n  {tuple}"));
                }
            }
        }
        if let Some(file) = &kit.config_file {
            let path = PathBuf::from(project_path).join(file);
            if !path.exists() {
                if let Some(body) = template_for(&kit.id) {
                    fs::write(&path, body)?;
                    log.push(format!("wrote {file}"));
                }
            } else {
                log.push(format!("{file} already exists — left untouched"));
            }
        }
        if kit.id == "format" {
            ensure_formatter(Path::new(project_path), &mut log)?;
        }
        if kit.id == "stream_data" || kit.id == "mox" {
            ensure_test_helper_note(Path::new(project_path), &kit.id, &mut log)?;
        }
    }
    if changed {
        fs::write(&mix, mix_text)?;
    }
    if fetch && changed {
        match crate::services::mixcmd::mix_in_project(
            Path::new(project_path),
            &["deps.get"],
            std::time::Duration::from_secs(180),
        ) {
            Ok(out) => log.push(out),
            Err(err) => log.push(format!("mix deps.get: {err}")),
        }
    }
    Ok(log.join("\n"))
}

pub fn remove_kit(project_path: &str, id: &str) -> AppResult<String> {
    let Some(kit) = catalog().into_iter().find(|k| k.id == id) else {
        return Err(AppError::msg(format!("Unknown kit `{id}`")));
    };
    let Some(hex) = kit.hex else {
        return Err(AppError::msg("That kit is built in and cannot be removed."));
    };
    let mix = PathBuf::from(project_path).join("mix.exs");
    let text = fs::read_to_string(&mix)?;
    match remove_dep(&text, &hex) {
        Ok(next) => {
            if next == text {
                return Ok(format!("`{hex}` was not in mix.exs."));
            }
            fs::write(&mix, next)?;
            Ok(format!("Removed `{hex}` from mix.exs. Config files were left in place."))
        }
        Err(err) => Err(AppError::msg(err)),
    }
}

pub fn write_kit_config(project_path: &str, id: &str) -> AppResult<String> {
    let Some(kit) = catalog().into_iter().find(|k| k.id == id) else {
        return Err(AppError::msg(format!("Unknown kit `{id}`")));
    };
    let Some(file) = kit.config_file else {
        return Err(AppError::msg("That kit has no config file."));
    };
    let path = PathBuf::from(project_path).join(&file);
    if path.exists() {
        return Ok(format!("{file} already exists — left untouched."));
    }
    let Some(body) = template_for(&kit.id) else {
        return Err(AppError::msg(format!("No template for `{id}`.")));
    };
    fs::write(&path, body)?;
    Ok(format!("wrote {file}"))
}

pub fn set_credo_strict(project_path: &str, strict: bool) -> AppResult<String> {
    let path = PathBuf::from(project_path).join(".credo.exs");
    if !path.exists() {
        fs::write(&path, template_for("credo").unwrap_or_default())?;
    }
    let text = fs::read_to_string(&path)?;
    let next = if strict {
        text.replace("strict: false", "strict: true")
    } else {
        text.replace("strict: true", "strict: false")
    };
    if next == text && !text.contains("strict:") {
        return Err(AppError::msg(
            ".credo.exs has no `strict:` key. Add `strict: false` and try again.",
        ));
    }
    if next != text {
        fs::write(&path, next)?;
    }
    Ok(if strict {
        "Credo strict mode on.".into()
    } else {
        "Credo strict mode off.".into()
    })
}

fn read_credo_strict(path: &Path) -> bool {
    fs::read_to_string(path)
        .map(|t| t.contains("strict: true"))
        .unwrap_or(false)
}

fn ensure_formatter(root: &Path, log: &mut Vec<String>) -> AppResult<()> {
    let path = root.join(".formatter.exs");
    if path.exists() {
        log.push(".formatter.exs already exists — left untouched".into());
        return Ok(());
    }
    fs::write(path, template_for("format").unwrap_or_default())?;
    log.push("wrote .formatter.exs".into());
    Ok(())
}

fn ensure_test_helper_note(root: &Path, kit: &str, log: &mut Vec<String>) -> AppResult<()> {
    let path = root.join("test").join("test_helper.exs");
    if !path.exists() {
        return Ok(());
    }
    let mut text = fs::read_to_string(&path)?;
    let marker = format!("# elin:{kit}");
    if text.contains(&marker) {
        return Ok(());
    }
    let note = match kit {
        "stream_data" => "\n# elin:stream_data — use ExUnitProperties in tests when you want property checks.\n",
        "mox" => "\n# elin:mox — define a behaviour, then Mox.defmock in this helper.\n",
        _ => return Ok(()),
    };
    text.push_str(note);
    fs::write(path, text)?;
    log.push(format!("annotated test/test_helper.exs for {kit}"));
    Ok(())
}

fn template_for(id: &str) -> Option<&'static str> {
    match id {
        "format" => Some(
            "[\n  inputs: [\"{mix,.formatter}.exs\", \"{config,lib,test}/**/*.{ex,exs}\"]\n]\n",
        ),
        "credo" => Some(
            r#"%{
  configs: [
    %{
      name: "default",
      files: %{
        included: ["lib/", "src/", "test/", "web/", "apps/"],
        excluded: [~r"/_build/", ~r"/deps/"]
      },
      plugins: [],
      requires: [],
      strict: false,
      parse_timeout: 5000,
      color: true,
      checks: %{
        enabled: [
          {Credo.Check.Readability.ModuleDoc, []},
          {Credo.Check.Readability.MaxLineLength, [max_length: 120]},
          {Credo.Check.Design.AliasUsage, [if_nested_deeper_than: 2]},
          {Credo.Check.Warning.UnusedEnumOperation, []},
          {Credo.Check.Warning.UnusedKeywordOperation, []},
          {Credo.Check.Warning.UnusedListOperation, []},
          {Credo.Check.Warning.UnusedStringOperation, []},
          {Credo.Check.Warning.UnusedTupleOperation, []}
        ],
        disabled: []
      }
    }
  ]
}
"#,
        ),
        "sobelow" => Some(
            "[verbose: false, private: false, skip: false, router: \"\", exit: \"low\", format: \"pretty\", ignore: [], ignore_files: []]\n",
        ),
        "doctor" => Some(
            r#"%Doctor.Config{
  ignore_modules: [],
  ignore_paths: [],
  min_module_doc_coverage: 40,
  min_module_spec_coverage: 0,
  min_overall_doc_coverage: 50,
  min_overall_spec_coverage: 0,
  moduledoc_required: true,
  raise: false,
  reporter: Doctor.Reporters.Full,
  struct_type_spec_required: true,
  umbrella: false
}
"#,
        ),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_ids_include_sobelow_only_for_phoenix() {
        let mix = default_ids(false);
        let phx = default_ids(true);
        assert!(mix.contains(&"credo".into()));
        assert!(mix.contains(&"mix_audit".into()));
        assert!(!mix.contains(&"sobelow".into()));
        assert!(phx.contains(&"sobelow".into()));
        assert!(!mix.contains(&"dialyxir".into()));
    }

    #[test]
    fn credo_template_is_not_strict() {
        let body = template_for("credo").unwrap();
        assert!(body.contains("strict: false"));
    }
}
