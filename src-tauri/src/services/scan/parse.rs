//! Best-effort parsers for Mix tool stdout. Tools change JSON shapes;
//! unknown payloads fall back to line-oriented text findings.

use super::ScanFinding;

pub fn parse_credo_json(text: &str) -> Vec<ScanFinding> {
    let json = extract_json(text);
    let mut findings = Vec::new();
    let Ok(value) = serde_json::from_str::<serde_json::Value>(json) else {
        if text.contains("issues") || text.contains("found") {
            return parse_text_findings("credo", "credo", text);
        }
        return findings;
    };
    let issues = value
        .get("issues")
        .or_else(|| value.get("data").and_then(|d| d.get("issues")))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    for issue in issues {
        let message = issue
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("credo issue")
            .to_string();
        let file = issue
            .get("filename")
            .or_else(|| issue.get("file"))
            .and_then(|v| v.as_str())
            .map(str::to_string);
        let line = issue
            .get("line_no")
            .or_else(|| issue.get("line"))
            .and_then(|v| v.as_u64())
            .map(|n| n as u32);
        let priority = issue.get("priority").and_then(|v| v.as_i64()).unwrap_or(0);
        findings.push(ScanFinding {
            layer: "credo".into(),
            severity: if priority >= 10 { "error" } else { "warn" }.into(),
            file,
            line,
            message,
            tool: "credo".into(),
        });
    }
    findings
}

pub fn parse_mix_audit_json(text: &str) -> Vec<ScanFinding> {
    let json = extract_json(text);
    let Ok(value) = serde_json::from_str::<serde_json::Value>(json) else {
        return parse_text_findings("mix_audit", "deps.audit", text);
    };
    let mut findings = Vec::new();
    let vulns = value
        .get("vulnerabilities")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    if vulns.is_empty() {
        if let Some(arr) = value.as_array() {
            for item in arr {
                push_audit_item(&mut findings, item);
            }
        }
    } else {
        for item in vulns {
            push_audit_item(&mut findings, &item);
        }
    }
    findings
}

fn push_audit_item(findings: &mut Vec<ScanFinding>, item: &serde_json::Value) {
    let advisory = item.get("advisory").unwrap_or(item);
    let package = item
        .get("dependency")
        .and_then(|d| d.get("package"))
        .or_else(|| item.get("package"))
        .and_then(|v| v.as_str())
        .unwrap_or("dependency");
    let title = advisory
        .get("title")
        .or_else(|| advisory.get("description"))
        .and_then(|v| v.as_str())
        .unwrap_or("security advisory");
    findings.push(ScanFinding {
        layer: "mix_audit".into(),
        severity: "error".into(),
        file: Some("mix.lock".into()),
        line: None,
        message: format!("{package}: {title}"),
        tool: "deps.audit".into(),
    });
}

pub fn parse_sobelow_json(text: &str) -> Vec<ScanFinding> {
    let json = extract_json(text);
    let Ok(value) = serde_json::from_str::<serde_json::Value>(json) else {
        return parse_text_findings("sobelow", "sobelow", text);
    };
    let mut findings = Vec::new();
    let findings_val = value.get("findings").cloned().unwrap_or(value);
    collect_sobelow(&findings_val, &mut findings);
    findings
}

fn collect_sobelow(value: &serde_json::Value, out: &mut Vec<ScanFinding>) {
    match value {
        serde_json::Value::Array(arr) => {
            for item in arr {
                collect_sobelow(item, out);
            }
        }
        serde_json::Value::Object(map) => {
            if let Some(file) = map
                .get("file")
                .or_else(|| map.get("filename"))
                .and_then(|v| v.as_str())
            {
                let line = map.get("line").and_then(|v| v.as_u64()).map(|n| n as u32);
                let message = map
                    .get("type")
                    .or_else(|| map.get("message"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("sobelow finding");
                out.push(ScanFinding {
                    layer: "sobelow".into(),
                    severity: "warn".into(),
                    file: Some(file.into()),
                    line,
                    message: message.into(),
                    tool: "sobelow".into(),
                });
            } else {
                for v in map.values() {
                    collect_sobelow(v, out);
                }
            }
        }
        _ => {}
    }
}

pub fn parse_format_output(text: &str) -> Vec<ScanFinding> {
    let mut findings = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.ends_with(".ex") || line.ends_with(".exs") || line.contains("would be formatted") {
            let file = line
                .split_whitespace()
                .find(|p| p.ends_with(".ex") || p.ends_with(".exs"))
                .unwrap_or(line);
            findings.push(ScanFinding {
                layer: "format".into(),
                severity: "warn".into(),
                file: Some(file.to_string()),
                line: None,
                message: "not formatted".into(),
                tool: "format".into(),
            });
        }
    }
    findings
}

pub fn parse_text_findings(layer: &str, tool: &str, text: &str) -> Vec<ScanFinding> {
    let mut findings = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with("**")
            || line.contains("error")
            || line.contains("warning")
            || line.contains("CVE")
        {
            findings.push(ScanFinding {
                layer: layer.into(),
                severity: if line.to_ascii_lowercase().contains("error") || line.contains("CVE") {
                    "error".into()
                } else {
                    "warn".into()
                },
                file: None,
                line: None,
                message: truncate(line, 300),
                tool: tool.into(),
            });
        }
    }
    findings
}

fn extract_json(text: &str) -> &str {
    let start = text.find('{').or_else(|| text.find('[')).unwrap_or(0);
    text[start..].trim()
}

pub(super) fn truncate(text: &str, max: usize) -> String {
    let t = text.trim();
    if t.chars().count() <= max {
        t.to_string()
    } else {
        format!("{}…", t.chars().take(max).collect::<String>())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credo_json_extracts_file_line_message() {
        let json = r#"{"issues":[{"filename":"lib/foo.ex","line_no":12,"message":"Don't use IO.inspect","priority":12}]}"#;
        let findings = parse_credo_json(json);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].file.as_deref(), Some("lib/foo.ex"));
        assert_eq!(findings[0].line, Some(12));
        assert_eq!(findings[0].severity, "error");
    }

    #[test]
    fn credo_json_ignores_log_prefix() {
        let text = "Checking 3 source files...\n{\"issues\":[]}";
        assert!(parse_credo_json(text).is_empty());
    }

    #[test]
    fn format_output_lists_files() {
        let out = "** (Mix) mix format failed due to --check-formatted.\nlib/foo.ex\ntest/foo_test.exs\n";
        let findings = parse_format_output(out);
        assert!(findings.iter().any(|f| f.file.as_deref() == Some("lib/foo.ex")));
    }

    #[test]
    fn mix_audit_json_reads_package_title() {
        let json = r#"{"vulnerabilities":[{"dependency":{"package":"plug"},"advisory":{"title":"CVE-2024-1"}}]}"#;
        let findings = parse_mix_audit_json(json);
        assert_eq!(findings[0].message, "plug: CVE-2024-1");
        assert_eq!(findings[0].file.as_deref(), Some("mix.lock"));
    }
}
