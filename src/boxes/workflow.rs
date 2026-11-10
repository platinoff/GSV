//! Workflow feed stream (#C.5) — surface **open GitHub issues** (agent-facing
//! repos: rust-lang/ACP, agent-teams, ADK-Rust) for the `.agents/skills`
//! proposals pipeline, cached to `data/gsv_workflow.json` so `cargo xtask
//! rules-check` and the Galaxy rust-intel card read it **offline** — no GitHub
//! call in tests/CI. Mirrors the #C.3/#C.4 radar pattern (fail-open carrier).
//!
//! ```text
//! cargo xtask intel            # live workflow sweep (GitHub open issues) → data/gsv_workflow.json
//! cargo xtask intel --dry-run  # canned findings (offline demo / tests)
//! cargo xtask intel --json     # machine-readable report
//! ```

use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Agent-facing repos scanned for open issues (workflow feed).
pub const WORKFLOW_REPOS: &[&str] = &["rust-lang/ACP", "agent-teams", "ADK-Rust"];

/// One workflow finding (an open GitHub issue mapped onto the skills pipeline).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkflowFinding {
    pub source_repo: String,
    pub issue_number: u64,
    pub title: String,
    pub html_url: String,
}

/// Full workflow report. `ok` is always true (fail-open carrier); `source` says
/// what actually happened (github-api | error | none).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkflowReport {
    pub ok: bool,
    /// github-api | error | none
    pub source: String,
    pub generated_at: String,
    #[serde(default)]
    pub findings: Vec<WorkflowFinding>,
    #[serde(default)]
    pub hints: Vec<String>,
}

impl WorkflowReport {
    pub fn new(source: &str, findings: Vec<WorkflowFinding>, hints: Vec<String>) -> Self {
        WorkflowReport {
            ok: true,
            source: source.to_string(),
            generated_at: chrono::Utc::now().to_rfc3339(),
            findings,
            hints,
        }
    }
}

/// Cache path: `data/gsv_workflow.json`.
pub fn path(data_dir: &Path) -> PathBuf {
    data_dir.join("gsv_workflow.json")
}

/// Live workflow sweep: open GitHub issues per repo in [`WORKFLOW_REPOS`].
pub fn collect_live(repo_root: &Path) -> WorkflowReport {
    let _ = repo_root;
    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(_) => {
            return WorkflowReport::new("error", Vec::new(), vec!["runtime: tokio".to_string()]);
        }
    };
    let mut findings: Vec<WorkflowFinding> = Vec::new();
    let hints = Vec::new();
    for repo in WORKFLOW_REPOS {
        let issues = rt.block_on(crate::boxes::github::fetch_issues_live(repo));
        for i in issues {
            findings.push(WorkflowFinding {
                source_repo: repo.to_string(),
                issue_number: i.number,
                title: i.title.clone(),
                html_url: i.html_url.clone(),
            });
        }
    }
    let source = "github-api";
    WorkflowReport::new(source, findings, hints)
}

/// Persist (atomic tmp + rename).
pub fn save(data_dir: &Path, report: &WorkflowReport) -> Result<(), String> {
    std::fs::create_dir_all(data_dir).map_err(|e| format!("create data dir: {e}"))?;
    let raw = serde_json::to_string_pretty(report).map_err(|e| format!("serialize: {e}"))?;
    let target = path(data_dir);
    let tmp = data_dir.join("gsv_workflow.json.tmp");
    let mut f = std::fs::File::create(&tmp).map_err(|e| format!("create tmp: {e}"))?;
    f.write_all(raw.as_bytes())
        .map_err(|e| format!("write: {e}"))?;
    std::fs::rename(&tmp, &target).map_err(|e| format!("rename: {e}"))
}

/// Load the cached report (None when missing / unparsable).
pub fn load(data_dir: &Path) -> Option<WorkflowReport> {
    let raw = std::fs::read_to_string(path(data_dir)).ok()?;
    serde_json::from_str::<WorkflowReport>(&raw).ok()
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
pub const REPORT_FRESH_DAYS: i64 = 7;

pub fn is_fresh(report: &WorkflowReport) -> bool {
    age_days(&report.generated_at).is_some_and(|d| d <= REPORT_FRESH_DAYS)
}

/// Read-only wire merged into the rust-intel card + `gsv_rust_updates` (cached;
/// no GitHub call). One flat shape whether or not a cache exists; `ok` always true.
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
            "hint": "not run — `cargo xtask intel` (github api)",
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("gsv-workflow-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn wire_missing_cache_is_fail_open_with_hint() {
        let d = tmp("wire");
        let w = wire(&d);
        assert_eq!(w["ok"], json!(true));
        assert_eq!(w["source"], json!("none"));
        assert!(w["hint"].as_str().unwrap().contains("cargo xtask intel"));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn save_load_roundtrip_and_freshness() {
        let d = tmp("roundtrip");
        let r = WorkflowReport {
            ok: true,
            source: "github-api".into(),
            generated_at: chrono::Utc::now().to_rfc3339(),
            findings: vec![WorkflowFinding {
                source_repo: "rust-lang/ACP".into(),
                issue_number: 42,
                title: "t".into(),
                html_url: "u".into(),
            }],
            hints: vec![],
        };
        save(&d, &r).unwrap();
        let back = load(&d).unwrap();
        assert_eq!(back, r);
        assert!(is_fresh(&back));
        let old = WorkflowReport {
            generated_at: (chrono::Utc::now() - chrono::Duration::days(9)).to_rfc3339(),
            ..r.clone()
        };
        assert!(!is_fresh(&old));
        let _ = std::fs::remove_dir_all(&d);
    }
}
