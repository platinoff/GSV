//! fm-xlink: advisory cross-link between GSV tickets and poolAI FM ids.
//!
//! Read-only. GSV never stores FM prose — only ids. `P4` protocol federation:
//! poolAI FM remains the management-plane (тато) canon; this box just tells
//! the operator which `fm_ref` links on the board do not resolve, so FM and
//! tickets do not drift apart silently. Report-only by default; `--strict`
//! fails the command on orphans.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

const FM_PREFIXES: [&str; 3] = ["FM-", "PH-S", "PH-SVC"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FmLinkRow {
    pub ticket_id: String,
    pub fm_ref: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OrphanRow {
    pub ticket_id: String,
    pub fm_ref: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FmXlinkReport {
    pub ok: bool,
    pub at: String,
    pub fm_path: String,
    pub fm_found: bool,
    pub tickets_total: usize,
    pub linked: Vec<FmLinkRow>,
    pub orphans: Vec<OrphanRow>,
}

/// Valid FM id token shape: `FM-###` (3 digits), `PH-S<digits>` (3–6 chars,
/// digits only), `PH-SVC<digits>` (2–4 digits). Rejects `FM-FM-FM-…`-class
/// garbage produced by naive prefix scans.
fn is_fm_token(tok: &str) -> bool {
    if let Some(rest) = tok.strip_prefix("FM-") {
        return rest.len() == 3 && rest.chars().all(|c| c.is_ascii_digit());
    }
    if let Some(rest) = tok.strip_prefix("PH-SVC") {
        let n = rest.len();
        return (2..=4).contains(&n) && rest.chars().all(|c| c.is_ascii_digit());
    }
    if let Some(rest) = tok.strip_prefix("PH-S") {
        let n = rest.len();
        return (3..=6).contains(&n) && rest.chars().all(|c| c.is_ascii_digit());
    }
    false
}

/// All FM-ish id tokens in an FM source (`FM-001`, `PH-S156`, `PH-SVC47`).
pub fn fm_ids_from_text(text: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for needle in FM_PREFIXES {
        let mut search = text;
        while let Some(i) = search.find(needle) {
            let cand = &search[i..];
            let tok: String = cand
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '-')
                .collect();
            if is_fm_token(&tok) {
                out.insert(tok);
            }
            search = &search[i + needle.len()..];
        }
    }
    out
}

fn rfc3339_now() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

/// Collect the cross-link report (read-only; never writes).
pub fn collect(repo_root: &Path, fm_path: &Path) -> FmXlinkReport {
    let fm_path_s = fm_path.display().to_string();
    let tick_path = crate::boxes::tickets::tickets_path(repo_root);
    let mut report = FmXlinkReport {
        ok: true,
        at: rfc3339_now(),
        fm_path: fm_path_s,
        fm_found: fm_path.is_file(),
        tickets_total: 0,
        linked: Vec::new(),
        orphans: Vec::new(),
    };

    // FM index (skip silently when missing — advisory, not a hard gate).
    let fm_index = match fs::read_to_string(fm_path) {
        Ok(text) => fm_ids_from_text(&text),
        Err(_) => BTreeSet::new(),
    };

    let Ok(bytes) = fs::read(&tick_path) else {
        report.ok = false;
        return report;
    };
    let Ok(text) = String::from_utf8(bytes) else {
        report.ok = false;
        return report;
    };

    for (line_no, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
            eprintln!(
                "fm-xlink: skip unparseable tickets.jsonl line {}",
                line_no + 1
            );
            continue;
        };
        let id = v
            .get("id")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("<missing>")
            .to_string();
        let fm_ref = v
            .get("fm_ref")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .trim()
            .to_string();
        let status = v
            .get("status")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown")
            .to_string();
        if fm_ref.is_empty() {
            continue;
        }
        report.tickets_total += 1;
        if fm_index.contains(&fm_ref) {
            report.linked.push(FmLinkRow {
                ticket_id: id,
                fm_ref,
                status,
            });
        } else {
            report.orphans.push(OrphanRow {
                ticket_id: id,
                fm_ref,
                status,
            });
        }
    }

    report
}

/// Human-readable render (gate output is captured as text).
pub fn render(r: &FmXlinkReport) -> String {
    let mut out = String::from("fm-xlink (GSV tickets ↔ poolAI FM) — advisory\n");
    out.push_str(&format!("- at: {}\n", r.at));
    out.push_str(&format!(
        "- poolAI FM: {} ({})\n",
        r.fm_path,
        if r.fm_found { "ok" } else { "MISSING" }
    ));
    out.push_str(&format!("- tickets with fm_ref: {}\n", r.tickets_total));
    out.push_str(&format!("- linked: {}\n", r.linked.len()));
    for row in &r.linked {
        out.push_str(&format!(
            "  ✓ {:<24} → {} ({})\n",
            row.ticket_id, row.fm_ref, row.status
        ));
    }
    out.push_str(&format!("- orphans: {}\n", r.orphans.len()));
    for row in &r.orphans {
        out.push_str(&format!(
            "  ✗ {:<24} → {} ({}) — not found in poolAI FM\n",
            row.ticket_id, row.fm_ref, row.status
        ));
    }
    if r.orphans.is_empty() {
        out.push_str("- verdict: linked orphans zero — no drift declared\n");
    }
    out
}

/// Default FM path relative to the workspace parent (the kit's convention:
/// a sibling `poolAI/docs/catalog/FUNCTION_MANAGEMENT.md`).
pub fn default_fm_path(repo_root: &Path) -> PathBuf {
    if let Some(parent) = repo_root.parent() {
        return parent
            .join("poolAI")
            .join("docs")
            .join("catalog")
            .join("FUNCTION_MANAGEMENT.md");
    }
    PathBuf::from("S:/rust/poolAI/docs/catalog/FUNCTION_MANAGEMENT.md")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fm_ids_extracts_numbered_tokens() {
        let text = "| FM-001 | ...\n| PH-S1279 | ...\n| **PH-SVC47** | ...\n| no ids |\n";
        let set = fm_ids_from_text(text);
        assert!(set.contains("FM-001"));
        assert!(set.contains("PH-S1279"));
        assert!(set.contains("PH-SVC47"));
        assert_eq!(set.len(), 3);
    }

    #[test]
    fn fm_ids_ignores_far_too_short_tokens() {
        let set = fm_ids_from_text("FM-FM-FM-FM-\tph-\n");
        assert!(set.is_empty());
    }

    #[test]
    fn collect_reports_linked_and_orphan_against_fixture() {
        let dir = std::env::temp_dir().join(format!("gsv-xlink-{}", std::process::id()));
        let _ = std::fs::create_dir_all(dir.join("docs/gsv"));
        let tick = dir.join("docs/gsv/tickets.jsonl");
        std::fs::write(
            &tick,
            concat!(
                "{\"id\":\"t-1\",\"fm_ref\":\"FM-001\",\"status\":\"done\"}\n",
                "{\"id\":\"t-2\",\"fm_ref\":\"PH-S1279\",\"status\":\"open\"}\n",
                "{\"id\":\"t-3\",\"fm_ref\":\"FM-999\",\"status\":\"open\"}\n",
                "{\"id\":\"t-4\",\"status\":\"open\"}\n",
            ),
        )
        .unwrap();
        let fm = dir.join("FM.md");
        std::fs::write(&fm, "| FM-001 | x\n| PH-S1279 | y\n").unwrap();
        let r = collect(&dir, &fm);
        assert!(r.fm_found);
        assert_eq!(r.tickets_total, 3);
        assert_eq!(r.linked.len(), 2);
        assert_eq!(r.orphans.len(), 1);
        assert_eq!(r.orphans[0].fm_ref, "FM-999");
    }
}
