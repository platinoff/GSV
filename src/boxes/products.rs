//! VDT products picker — environment discovery, select, confined open, scan.
//!
//! Same merge as `cargo xtask products` (workspace folders ∪ sibling git ∪ kit).
//! Open path is an id in the discovered set.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// One discovered environment project (same columns as `cargo xtask products`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProductRow {
    pub id: String,
    pub name: String,
    pub path: String,
    pub kind: String,
    pub registered: bool,
    pub source: String,
    pub git: bool,
    pub cargo: bool,
}

/// Auto-parse of a selected product (no `cargo test`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProductScan {
    pub ok: bool,
    pub id: String,
    pub git_head: String,
    pub git_status_short: String,
    pub kind: String,
    pub registered: bool,
    pub handoff_exists: bool,
    pub next_exists: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cargo_name: Option<String>,
}

/// Display path as `S:/rust/...` (`/` not `\`).
pub fn display_path(p: &Path) -> String {
    let raw = p.to_string_lossy();
    let stripped = raw.strip_prefix("\\\\?\\").unwrap_or(&raw);
    stripped.replace('\\', "/")
}

fn canon_key(p: &Path) -> String {
    fs::canonicalize(p)
        .map(|c| display_path(&c).to_lowercase())
        .unwrap_or_else(|_| display_path(p).to_lowercase())
}

fn slug_of(path: &Path) -> String {
    path.file_name()
        .map(|s| {
            s.to_string_lossy()
                .to_lowercase()
                .chars()
                .filter(|c| !c.is_whitespace())
                .collect()
        })
        .unwrap_or_default()
}

fn is_registered(kit_root: &Path, id: &str) -> bool {
    let text = fs::read_to_string(kit_root.join("docs/gsv/PRODUCTS.md")).unwrap_or_default();
    let needle = format!("| **{id}**");
    text.to_ascii_lowercase()
        .contains(&needle.to_ascii_lowercase())
}

fn has_git(path: &Path) -> bool {
    let g = path.join(".git");
    g.is_dir() || g.is_file()
}

fn workspace_paths(kit_root: &Path) -> Vec<PathBuf> {
    let text = fs::read_to_string(kit_root.join("gsv.code-workspace")).unwrap_or_default();
    let Ok(v) = serde_json::from_str::<Value>(&text) else {
        return Vec::new();
    };
    let Some(folders) = v.get("folders").and_then(Value::as_array) else {
        return Vec::new();
    };
    folders
        .iter()
        .filter_map(|f| f.get("path").and_then(Value::as_str))
        .filter(|rel| !rel.is_empty())
        .map(|rel| {
            if rel == "." {
                kit_root.to_path_buf()
            } else {
                kit_root.join(rel)
            }
        })
        .collect()
}

fn sibling_git_dirs(kit_root: &Path) -> Vec<PathBuf> {
    let Some(parent) = kit_root.parent() else {
        return Vec::new();
    };
    let Ok(rd) = fs::read_dir(parent) else {
        return Vec::new();
    };
    let mut out: Vec<PathBuf> = rd
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir() && has_git(p))
        .collect();
    out.sort();
    out
}

fn push_row(
    kit_root: &Path,
    path: &Path,
    source: &str,
    seen: &mut HashSet<String>,
    out: &mut Vec<ProductRow>,
) {
    if !path.is_dir() {
        return;
    }
    let key = canon_key(path);
    if !seen.insert(key) {
        return;
    }
    let name = path
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "unknown".into());
    let id = slug_of(path);
    let git = has_git(path);
    let cargo = path.join("Cargo.toml").is_file();
    let node = path.join("package.json").is_file();
    let kind = if cargo {
        "rust"
    } else if node {
        "node"
    } else if git {
        "git"
    } else {
        "folder"
    };
    out.push(ProductRow {
        registered: is_registered(kit_root, &id),
        id,
        name,
        path: display_path(&fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())),
        kind: kind.into(),
        source: source.into(),
        git,
        cargo,
    });
}

/// Discover environment projects (workspace → siblings → kit). Dedup by path.
pub fn discover(kit_root: &Path) -> Vec<ProductRow> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for p in workspace_paths(kit_root) {
        push_row(kit_root, &p, "workspace", &mut seen, &mut out);
    }
    for p in sibling_git_dirs(kit_root) {
        push_row(kit_root, &p, "sibling", &mut seen, &mut out);
    }
    push_row(kit_root, kit_root, "kit", &mut seen, &mut out);
    out
}

/// Find a discovered row by id.
pub fn lookup<'a>(rows: &'a [ProductRow], id: &str) -> Option<&'a ProductRow> {
    rows.iter().find(|r| r.id == id)
}

fn git_capture(cwd: &Path, args: &[&str]) -> String {
    crate::vision::command("git")
        .current_dir(cwd)
        .args(args)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn parse_cargo_name(toml: &Path) -> Option<String> {
    let text = fs::read_to_string(toml).ok()?;
    let mut in_package = false;
    for line in text.lines() {
        let t = line.trim();
        if t.starts_with('[') {
            in_package = t == "[package]";
            continue;
        }
        if in_package {
            if let Some(rest) = t.strip_prefix("name") {
                // Only a plain `name = "…"` key — not `name.workspace`,
                // `namespaced`, … (mirror of fingerprint PH-S2633).
                if !rest.starts_with([' ', '\t', '=']) {
                    continue;
                }
                let rest = rest.trim().trim_start_matches('=').trim();
                let name = rest.trim_matches('"').trim_matches('\'').trim();
                if !name.is_empty() {
                    return Some(name.to_string());
                }
            }
        }
    }
    None
}

/// Metadata scan of one discovered id (git / HANDOFF / Cargo name). No `cargo test`.
pub fn scan(kit_root: &Path, id: &str) -> Result<ProductScan, String> {
    let rows = discover(kit_root);
    let row = lookup(&rows, id).ok_or_else(|| "unknown product".to_string())?;
    let path = PathBuf::from(&row.path);
    let handoff_exists = path.join("docs/HANDOFF_NEW_SESSION.md").is_file()
        || path
            .join("docs/development/HANDOFF_NEW_SESSION.md")
            .is_file()
        || path.join("AGENTS.md").is_file();
    let next_exists = path.join("docs/NEXT_SESSION_PROMPT.md").is_file()
        || path
            .join("docs/development/NEXT_SESSION_PROMPT.md")
            .is_file()
        || path.join("docs/ROADMAP.md").is_file();
    let cargo_name = if row.cargo {
        parse_cargo_name(&path.join("Cargo.toml"))
    } else {
        None
    };
    Ok(ProductScan {
        ok: true,
        id: row.id.clone(),
        git_head: git_capture(&path, &["rev-parse", "--short", "HEAD"]),
        git_status_short: git_capture(&path, &["status", "-sb"]),
        kind: row.kind.clone(),
        registered: row.registered,
        handoff_exists,
        next_exists,
        cargo_name,
    })
}

fn cmd_on_path(name: &str) -> bool {
    let Ok(path) = std::env::var("PATH") else {
        return false;
    };
    for dir in std::env::split_paths(&path) {
        for cand in [
            dir.join(name),
            dir.join(format!("{name}.exe")),
            dir.join(format!("{name}.cmd")),
        ] {
            if cand.is_file() {
                return true;
            }
        }
    }
    false
}

/// Open a discovered product folder (`cursor` if on PATH, else `explorer`).
///
/// Lookup is by id in [`discover`] — traversal is impossible. Cargo-test
/// harnesses skip the spawn (same `/deps/` gate as update apply).
pub fn open_folder(kit_root: &Path, id: &str) -> Result<String, String> {
    let rows = discover(kit_root);
    let row = lookup(&rows, id).ok_or_else(|| "unknown product".to_string())?;
    let how = if cmd_on_path("cursor") {
        "cursor"
    } else {
        "explorer"
    };
    if crate::boxes::update::is_cargo_test_harness() {
        return Ok(how.to_string());
    }
    let mut cmd = if how == "cursor" {
        crate::vision::command("cursor")
    } else {
        crate::vision::command("explorer.exe")
    };
    cmd.arg(&row.path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    cmd.spawn().map_err(|e| e.to_string())?;
    Ok(how.to_string())
}

/// `GET /api/products` wire.
pub fn wire(kit_root: &Path, selected: Option<&str>) -> Value {
    json!({
        "ok": true,
        "products": discover(kit_root),
        "selected": selected,
    })
}

/// Card wire: list + optional scan of the current selection.
pub fn card_wire(kit_root: &Path, selected: Option<&str>) -> Value {
    let mut w = wire(kit_root, selected);
    if let Some(id) = selected {
        if let Ok(scan) = scan(kit_root, id) {
            w["scan"] = json!(scan);
        }
    }
    w
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_path_strips_verbatim_and_backslashes() {
        let p = PathBuf::from("\\\\?\\S:\\rust\\GSV");
        assert_eq!(display_path(&p), "S:/rust/GSV");
    }

    #[test]
    fn parse_cargo_name_ignores_prefixed_and_workspace_keys() {
        let dir = std::env::temp_dir().join(format!("gsv-products-name-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let toml = dir.join("Cargo.toml");
        fs::write(
            &toml,
            "[package]\nname.workspace = true\nnamespace = \"x\"\n",
        )
        .expect("write toml");
        assert_eq!(
            parse_cargo_name(&toml),
            None,
            "prefixed keys must not parse"
        );
        fs::write(&toml, "[package]\nname = \"gsv\"\n").expect("write toml");
        assert_eq!(parse_cargo_name(&toml).as_deref(), Some("gsv"));
        fs::write(&toml, "[package]\nname=\"quoted\"\n").expect("write toml");
        assert_eq!(parse_cargo_name(&toml).as_deref(), Some("quoted"));
        let _ = fs::remove_dir_all(&dir);
    }
}
