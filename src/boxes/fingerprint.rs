//! Drain fingerprint JSONL — who / which client / which model / when.
//!
//! Path: `{repo}/docs/gsv/fingerprints.jsonl` (git-tracked, never `data/`).

use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

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
}

/// Canonical JSONL path under the product repo.
pub fn jsonl_path(repo_root: &Path) -> PathBuf {
    repo_root.join("docs/gsv/fingerprints.jsonl")
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

/// HTTP / card wire.
pub fn wire(repo_root: &Path, limit: usize) -> Value {
    let fingerprints = latest(&jsonl_path(repo_root), limit);
    json!({
        "ok": true,
        "path": "docs/gsv/fingerprints.jsonl",
        "count": fingerprints.len(),
        "fingerprints": fingerprints,
    })
}
