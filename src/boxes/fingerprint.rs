//! Drain fingerprint JSONL — who / which client / which model / when / which product.
//!
//! Path: `{kit}/docs/gsv/fingerprints.jsonl` (git-tracked, never `data/`).
//! Each row is tagged with `product` so GSV crate semver is not mistaken for
//! poolAI / omniroute versions. Legacy rows without `product` deserialize as `gsv`.

use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

fn default_product() -> String {
    "gsv".to_string()
}

/// One drain close record.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Fingerprint {
    pub ts: String,
    pub actor: String,
    pub ide: String,
    pub model: String,
    pub agent: String,
    pub version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git_head: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub band: Option<String>,
    pub summary: String,
    /// VDT product id (`gsv` / `poolai` / `omniroute`). Missing JSONL → `gsv`.
    #[serde(default = "default_product")]
    pub product: String,
}

/// Canonical JSONL path under the **kit** repo (GSV), not the selected product.
pub fn jsonl_path(repo_root: &Path) -> PathBuf {
    repo_root.join("docs/gsv/fingerprints.jsonl")
}

/// Package version from a product tree (`Cargo.toml` [package], else `package.json`).
pub fn pkg_version(root: &Path) -> Option<String> {
    parse_cargo_version(&root.join("Cargo.toml"))
        .or_else(|| parse_npm_version(&root.join("package.json")))
}

fn parse_cargo_version(toml: &Path) -> Option<String> {
    let text = fs::read_to_string(toml).ok()?;
    let mut in_package = false;
    for line in text.lines() {
        let t = line.trim();
        if t.starts_with('[') {
            in_package = t == "[package]";
            continue;
        }
        if in_package {
            if let Some(rest) = t.strip_prefix("version") {
                let rest = rest.trim().trim_start_matches('=').trim();
                let ver = rest.trim_matches('"').trim_matches('\'').trim();
                if !ver.is_empty() {
                    return Some(ver.to_string());
                }
            }
        }
    }
    None
}

fn parse_npm_version(json_path: &Path) -> Option<String> {
    let text = fs::read_to_string(json_path).ok()?;
    let v: Value = serde_json::from_str(&text).ok()?;
    v.get("version")
        .and_then(Value::as_str)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Append one record (creates the file if missing).
pub fn append(path: &Path, fp: &Fingerprint) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut f = OpenOptions::new().create(true).append(true).open(path)?;
    serde_json::to_writer(&mut f, fp)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    f.write_all(b"\n")?;
    Ok(())
}

/// Last `n` records, newest first. Missing file or bad lines → skip.
pub fn latest(path: &Path, n: usize) -> Vec<Fingerprint> {
    let Ok(f) = fs::File::open(path) else {
        return Vec::new();
    };
    let mut all = Vec::new();
    for line in BufReader::new(f).lines().map_while(Result::ok) {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Ok(fp) = serde_json::from_str::<Fingerprint>(line) {
            all.push(fp);
        }
    }
    if n == 0 || all.is_empty() {
        return Vec::new();
    }
    let start = all.len().saturating_sub(n);
    let mut out: Vec<Fingerprint> = all.drain(start..).collect();
    out.reverse();
    out
}

/// `GET /api/fingerprints?limit=` — default 20, min 1, max 100.
pub fn clamp_limit(raw: Option<usize>) -> usize {
    raw.unwrap_or(20).clamp(1, 100)
}

/// HTTP / card wire. `selected` is the VDT product id (may differ from GSV crate).
pub fn wire(repo_root: &Path, selected: Option<&str>, limit: usize) -> Value {
    let fingerprints = latest(&jsonl_path(repo_root), limit);
    let server_version = crate::gsv_version();
    let selected_version = selected.and_then(|id| {
        let rows = crate::boxes::products::discover(repo_root);
        crate::boxes::products::lookup(&rows, id).and_then(|row| pkg_version(Path::new(&row.path)))
    });
    let cross_product = selected.map(|id| id != "gsv").unwrap_or(false);
    json!({
        "ok": true,
        "path": "docs/gsv/fingerprints.jsonl",
        "server_product": "gsv",
        "server_version": server_version,
        "selected": selected,
        "selected_version": selected_version,
        "cross_product": cross_product,
        "count": fingerprints.len(),
        "fingerprints": fingerprints,
    })
}
