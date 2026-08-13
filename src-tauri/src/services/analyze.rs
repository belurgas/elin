//! Static Elixir analysis: modules as files, edges, and `# elin:` comments.
//!
//! Does not compile Mix or start BEAM. Walks `lib/` and `test/` only.

use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub git: Option<String>,
    #[serde(default)]
    pub boundary: Option<String>,
    #[serde(default)]
    pub notes: Vec<String>,
    #[serde(default)]
    pub ignored: bool,
    #[serde(default)]
    pub wired: bool,
    /// Inferred role: module, genserver, supervisor, liveview, controller, schema, router, test.
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub loc: u32,
    #[serde(default)]
    pub defs: u32,
    #[serde(default)]
    pub defps: u32,
    #[serde(default)]
    pub behaviours: Vec<String>,
    #[serde(default)]
    pub fan_in: u32,
    #[serde(default)]
    pub fan_out: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphEdge {
    pub from: String,
    pub to: String,
    pub kind: String,
    #[serde(default)]
    pub is_new: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ElinComment {
    pub file: String,
    pub line: u32,
    pub tag: String,
    pub value: String,
    pub module: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectEntry {
    pub rel: String,
    pub is_dir: bool,
    pub module: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModuleGraph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    #[serde(default)]
    pub comments: Vec<ElinComment>,
    #[serde(default)]
    pub files: Vec<ProjectEntry>,
    #[serde(default)]
    pub stats: GraphStats,
    #[serde(default)]
    pub cycles: Vec<Vec<String>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphStats {
    pub modules: u32,
    pub tests: u32,
    pub edges: u32,
    pub unwired: u32,
    pub cycles: u32,
}

#[derive(Debug, Clone, Default)]
struct FileTags {
    ignore: bool,
    module_override: Option<String>,
    file_bind: Option<String>,
    boundary: Option<String>,
    deps: Vec<String>,
    notes: Vec<String>,
}

struct Patterns {
    def: Regex,
    alias_as: Regex,
    alias_curly: Regex,
    alias: Regex,
    import: Regex,
    use_mod: Regex,
    delegate: Regex,
    call: Regex,
    require: Regex,
    behaviour: Regex,
    def_fun: Regex,
    defp_fun: Regex,
}

static PAT: Lazy<Patterns> = Lazy::new(|| Patterns {
    def: Regex::new(r"(?m)^\s*defmodule\s+([A-Za-z0-9_.]+)\s+do").expect("defmodule"),
    alias_as: Regex::new(r"(?m)^\s*alias\s+([A-Za-z0-9_.]+)\s*,\s*as:\s*([A-Za-z0-9_]+)")
        .expect("alias as"),
    alias_curly: Regex::new(r"(?m)^\s*alias\s+([A-Za-z0-9_.]+)\.\{([^}]+)\}").expect("alias curly"),
    alias: Regex::new(r"(?m)^\s*alias\s+([A-Za-z0-9_.]+)").expect("alias"),
    import: Regex::new(r"(?m)^\s*import\s+([A-Za-z0-9_.]+)").expect("import"),
    use_mod: Regex::new(r"(?m)^\s*use\s+([A-Za-z0-9_.]+)").expect("use"),
    delegate: Regex::new(r"(?m)defdelegate\s+[^,\n]+,\s*to:\s*([A-Za-z0-9_.]+)").expect("delegate"),
    call: Regex::new(
        r"\b([A-Z][A-Za-z0-9_]*(?:\.[A-Z][A-Za-z0-9_]*)*)\.[a-z_][A-Za-z0-9_!?]*(?:\s*\(|/)",
    )
    .expect("call"),
    require: Regex::new(r"(?m)^\s*require\s+([A-Za-z0-9_.]+)").expect("require"),
    behaviour: Regex::new(r"(?m)^\s*@behaviour\s+([A-Za-z0-9_.]+)").expect("behaviour"),
    def_fun: Regex::new(r"(?m)^\s*def\s+[a-z_]").expect("def"),
    defp_fun: Regex::new(r"(?m)^\s*defp\s+[a-z_]").expect("defp"),
});

/// Analyze a Mix project root. Missing `lib/` yields an empty graph, not an error.
pub fn analyze_project(project_path: &str) -> AppResult<ModuleGraph> {
    Ok(analyze_path(Path::new(project_path)))
}

pub fn analyze_path(root: &Path) -> ModuleGraph {
    let mut comments = Vec::new();
    let mut files = collect_tree(root);
    let mut nodes_by_id: BTreeMap<String, GraphNode> = BTreeMap::new();
    let mut refs: Vec<(String, String, String)> = Vec::new();
    let mut local_aliases: HashMap<String, HashMap<String, String>> = HashMap::new();

    let mut source_files = Vec::new();
    for dir_name in ["lib", "test"] {
        let dir = root.join(dir_name);
        if !dir.exists() {
            continue;
        }
        for entry in WalkDir::new(&dir).into_iter().flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if ext != "ex" && ext != "exs" {
                continue;
            }
            if skip_walk(path) {
                continue;
            }
            source_files.push(path.to_path_buf());
        }
    }

    for path in &source_files {
        let Ok(text) = fs::read_to_string(path) else {
            continue;
        };
        let rel = rel_to(root, path);
        let kind = if rel.replace('\\', "/").starts_with("test/") {
            "test"
        } else {
            "lib"
        };
        let tags = parse_file_tags(&text);
        let file_comments = parse_elin_comments(&text, &rel);
        let defined: Vec<String> = PAT
            .def
            .captures_iter(&text)
            .filter_map(|c| c.get(1).map(|m| m.as_str().to_string()))
            .collect();
        let primary = tags
            .module_override
            .clone()
            .or_else(|| defined.first().cloned());

        for comment in file_comments {
            comments.push(ElinComment {
                module: primary.clone(),
                ..comment
            });
        }

        if tags.ignore {
            continue;
        }

        let display_path = tags.file_bind.clone().unwrap_or_else(|| rel.clone());
        let mut notes = tags.notes.clone();
        for dep in &tags.deps {
            notes.push(format!("dep {dep}"));
        }
        let uses: Vec<String> = PAT
            .use_mod
            .captures_iter(&text)
            .map(|c| c[1].to_string())
            .collect();
        let behaviours: Vec<String> = PAT
            .behaviour
            .captures_iter(&text)
            .map(|c| c[1].to_string())
            .collect();
        let loc = text.lines().filter(|l| !l.trim().is_empty()).count() as u32;
        let defs = PAT.def_fun.find_iter(&text).count() as u32;
        let defps = PAT.defp_fun.find_iter(&text).count() as u32;
        for name in defined.iter().chain(tags.module_override.iter()) {
            let role = infer_role(kind, name, &uses, &behaviours);
            nodes_by_id.entry(name.clone()).or_insert_with(|| GraphNode {
                label: name.rsplit('.').next().unwrap_or(name).to_string(),
                id: name.clone(),
                path: Some(display_path.clone()),
                kind: kind.into(),
                git: None,
                boundary: tags.boundary.clone(),
                notes: notes.clone(),
                ignored: false,
                wired: false,
                role,
                loc,
                defs,
                defps,
                behaviours: behaviours.clone(),
                fan_in: 0,
                fan_out: 0,
            });
        }

        let Some(from) = primary else {
            continue;
        };

        let mut aliases: HashMap<String, String> = HashMap::new();
        for cap in PAT.alias_as.captures_iter(&text) {
            let target = cap[1].to_string();
            let local = cap[2].to_string();
            aliases.insert(local, target.clone());
            refs.push((from.clone(), target, "alias".into()));
        }
        for cap in PAT.alias_curly.captures_iter(&text) {
            let prefix = &cap[1];
            for part in cap[2].split(',') {
                let name = part.trim();
                if name.is_empty() {
                    continue;
                }
                let target = format!("{prefix}.{name}");
                aliases.insert(name.to_string(), target.clone());
                refs.push((from.clone(), target, "alias".into()));
            }
        }
        for cap in PAT.alias.captures_iter(&text) {
            let Some(full) = cap.get(0) else {
                continue;
            };
            let line_start = text[..full.start()].rfind('\n').map(|i| i + 1).unwrap_or(0);
            let line_end = text[full.start()..]
                .find('\n')
                .map(|i| full.start() + i)
                .unwrap_or(text.len());
            let line = &text[line_start..line_end];
            if line.contains(", as:") || line.contains(".{") {
                continue;
            }
            let target = cap[1].to_string();
            if aliases.values().any(|v| v == &target) {
                continue;
            }
            let local = target.rsplit('.').next().unwrap_or(&target).to_string();
            aliases.insert(local, target.clone());
            refs.push((from.clone(), target, "alias".into()));
        }
        for cap in PAT.import.captures_iter(&text) {
            refs.push((from.clone(), cap[1].to_string(), "import".into()));
        }
        for cap in PAT.use_mod.captures_iter(&text) {
            refs.push((from.clone(), cap[1].to_string(), "use".into()));
        }
        for cap in PAT.delegate.captures_iter(&text) {
            refs.push((from.clone(), cap[1].to_string(), "delegate".into()));
        }
        for cap in PAT.require.captures_iter(&text) {
            refs.push((from.clone(), cap[1].to_string(), "require".into()));
        }
        for cap in PAT.call.captures_iter(&text) {
            let target = cap[1].to_string();
            if target == from {
                continue;
            }
            refs.push((from.clone(), target, "call".into()));
        }
        local_aliases.insert(from, aliases);
    }

    let project_modules: BTreeSet<String> = nodes_by_id.keys().cloned().collect();
    let mut seen_edges = BTreeSet::new();
    let mut edges = Vec::new();
    for (from, to, kind) in refs {
        let resolved = local_aliases
            .get(&from)
            .and_then(|map| map.get(&to).cloned())
            .unwrap_or(to);
        if !project_modules.contains(&resolved) || resolved == from {
            continue;
        }
        if !seen_edges.insert((from.clone(), resolved.clone(), kind.clone())) {
            continue;
        }
        edges.push(GraphEdge {
            from,
            to: resolved,
            kind,
            is_new: false,
        });
    }

    let wired: BTreeSet<String> = edges
        .iter()
        .flat_map(|e| [e.from.clone(), e.to.clone()])
        .collect();
    let mut fan_in: HashMap<String, u32> = HashMap::new();
    let mut fan_out: HashMap<String, u32> = HashMap::new();
    for edge in &edges {
        *fan_out.entry(edge.from.clone()).or_default() += 1;
        *fan_in.entry(edge.to.clone()).or_default() += 1;
    }
    for node in nodes_by_id.values_mut() {
        node.wired = wired.contains(&node.id);
        node.fan_in = fan_in.get(&node.id).copied().unwrap_or(0);
        node.fan_out = fan_out.get(&node.id).copied().unwrap_or(0);
    }

    let cycles = find_cycles(&edges);
    let tests = nodes_by_id.values().filter(|n| n.kind == "test").count() as u32;
    let unwired = nodes_by_id
        .values()
        .filter(|n| !n.wired && n.kind != "test")
        .count() as u32;
    let stats = GraphStats {
        modules: nodes_by_id.len() as u32,
        tests,
        edges: edges.len() as u32,
        unwired,
        cycles: cycles.len() as u32,
    };

    let module_by_rel: HashMap<String, String> = nodes_by_id
        .values()
        .filter_map(|n| n.path.as_ref().map(|p| (normalize_rel(p), n.id.clone())))
        .collect();
    for file in &mut files {
        if !file.is_dir {
            file.module = module_by_rel.get(&normalize_rel(&file.rel)).cloned();
        }
    }

    ModuleGraph {
        nodes: nodes_by_id.into_values().collect(),
        edges,
        comments,
        files,
        stats,
        cycles,
    }
}

pub fn parse_elin_comments(text: &str, file: &str) -> Vec<ElinComment> {
    let mut out = Vec::new();
    for (idx, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        let payload = if let Some(rest) = trimmed.strip_prefix("# elin:") {
            rest
        } else if let Some(rest) = trimmed.strip_prefix("#elin:") {
            rest
        } else if let Some(rest) = trimmed.strip_prefix("@elin") {
            rest.trim_start_matches(':').trim()
        } else {
            continue;
        };
        let payload = payload.trim();
        if payload.is_empty() {
            continue;
        }
        let (tag, value) = match payload.split_once(char::is_whitespace) {
            Some((tag, rest)) => (tag.to_ascii_lowercase(), rest.trim().to_string()),
            None => (payload.to_ascii_lowercase(), String::new()),
        };
        out.push(ElinComment {
            file: file.into(),
            line: (idx + 1) as u32,
            tag,
            value,
            module: None,
        });
    }
    out
}

const COMMENT_TAGS: &[&str] = &["note", "todo", "warn", "ignore", "boundary"];

/// Insert or replace `# elin:<tag> …` just above `defmodule` in a source file.
pub fn insert_comment(project_path: &str, rel: &str, tag: &str, value: &str) -> AppResult<()> {
    let tag = tag.trim().to_ascii_lowercase();
    if !COMMENT_TAGS.contains(&tag.as_str()) {
        return Err(AppError::msg("Unknown tag. Use note, todo, warn, ignore, or boundary."));
    }
    let value = value.trim().replace(['\n', '\r'], " ");
    if tag != "ignore" && value.is_empty() {
        return Err(AppError::msg("Write a short note."));
    }
    if value.len() > 240 {
        return Err(AppError::msg("Keep notes under 240 characters."));
    }
    let rel = rel.replace('\\', "/");
    if rel.contains("..") {
        return Err(AppError::msg("That path looks unsafe."));
    }
    let root = PathBuf::from(project_path);
    let path = root.join(rel.replace('/', std::path::MAIN_SEPARATOR_STR));
    if !path.starts_with(&root) || !path.is_file() {
        return Err(AppError::msg("That source file is not in this project."));
    }
    let text = fs::read_to_string(&path)?;
    let next = splice_comment(&text, &tag, &value);
    if next != text {
        fs::write(&path, next)?;
    }
    Ok(())
}

fn splice_comment(text: &str, tag: &str, value: &str) -> String {
    let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let ends_nl = text.ends_with('\n');
    let mut lines: Vec<String> = text.lines().map(|l| l.to_string()).collect();
    let fresh = if tag == "ignore" {
        "# elin:ignore".to_string()
    } else {
        format!("# elin:{tag} {value}")
    };
    let def_idx = lines.iter().position(|l| {
        l.trim_start().starts_with("defmodule ") && l.contains(" do")
    });
    let Some(def_idx) = def_idx else {
        lines.insert(0, fresh);
        return join_lines(&lines, newline, ends_nl);
    };
    let indent: String = lines[def_idx]
        .chars()
        .take_while(|c| c.is_whitespace())
        .collect();
    let prefixed = format!("{indent}{fresh}");
    let mut replace_at = None;
    for i in (0..def_idx).rev() {
        let trimmed = lines[i].trim();
        if trimmed.is_empty() {
            continue;
        }
        let hit = trimmed.starts_with(&format!("# elin:{tag}"))
            || trimmed.starts_with(&format!("#elin:{tag}"));
        if hit {
            replace_at = Some(i);
            break;
        }
        if !trimmed.starts_with('#') && !trimmed.starts_with('@') {
            break;
        }
    }
    if let Some(i) = replace_at {
        lines[i] = prefixed;
    } else {
        lines.insert(def_idx, prefixed);
    }
    join_lines(&lines, newline, ends_nl)
}

fn join_lines(lines: &[String], newline: &str, ends_nl: bool) -> String {
    let mut out = lines.join(newline);
    if ends_nl {
        out.push_str(newline);
    }
    out
}

fn parse_file_tags(text: &str) -> FileTags {
    let mut tags = FileTags::default();
    for comment in parse_elin_comments(text, "") {
        match comment.tag.as_str() {
            "ignore" => tags.ignore = true,
            "mod" if !comment.value.is_empty() => tags.module_override = Some(comment.value),
            "file" if !comment.value.is_empty() => tags.file_bind = Some(comment.value),
            "boundary" if !comment.value.is_empty() => {
                tags.boundary = Some(comment.value.to_ascii_lowercase())
            }
            "dep" if !comment.value.is_empty() => tags.deps.push(comment.value),
            "note" if !comment.value.is_empty() => tags.notes.push(comment.value),
            _ => {}
        }
    }
    tags
}

fn infer_role(kind: &str, name: &str, uses: &[String], behaviours: &[String]) -> String {
    if kind == "test" || name.ends_with("Test") {
        return "test".into();
    }
    let blob = uses
        .iter()
        .chain(behaviours)
        .map(|s| s.as_str())
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase();
    if blob.contains("liveview") || blob.contains("live_view") {
        return "liveview".into();
    }
    if blob.contains("phoenix.controller") {
        return "controller".into();
    }
    if blob.contains("ecto.schema") {
        return "schema".into();
    }
    if blob.contains("phoenix.router") {
        return "router".into();
    }
    if blob.contains("supervisor") {
        return "supervisor".into();
    }
    if blob.contains("genserver") {
        return "genserver".into();
    }
    if blob.contains("agent") {
        return "agent".into();
    }
    "module".into()
}

fn find_cycles(edges: &[GraphEdge]) -> Vec<Vec<String>> {
    let mut adj: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for edge in edges {
        adj.entry(edge.from.clone())
            .or_default()
            .push(edge.to.clone());
    }
    let mut cycles = Vec::new();
    let mut stack = Vec::new();
    let mut on_stack = BTreeSet::new();
    let mut color: BTreeMap<String, u8> = BTreeMap::new();

    fn visit(
        node: &str,
        adj: &BTreeMap<String, Vec<String>>,
        stack: &mut Vec<String>,
        on_stack: &mut BTreeSet<String>,
        color: &mut BTreeMap<String, u8>,
        cycles: &mut Vec<Vec<String>>,
    ) {
        if cycles.len() >= 12 {
            return;
        }
        color.insert(node.to_string(), 1);
        stack.push(node.to_string());
        on_stack.insert(node.to_string());
        if let Some(nexts) = adj.get(node) {
            for next in nexts {
                let state = color.get(next).copied().unwrap_or(0);
                if state == 0 {
                    visit(next, adj, stack, on_stack, color, cycles);
                } else if on_stack.contains(next) {
                    if let Some(idx) = stack.iter().position(|s| s == next) {
                        let mut cycle = stack[idx..].to_vec();
                        cycle.push(next.clone());
                        if !cycles.iter().any(|c| same_cycle(c, &cycle)) {
                            cycles.push(cycle);
                        }
                    }
                }
                if cycles.len() >= 12 {
                    break;
                }
            }
        }
        stack.pop();
        on_stack.remove(node);
        color.insert(node.to_string(), 2);
    }

    let keys: Vec<String> = adj.keys().cloned().collect();
    for start in keys {
        if color.get(&start).copied().unwrap_or(0) == 0 {
            visit(
                &start,
                &adj,
                &mut stack,
                &mut on_stack,
                &mut color,
                &mut cycles,
            );
        }
    }
    cycles
}

fn same_cycle(a: &[String], b: &[String]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let a_body = &a[..a.len().saturating_sub(1)];
    let b_body = &b[..b.len().saturating_sub(1)];
    if a_body.is_empty() {
        return b_body.is_empty();
    }
    b_body.iter().cycle().take(b_body.len() * 2).collect::<Vec<_>>()
        .windows(a_body.len())
        .any(|w| w.iter().copied().eq(a_body.iter()))
}

fn collect_tree(root: &Path) -> Vec<ProjectEntry> {
    let mut files = Vec::new();
    for name in ["mix.exs", "mix.lock", ".formatter.exs", ".credo.exs"] {
        let path = root.join(name);
        if path.is_file() {
            files.push(ProjectEntry {
                rel: name.into(),
                is_dir: false,
                module: None,
            });
        }
    }
    for dir_name in ["lib", "test", "config"] {
        let dir = root.join(dir_name);
        if !dir.exists() {
            continue;
        }
        files.push(ProjectEntry {
            rel: dir_name.into(),
            is_dir: true,
            module: None,
        });
        for entry in WalkDir::new(&dir).into_iter().flatten() {
            let path = entry.path();
            if path == dir || skip_walk(path) {
                continue;
            }
            let rel = rel_to(root, path);
            files.push(ProjectEntry {
                rel,
                is_dir: path.is_dir(),
                module: None,
            });
        }
    }
    files.sort_by(|a, b| a.rel.replace('\\', "/").cmp(&b.rel.replace('\\', "/")));
    files
}

fn skip_walk(path: &Path) -> bool {
    path.components().any(|c| {
        matches!(
            c.as_os_str().to_string_lossy().as_ref(),
            "deps" | "_build" | ".elixir_ls" | "node_modules" | ".git" | "assets" | "priv"
        )
    })
}

fn rel_to(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn normalize_rel(rel: &str) -> String {
    rel.replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn scratch(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("elin-{name}-{nanos}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("lib")).unwrap();
        fs::create_dir_all(dir.join("test")).unwrap();
        dir
    }

    #[test]
    fn parses_elin_comment_variants() {
        let text = "# elin:boundary ui\n#elin:ignore\n@elin note hello\n# elin:mod Foo.Bar\n";
        let comments = parse_elin_comments(text, "lib/x.ex");
        assert_eq!(comments.len(), 4);
        assert_eq!(comments[0].tag, "boundary");
        assert_eq!(comments[0].value, "ui");
        assert_eq!(comments[1].tag, "ignore");
        assert_eq!(comments[2].tag, "note");
        assert_eq!(comments[2].value, "hello");
        assert_eq!(comments[3].tag, "mod");
    }

    #[test]
    fn unknown_tags_are_kept_but_do_not_crash() {
        let comments = parse_elin_comments("# elin:future-thing xyz\n", "a.ex");
        assert_eq!(comments[0].tag, "future-thing");
    }

    #[test]
    fn graph_maps_module_to_file_and_edges() {
        let dir = scratch("graph");
        fs::write(
            dir.join("lib/foo.ex"),
            r#"
defmodule Demo.Foo do
  # elin:boundary core
  # elin:note entry
  alias Demo.Bar
  use Demo.Baz
  def run, do: Demo.Bar.ping()
end
"#,
        )
        .unwrap();
        fs::write(
            dir.join("lib/bar.ex"),
            "defmodule Demo.Bar do\n  def ping, do: :ok\nend\n",
        )
        .unwrap();
        fs::write(
            dir.join("lib/baz.ex"),
            "defmodule Demo.Baz do\n  defmacro __using__(_), do: :ok\nend\n",
        )
        .unwrap();
        fs::write(
            dir.join("test/foo_test.exs"),
            "defmodule Demo.FooTest do\n  alias Demo.Foo\nend\n",
        )
        .unwrap();

        let graph = analyze_path(&dir);
        let foo = graph.nodes.iter().find(|n| n.id == "Demo.Foo").unwrap();
        assert_eq!(foo.path.as_deref(), Some("lib/foo.ex"));
        assert_eq!(foo.boundary.as_deref(), Some("core"));
        assert_eq!(foo.notes, vec!["entry".to_string()]);
        assert!(foo.wired);
        assert!(graph.edges.iter().any(|e| e.from == "Demo.Foo" && e.to == "Demo.Bar" && e.kind == "alias"));
        assert!(graph.edges.iter().any(|e| e.from == "Demo.Foo" && e.to == "Demo.Baz" && e.kind == "use"));
        assert!(graph.edges.iter().any(|e| e.from == "Demo.Foo" && e.to == "Demo.Bar" && e.kind == "call"));
        let test = graph.nodes.iter().find(|n| n.id == "Demo.FooTest").unwrap();
        assert_eq!(test.kind, "test");
        assert_eq!(test.role, "test");
        assert_eq!(foo.role, "module");
        assert!(foo.defs >= 1);
        assert_eq!(graph.stats.modules, 4);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn infers_genserver_role_and_cycle() {
        let dir = scratch("role");
        fs::write(
            dir.join("lib/a.ex"),
            "defmodule Demo.A do\n  use GenServer\n  alias Demo.B\nend\n",
        )
        .unwrap();
        fs::write(
            dir.join("lib/b.ex"),
            "defmodule Demo.B do\n  alias Demo.A\nend\n",
        )
        .unwrap();
        let graph = analyze_path(&dir);
        let a = graph.nodes.iter().find(|n| n.id == "Demo.A").unwrap();
        assert_eq!(a.role, "genserver");
        assert!(graph.stats.cycles >= 1);
        assert!(graph.cycles.iter().any(|c| c.contains(&"Demo.A".into()) && c.contains(&"Demo.B".into())));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn dep_comment_lands_on_node_notes() {
        let dir = scratch("dep");
        fs::write(
            dir.join("lib/foo.ex"),
            "# elin:dep jason\ndefmodule Demo.Dep do\nend\n",
        )
        .unwrap();
        let graph = analyze_path(&dir);
        let node = graph.nodes.iter().find(|n| n.id == "Demo.Dep").unwrap();
        assert_eq!(node.notes, vec!["dep jason".to_string()]);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn ignore_comment_drops_file_from_graph() {
        let dir = scratch("ignore");
        fs::write(
            dir.join("lib/gen.ex"),
            "# elin:ignore\ndefmodule Demo.Gen do\nend\n",
        )
        .unwrap();
        fs::write(dir.join("lib/keep.ex"), "defmodule Demo.Keep do\nend\n").unwrap();
        let graph = analyze_path(&dir);
        assert!(graph.nodes.iter().all(|n| n.id != "Demo.Gen"));
        assert!(graph.nodes.iter().any(|n| n.id == "Demo.Keep"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn curly_alias_and_delegate_become_edges() {
        let dir = scratch("alias");
        fs::write(
            dir.join("lib/web.ex"),
            r#"
defmodule Demo.Web do
  alias Demo.{Alpha, Beta}
  defdelegate ping(), to: Demo.Alpha
end
"#,
        )
        .unwrap();
        fs::write(dir.join("lib/alpha.ex"), "defmodule Demo.Alpha do\nend\n").unwrap();
        fs::write(dir.join("lib/beta.ex"), "defmodule Demo.Beta do\nend\n").unwrap();
        let graph = analyze_path(&dir);
        assert!(graph.edges.iter().any(|e| e.to == "Demo.Alpha" && e.kind == "alias"));
        assert!(graph.edges.iter().any(|e| e.to == "Demo.Beta" && e.kind == "alias"));
        assert!(graph.edges.iter().any(|e| e.to == "Demo.Alpha" && e.kind == "delegate"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn unwired_module_is_flagged() {
        let dir = scratch("wire");
        fs::write(dir.join("lib/lonely.ex"), "defmodule Demo.Lonely do\nend\n").unwrap();
        let graph = analyze_path(&dir);
        let node = graph.nodes.iter().find(|n| n.id == "Demo.Lonely").unwrap();
        assert!(!node.wired);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn splice_inserts_and_replaces_note() {
        let src = "defmodule Demo.A do\nend\n";
        let once = splice_comment(src, "note", "hello");
        assert!(once.starts_with("# elin:note hello\n"));
        let twice = splice_comment(&once, "note", "world");
        assert_eq!(twice.matches("# elin:note").count(), 1);
        assert!(twice.contains("# elin:note world"));
        let ignore = splice_comment(&twice, "ignore", "");
        assert!(ignore.contains("# elin:ignore"));
        assert!(ignore.contains("# elin:note world"));
    }
}
