//! Cursor user-hook: inject GSV hub rules into rust-environment windows.
//!
//! `sessionStart` reads stdin JSON and prints `{"additional_context":...}` only
//! when a workspace root sits under the rust folder (`S:/rust`, parent of the
//! hub) and none of the roots is the hub itself. Other folders get `{}`.
//! User-scope MCP is not part of this bin.
//!
//! Install/refresh: `cargo build --release --bin gsv-cursor-rules`, then copy
//! `target/release/gsv-cursor-rules.exe` to `%USERPROFILE%\.cursor\hooks\`
//! (referenced by `hooks.json` `sessionStart`). New chats pick it up.

use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use serde_json::Value;

fn main() {
    let mut raw = String::new();
    let _ = io::stdin().read_to_string(&mut raw);
    let roots = workspace_roots(&raw);
    let hub = hub_dir();
    let payload = hook_output(&roots, &hub);
    let line = serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string());
    let mut out = io::stdout().lock();
    let _ = writeln!(out, "{line}");
}

fn hub_dir() -> PathBuf {
    std::env::var_os("GSV_HUB")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"S:\rust\GSV"))
}

fn workspace_roots(raw: &str) -> Vec<String> {
    let Ok(value) = serde_json::from_str::<Value>(raw) else {
        return Vec::new();
    };
    value
        .get("workspace_roots")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn norm_path(path: &str) -> String {
    let mut norm = path
        .replace('\\', "/")
        .trim_end_matches('/')
        .to_ascii_lowercase();
    if let Some(rest) = norm.strip_prefix('/') {
        let bytes = rest.as_bytes();
        if bytes.len() >= 2 && bytes[1] == b'/' && bytes[0].is_ascii_alphabetic() {
            let drive = bytes[0] as char;
            norm = format!("{drive}:{}", &rest[1..]);
        }
    }
    norm
}

fn is_gsv_root(path: &str) -> bool {
    norm_path(path).ends_with("/rust/gsv")
}

fn rust_env_root(hub: &Path) -> String {
    let hub_norm = norm_path(&hub.to_string_lossy());
    hub_norm
        .rsplit_once('/')
        .map(|(parent, _)| parent.to_string())
        .unwrap_or(hub_norm)
}

fn in_rust_env(path: &str, hub: &Path) -> bool {
    let path = norm_path(path);
    let root = rust_env_root(hub);
    path == root || path.starts_with(&(root.clone() + "/"))
}

fn hook_output(roots: &[String], hub: &Path) -> Value {
    if roots.is_empty() || roots.iter().any(|root| is_gsv_root(root)) {
        return serde_json::json!({});
    }
    if !roots.iter().any(|root| in_rust_env(root, hub)) {
        return serde_json::json!({});
    }
    let text = collect(hub);
    if text.is_empty() {
        serde_json::json!({})
    } else {
        serde_json::json!({ "additional_context": text })
    }
}

fn collect(hub: &Path) -> String {
    let mut out = String::from(
        "GSV hub global rules for a window inside the rust environment (S:/rust). \
The hub folder itself is not open, so these rules are attached here. \
Product rules in the open folder still cover that product only. \
Do not install User-scope MCP.\n",
    );
    push_file(&mut out, &hub.join("AGENTS.md"), "AGENTS.md");
    let mut rules = Vec::new();
    if let Ok(entries) = fs::read_dir(hub.join(".cursor").join("rules")) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) == Some("mdc") {
                rules.push(path);
            }
        }
    }
    rules.sort();
    for path in rules {
        let label = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("rule.mdc");
        let owned = label.to_string();
        push_file(&mut out, &path, &owned);
    }
    if out.lines().count() <= 1 {
        String::new()
    } else {
        out
    }
}

fn push_file(out: &mut String, path: &Path, label: &str) {
    let Ok(raw) = fs::read_to_string(path) else {
        return;
    };
    let body = strip_frontmatter(&raw);
    if body.trim().is_empty() {
        return;
    }
    out.push_str("\n## ");
    out.push_str(label);
    out.push('\n');
    out.push_str(body);
    if !body.ends_with('\n') {
        out.push('\n');
    }
}

fn strip_frontmatter(raw: &str) -> &str {
    let raw = raw.trim_start_matches('\u{feff}');
    let Some(after_open) = raw
        .strip_prefix("---\r\n")
        .or_else(|| raw.strip_prefix("---\n"))
    else {
        return raw;
    };
    let Some(rel) = after_open.find("\n---") else {
        return raw;
    };
    let after_mark = after_open[rel + 1..].strip_prefix("---").unwrap_or("");
    after_mark
        .strip_prefix("\r\n")
        .or_else(|| after_mark.strip_prefix("\n"))
        .unwrap_or(after_mark)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gsv_root_spellings_match_only_the_hub() {
        assert!(is_gsv_root(r"S:\rust\GSV"));
        assert!(is_gsv_root("S:/rust/GSV/"));
        assert!(is_gsv_root("/s/rust/GSV"));
        assert!(!is_gsv_root(r"S:\rust\rusty-engine"));
        assert!(!is_gsv_root(r"S:\rust\GSV-extra"));
    }

    #[test]
    fn strip_frontmatter_keeps_body() {
        let raw = "---\nalwaysApply: true\n---\n# Title\n";
        assert_eq!(strip_frontmatter(raw), "# Title\n");
        assert_eq!(strip_frontmatter("# bare\n"), "# bare\n");
    }

    #[test]
    fn hub_window_stays_empty_and_other_windows_get_agents() {
        let hub = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let quiet = hook_output(&[r"S:\rust\GSV".to_string()], &hub);
        assert!(quiet.get("additional_context").is_none());
        let injected = hook_output(&[r"S:\rust\rusty-engine".to_string()], &hub);
        let text = injected["additional_context"].as_str().unwrap_or("");
        assert!(text.contains("Rust folder is the host"));
        assert!(text.contains("MSYS2"));
        assert!(!text.contains("alwaysApply:"));
        let msys = hook_output(&["/s/rust/poolAI".to_string()], &hub);
        assert!(msys.get("additional_context").is_some());
        let outside = hook_output(&[r"C:\Users\plati\notes".to_string()], &hub);
        assert!(outside.get("additional_context").is_none());
        let unknown = hook_output(&[], &hub);
        assert!(unknown.get("additional_context").is_none());
    }
}
