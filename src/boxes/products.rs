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
    /// `host` = this GSV crate (the rust-folder brain). `plugin` = every other tree.
    pub role: String,
    /// Plugin nested under the kit (Telenetis exception), not a sibling.
    pub nested: bool,
    /// Cargo `target/` for this tree. Drain uses this dir; never share with another plugin.
    pub target_dir: String,
    /// Host-only live copy (`target/live/`). Do not kill before GSV `cargo test`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub live_dir: Option<String>,
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
    /// Keep-live heartbeat file (llama-rs only); `heartbeat_alive` mirrors age ≤ 60s.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub heartbeat_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub heartbeat_alive: Option<bool>,
    pub role: String,
    pub nested: bool,
    pub target_dir: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub live_dir: Option<String>,
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

fn ecosystem_role(kit_root: &Path, path: &Path) -> &'static str {
    if canon_key(kit_root) == canon_key(path) {
        "host"
    } else {
        "plugin"
    }
}

fn is_nested_plugin(kit_root: &Path, path: &Path) -> bool {
    if ecosystem_role(kit_root, path) != "plugin" {
        return false;
    }
    let root = canon_key(kit_root);
    let child = canon_key(path);
    child.starts_with(&format!("{root}/"))
}

fn target_dir_of(path: &Path) -> String {
    let base = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    display_path(&base.join("target"))
}

fn live_dir_of(role: &str, path: &Path) -> Option<String> {
    if role == "host" {
        let base = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        Some(display_path(&base.join("target/live")))
    } else {
        None
    }
}

/// Plugin ids that contain the GSV-only `agi` skill (kit must not copy into plugins).
pub fn kit_skill_leaked_into_plugins(kit_root: &Path) -> Vec<String> {
    discover(kit_root)
        .into_iter()
        .filter(|r| r.role == "plugin")
        .filter_map(|r| {
            let p = PathBuf::from(&r.path);
            if p.join(".agents/skills/agi").is_dir() || p.join(".cursor/skills/agi").is_dir() {
                Some(r.id)
            } else {
                None
            }
        })
        .collect()
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
    let role = ecosystem_role(kit_root, path);
    out.push(ProductRow {
        registered: is_registered(kit_root, &id),
        id,
        name,
        path: display_path(&fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())),
        kind: kind.into(),
        source: source.into(),
        git,
        cargo,
        role: role.into(),
        nested: is_nested_plugin(kit_root, path),
        target_dir: target_dir_of(path),
        live_dir: live_dir_of(role, path),
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
    let (heartbeat_path, heartbeat_alive) = heartbeat_of(&row.id, &path);
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
        heartbeat_path,
        heartbeat_alive,
        role: row.role.clone(),
        nested: row.nested,
        target_dir: row.target_dir.clone(),
        live_dir: row.live_dir.clone(),
    })
}

/// Heartbeat enrichment for keep-live products (llama-rs today). Resolves the
/// `LLAMA_HEARTBEAT_PATH` override else `<root>/target/live/llama_heartbeat.json`
/// — the file `keep_live` reads — and mirrors its freshness (age ≤ 60s) using the
/// same [`keep_live::heartbeat_fresh`] helper so the scan and the box agree.
fn heartbeat_of(id: &str, root: &Path) -> (Option<String>, Option<bool>) {
    if id != "llama-rs" {
        return (None, None);
    }
    let path = std::env::var_os("LLAMA_HEARTBEAT_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("target/live/llama_heartbeat.json"));
    let alive = crate::boxes::keep_live::heartbeat_fresh(&path, now_unix());
    (Some(display_path(&path)), Some(alive))
}

fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
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

    #[test]
    fn heartbeat_of_only_enriches_llama_rs() {
        let _guard = crate::boxes::ENV_LOCK
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        let root = Path::new("S:/rust/llama-rs");
        assert_eq!(heartbeat_of("gsv", root), (None, None));
        // Override to a never-written path: a live `llama_serve` heartbeat on
        // this box must not flip the no-file freshness expectation.
        let missing = std::env::temp_dir().join(format!(
            "gsv-products-hb-missing-{}.json",
            std::process::id()
        ));
        std::env::set_var("LLAMA_HEARTBEAT_PATH", &missing);
        let (path, alive) = heartbeat_of("llama-rs", root);
        std::env::remove_var("LLAMA_HEARTBEAT_PATH");
        let (_path, alive) = (path.expect("path"), alive.expect("alive"));
        // With no file the freshness mirrors keep_live (false), not a panic.
        assert!(!alive);
        // Default shape still points at the llama-rs live heartbeat.
        let (default_path, _) = heartbeat_of("llama-rs", root);
        assert!(default_path
            .expect("default path")
            .ends_with("target/live/llama_heartbeat.json"));
    }

    #[test]
    fn heartbeat_of_respects_env_override_and_freshness() {
        let _guard = crate::boxes::ENV_LOCK
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        let dir = std::env::temp_dir().join(format!("gsv-products-hb-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let file = dir.join("heartbeat.json");
        std::env::set_var("LLAMA_HEARTBEAT_PATH", &file);
        fs::write(
            &file,
            format!(
                r#"{{"pid":1,"model":"m","epoch_secs":{},"bin_version":"0.0.0"}}"#,
                now_unix() - 10
            ),
        )
        .expect("write hb");
        let root = Path::new("S:/rust/llama-rs");
        let (path, alive) = heartbeat_of("llama-rs", root);
        let expected = file.to_string_lossy().replace('\\', "/");
        assert_eq!(path.as_deref(), Some(expected.as_str()));
        assert_eq!(alive, Some(true));
        std::env::remove_var("LLAMA_HEARTBEAT_PATH");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn gsv_is_host_with_live_dir() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let rows = discover(&root);
        let gsv = rows.iter().find(|r| r.id == "gsv").expect("gsv");
        assert_eq!(gsv.role, "host");
        assert!(!gsv.nested);
        assert!(gsv.target_dir.replace('\\', "/").ends_with("/GSV/target"));
        assert!(gsv
            .live_dir
            .as_deref()
            .unwrap_or("")
            .replace('\\', "/")
            .ends_with("/GSV/target/live"));
    }

    #[test]
    fn non_gsv_rows_are_plugins_with_own_target() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let rows = discover(&root);
        for row in rows.iter().filter(|r| r.id != "gsv") {
            assert_eq!(row.role, "plugin", "{}", row.id);
            assert!(row.live_dir.is_none(), "{}", row.id);
            assert!(
                row.target_dir.ends_with("/target") || row.target_dir.ends_with("\\target"),
                "{} target={}",
                row.id,
                row.target_dir
            );
            assert!(
                !row.target_dir.contains(".."),
                "target_dir must be canonical: {} {}",
                row.id,
                row.target_dir
            );
            assert_ne!(
                row.target_dir.replace('\\', "/").to_ascii_lowercase(),
                format!("{}/target", gsv_target_prefix(&root)),
                "plugin {} must not share GSV target/",
                row.id
            );
        }
    }

    fn gsv_target_prefix(root: &Path) -> String {
        display_path(&root.join("target"))
            .replace('\\', "/")
            .trim_end_matches("/target")
            .to_string()
    }

    #[test]
    fn telenetis_nested_plugin_is_exception() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let rows = discover(&root);
        if let Some(t) = rows.iter().find(|r| r.id == "telenetis") {
            assert_eq!(t.role, "plugin");
            assert!(t.nested, "telenetis lives under GSV, not a sibling");
        }
    }

    #[test]
    fn kit_agi_skill_stays_out_of_plugins() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let leaked = kit_skill_leaked_into_plugins(&root);
        assert!(
            leaked.is_empty(),
            "agi skill copied into plugins: {leaked:?}"
        );
    }
}
