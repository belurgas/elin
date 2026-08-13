//! Conservative mix.exs edits: insert or remove a `{:name, ...}` tuple.
//!
//! If the file is too unusual to patch safely, callers get an error string
//! they can show as a paste snippet. Never rewrite the rest of the file.

/// Insert `tuple` at the start of the project's `deps` list. Idempotent.
pub fn insert_dep(mix_exs: &str, tuple: &str) -> Result<String, String> {
    let name = dep_name(tuple).ok_or_else(|| "Could not read the package name from the tuple.".to_string())?;
    if dep_present(mix_exs, &name) {
        return Ok(mix_exs.to_string());
    }
    let open = find_deps_open(mix_exs)
        .ok_or_else(|| "Could not find a `deps` list in mix.exs. It may be generated.".to_string())?;
    let indent = detect_indent(mix_exs, open);
    let insert = format!("\n{indent}{tuple},");
    let mut out = String::with_capacity(mix_exs.len() + insert.len());
    out.push_str(&mix_exs[..=open]);
    out.push_str(&insert);
    out.push_str(&mix_exs[open + 1..]);
    Ok(out)
}

/// Remove a single-line (or brace-balanced) `{:name, ...}` tuple.
pub fn remove_dep(mix_exs: &str, name: &str) -> Result<String, String> {
    let needle = format!("{{:{name},");
    let Some(start) = mix_exs.find(&needle) else {
        return Ok(mix_exs.to_string());
    };
    let line_start = mix_exs[..start].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let rest = &mix_exs[start..];
    let mut depth = 0i32;
    let mut end = None;
    for (i, ch) in rest.char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end = Some(start + i + 1);
                    break;
                }
            }
            _ => {}
        }
    }
    let Some(tuple_end) = end else {
        return Err(format!("`{name}` looks nested; remove it in the editor."));
    };
    let mut line_end = tuple_end;
    let after = &mix_exs[tuple_end..];
    let trimmed = after.trim_start_matches([' ', '\t']);
    if trimmed.starts_with(',') {
        line_end = tuple_end + (after.len() - trimmed.len()) + 1;
    }
    if mix_exs[line_end..].starts_with('\r') {
        line_end += 1;
    }
    if mix_exs[line_end..].starts_with('\n') {
        line_end += 1;
    }
    let mut out = String::new();
    out.push_str(&mix_exs[..line_start]);
    out.push_str(&mix_exs[line_end..]);
    Ok(out)
}

pub fn dep_present(mix_exs: &str, name: &str) -> bool {
    mix_exs.contains(&format!("{{:{name},"))
}

fn dep_name(tuple: &str) -> Option<String> {
    let rest = tuple.trim().strip_prefix("{:")?;
    let name: String = rest
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect();
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

fn find_deps_open(text: &str) -> Option<usize> {
    for marker in ["defp deps do", "def deps do"] {
        if let Some(idx) = text.find(marker) {
            let after = &text[idx + marker.len()..];
            let rel = after.find('[')?;
            let open = idx + marker.len() + rel;
            if matching_bracket(text, open).is_some() {
                return Some(open);
            }
        }
    }
    if let Some(idx) = text.find("deps: [") {
        let open = idx + "deps: ".len();
        if matching_bracket(text, open).is_some() {
            return Some(open);
        }
    }
    None
}

fn matching_bracket(text: &str, open: usize) -> Option<usize> {
    let mut depth = 0i32;
    for (i, ch) in text[open..].char_indices() {
        match ch {
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    return Some(open + i);
                }
            }
            _ => {}
        }
    }
    None
}

fn detect_indent(text: &str, open: usize) -> String {
    let line_start = text[..open].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let line = &text[line_start..=open];
    let base: String = line.chars().take_while(|c| *c == ' ' || *c == '\t').collect();
    format!("{base}  ")
}

#[cfg(test)]
mod tests {
    use super::*;

    const STOCK: &str = r#"defmodule Hello.MixProject do
  use Mix.Project

  def project do
    [
      app: :hello,
      elixir: "~> 1.15",
      deps: deps()
    ]
  end

  defp deps do
    [
      # {:dep_from_hexpm, "~> 0.3.0"},
    ]
  end
end
"#;

    const WITH_CREDO: &str = r#"  defp deps do
    [
      {:credo, "~> 1.7", only: [:dev, :test], runtime: false},
      {:jason, "~> 1.4"}
    ]
  end
"#;

    const INLINE: &str = r#"def project do
  [
    app: :hello,
    deps: [
      {:phoenix, "~> 1.7.0"},
      {:jason, "~> 1.4"}
    ]
  ]
end
"#;

    #[test]
    fn insert_is_idempotent_when_present() {
        let tuple = r#"{:credo, "~> 1.7", only: [:dev, :test], runtime: false}"#;
        let once = insert_dep(STOCK, tuple).unwrap();
        assert!(dep_present(&once, "credo"));
        let twice = insert_dep(&once, tuple).unwrap();
        assert_eq!(once.matches("{:credo,").count(), 1);
        assert_eq!(twice, once);
    }

    #[test]
    fn insert_keeps_existing_comment_deps() {
        let tuple = r#"{:credo, "~> 1.7", only: [:dev, :test], runtime: false}"#;
        let next = insert_dep(STOCK, tuple).unwrap();
        assert!(next.contains("# {:dep_from_hexpm"));
        assert!(next.contains("{:credo,"));
    }

    #[test]
    fn insert_into_inline_deps_list() {
        let tuple = r#"{:credo, "~> 1.7", only: [:dev, :test], runtime: false}"#;
        let next = insert_dep(INLINE, tuple).unwrap();
        assert!(next.contains("{:credo,"));
        assert!(next.contains("{:phoenix,"));
    }

    #[test]
    fn remove_single_line_tuple() {
        let next = remove_dep(WITH_CREDO, "credo").unwrap();
        assert!(!dep_present(&next, "credo"));
        assert!(next.contains("{:jason,"));
    }

    #[test]
    fn remove_missing_is_ok() {
        let next = remove_dep(STOCK, "credo").unwrap();
        assert_eq!(next, STOCK);
    }
}
