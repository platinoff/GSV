//! Dead-code hygiene stream (#C.4) — surface **unused cargo dependencies**
//! (cargo-machete) for the workspace, cached to `data/gsv_hygiene.json` so
//! `cargo xtask rules-check` and the Galaxy rust-intel card read it **offline**
//! — no tool run in tests/CI. Mirrors the #C.3 cargo-deny radar pattern:
//!
//! ```text
//! cargo xtask intel            # live hygiene sweep (cargo machete) → data/gsv_hygiene.json
//! cargo xtask intel --dry-run  # canned findings (offline demo / tests)
//! cargo xtask intel --json     # machine-readable report
//! ```
//!
//! Fail-open carrier like the OSV/deny radars: missing `cargo-machete`, spawn
//! error, or parse trouble yield a `source`-marked report with hints — never a
//! panic. The **HARD** `rules-check` gate is NOT applied here (owner spec:
//! security HARD / freshness+hygiene+workflow SOFT advisory).

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Cached report is considered fresh for this many days (OSV/deny parity).
pub const REPORT_FRESH_DAYS: i64 = 7;

/// One hygiene finding (an apparently-unused dependency crate).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HygieneFinding {
    /// Crate name reported unused by cargo-machete (e.g. "tower-http").
    pub crate_name: String,
    /// Short reason / context (tool version line or raw output tail).
    pub reason: String,
}

/// Full hygiene report. `ok` is always true (fail-open carrier, like the OSV
/// radar); `source` says what actually happened.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HygieneReport {
    pub ok: bool,
    /// machete | not-installed | error | none
    pub source: String,
    pub generated_at: String,
    #[serde(default)]
    pub findings: Vec<HygieneFinding>,
    #[serde(default)]
    pub hints: Vec<String>,
}

impl HygieneReport {
    pub fn new(source: &str, findings: Vec<HygieneFinding>, hints: Vec<String>) -> Self {
        HygieneReport {
            ok: true,
            source: source.to_string(),
            generated_at: crate::vision::rfc3339_now(),
            findings,
            hints,
        }
    }
}

/// Cache path: `data/gsv_hygiene.json`.
pub fn path(data_dir: &Path) -> PathBuf {
    data_dir.join("gsv_hygiene.json")
}

/// Run `cargo machete` in the repo root; parse its stdout into findings.
/// Fail-open on missing tool / spawn error / parse trouble.
pub fn collect_live(repo_root: &Path) -> HygieneReport {
    let out = Command::new("cargo")
        .args(["machete"])
        .current_dir(repo_root)
        .output();
    let out = match out {
        Ok(o) => o,
        Err(e) => {
            let not_installed = e.kind() == std::io::ErrorKind::NotFound;
            return HygieneReport::new(
                if not_installed {
                    "not-installed"
                } else {
                    "error"
                },
                Vec::new(),
                vec![format!("spawn cargo machete: {e}")],
            );
        }
    };
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    let not_installed = stdout.is_empty() && stderr.is_empty() && !out.status.success();
    // cargo-machete prints either "No unused dependencies found!" or one
    // unused crate per line (crate name, optionally with version). Normalize.
    let mut findings: Vec<HygieneFinding> = Vec::new();
    let text = stdout.trim();
    if out.status.success() && !text.is_empty() && !text.contains("No unused") {
        for line in text.lines() {
            let l = line.trim();
            if l.is_empty() || l.contains("No unused") {
                continue;
            }
            let crate_name = l.split_whitespace().next().unwrap_or(l).to_string();
            if crate_name.is_empty() || crate_name == "No" {
                continue;
            }
            findings.push(HygieneFinding {
                crate_name,
                reason: String::new(),
            });
        }
    }
    let source = if not_installed {
        "not-installed"
    } else {
        "machete"
    };
    HygieneReport::new(source, findings, Vec::new())
}

/// Persist (atomic tmp + rename, like the OSV radar).
pub fn save(data_dir: &Path, report: &HygieneReport) -> Result<(), String> {
    fs::create_dir_all(data_dir).map_err(|e| format!("create data dir: {e}"))?;
    let raw = serde_json::to_string_pretty(report).map_err(|e| format!("serialize: {e}"))?;
    let target = path(data_dir);
    let tmp = data_dir.join("gsv_hygiene.json.tmp");
    let mut f = fs::File::create(&tmp).map_err(|e| format!("create tmp: {e}"))?;
    f.write_all(raw.as_bytes())
        .map_err(|e| format!("write: {e}"))?;
    fs::rename(&tmp, &target).map_err(|e| format!("rename: {e}"))
}

/// Load the cached report (None when missing / unparsable).
pub fn load(data_dir: &Path) -> Option<HygieneReport> {
    let raw = fs::read_to_string(path(data_dir)).ok()?;
    serde_json::from_str::<HygieneReport>(&raw).ok()
}

/// Days since `generated_at` (None when unparseable).
pub fn age_days(generated_at: &str) -> Option<i64> {
    let t = chrono::DateTime::parse_from_rfc3339(generated_at).ok()?;
    Some(
        chrono::Utc::now()
            .signed_duration_since(t.with_timezone(&chrono::Utc))
            .num_days(),
    )
}

/// Whether the cached report is within [`REPORT_FRESH_DAYS`].
pub fn is_fresh(report: &HygieneReport) -> bool {
    age_days(&report.generated_at).is_some_and(|d| d <= REPORT_FRESH_DAYS)
}

/// Read-only wire merged into the rust-intel card + `gsv_rust_updates` (cached;
/// no cargo run). One flat shape whether or not a cache exists; `ok` always true.
pub fn wire(data_dir: &Path) -> Value {
    match load(data_dir) {
        Some(r) => json!({
            "ok": true,
            "fresh": is_fresh(&r),
            "source": r.source,
            "generated_at": r.generated_at,
            "findings": r.findings,
            "hints": r.hints,
        }),
        None => json!({
            "ok": true,
            "fresh": false,
            "source": "none",
            "generated_at": String::new(),
            "findings": [],
            "hints": [],
            "hint": "not run — `cargo xtask intel` (needs cargo-machete installed)",
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("gsv-hygiene-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn collect_in_empty_tmp_dir_is_fail_open() {
        // An empty repo dir -> cargo machete exits cleanly with no findings;
        // the carrier must stay ok / fail-open regardless of tool behaviour.
        let d = tmp("collect");
        let r = collect_live(&d);
        assert!(r.ok);
        assert_eq!(r.source, "machete");
        assert!(r.findings.is_empty());
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn wire_missing_cache_is_fail_open_with_hint() {
        let d = tmp("wire");
        let w = wire(&d);
        assert_eq!(w["ok"], json!(true));
        assert_eq!(w["source"], json!("none"));
        assert!(w["hint"].as_str().unwrap().contains("cargo xtask intel"));
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn save_load_roundtrip_and_freshness() {
        let d = tmp("roundtrip");
        let r = HygieneReport {
            ok: true,
            source: "machete".into(),
            generated_at: chrono::Utc::now().to_rfc3339(),
            findings: vec![HygieneFinding {
                crate_name: "tower-http".into(),
                reason: String::new(),
            }],
            hints: vec![],
        };
        save(&d, &r).unwrap();
        let back = load(&d).unwrap();
        assert_eq!(back, r);
        assert!(is_fresh(&back));
        let old = HygieneReport {
            generated_at: (chrono::Utc::now() - chrono::Duration::days(9)).to_rfc3339(),
            ..r.clone()
        };
        assert!(!is_fresh(&old));
        let _ = fs::remove_dir_all(&d);
    }
}
