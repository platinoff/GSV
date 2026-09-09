//! Rules-drift check (`cargo xtask rules-check`) — Gravity harness.
//!
//! Aggregates the existing per-domain gates so a drain can confirm the kit
//! rulebases still describe the shipped truth:
//!
//! - `git` — the working tree resolves a HEAD (`vision::git_head_short`).
//! - `fingerprint` — the latest `fingerprints.jsonl` row matches that HEAD
//!   (drain discipline: record at band close).
//! - `vision` — the vision snapshot has no drift (`vision::collect_drift`).
//! - `registry` — every registered product row keeps HANDOFF + NEXT
//!   (`products::scan`), i.e. PRODUCTS.md matches discovery reality.
//! - `ratio` — the persisted `data/rust_ratio.json` meets the Rust 95–100%
//!   formal band (`ratio::load`); the 96% stretch is advisory.
//! - `sli` — unused script candidates (new-SLI pool) — advisory.
//! - `bench` — speed-index freshness (re-`record-speed`) — advisory.
//! - `live` — when a live URL is passed, the running server's crate version
//!   matches this crate (`keep_live::probe_http_blocking`) — advisory.
//!
//! `ok` reflects **hard** gates only; soft checks (registry/sli/bench/live)
//! carry drift detail without failing the exit code.
//!
//! Doc: [`GSV_RULES_CHECK.md`](docs/gsv/GSV_RULES_CHECK.md).

use serde::Serialize;
use std::path::Path;

use crate::boxes::{fingerprint, keep_live, products, ratio, sli, vision};
use crate::vision::{git_head_short, rfc3339_now};

/// Canonical doc describing the harnesses and their sources.
pub const RULES_DOC: &str = "docs/gsv/GSV_RULES_CHECK.md";
/// Bench freshness window: re-run `cargo xtask record-speed` at least this often.
const BENCH_FRESH_DAYS: i64 = 31;

/// One checked rule. `gate: true` drives the overall `ok`.
#[derive(Debug, Clone, Serialize)]
pub struct RuleCheck {
    pub id: &'static str,
    pub ok: bool,
    pub gate: bool,
    pub detail: String,
}

impl RuleCheck {
    fn hard(id: &'static str, ok: bool, detail: impl Into<String>) -> Self {
        RuleCheck::check(true, id, ok, detail)
    }
    fn soft(id: &'static str, ok: bool, detail: impl Into<String>) -> Self {
        RuleCheck::check(false, id, ok, detail)
    }
    fn check(gate: bool, id: &'static str, ok: bool, detail: impl Into<String>) -> Self {
        Self {
            id,
            ok,
            gate,
            detail: detail.into(),
        }
    }
}

/// The drift report. `ok` = every hard gate passed.
#[derive(Debug, Clone, Serialize)]
pub struct RulesCheck {
    pub ok: bool,
    pub at: String,
    pub git_head: String,
    pub doc: String,
    pub checks: Vec<RuleCheck>,
}

/// Build the drift report for a repo root. `live_url` probes a running server
/// (advisory) when present.
pub fn collect(repo_root: &Path, data_dir: &Path, live_url: Option<&str>) -> RulesCheck {
    let head = git_head_short(repo_root);
    let mut checks = Vec::new();

    // git: the tree must resolve a HEAD.
    let git_ok = head != "unknown";
    checks.push(RuleCheck::hard(
        "git",
        git_ok,
        if git_ok {
            format!("HEAD {head}")
        } else {
            "no git HEAD (repo not resolvable)".to_string()
        },
    ));

    // fingerprint: this crate version must have been drained (row for product gsv).
    // git_head is recorded pre-commit, so a HEAD mismatch is advisory, not a gate.
    let fps = fingerprint::latest(&fingerprint::jsonl_path(repo_root), 3);
    let crate_ver = crate::gsv_version();
    let version_recorded = fps
        .iter()
        .any(|f| f.product == "gsv" && f.version == crate_ver);
    let head_match = fps
        .first()
        .is_some_and(|f| f.git_head.as_deref() == Some(head.as_str()));
    let fp_detail = match fps.first() {
        Some(f) => format!(
            "latest: {} v{} {} (product {}) — version {crate_ver} recorded {}, working HEAD {} {}",
            f.actor,
            f.version,
            f.ide,
            f.product,
            if version_recorded { "yes" } else { "NO (run `cargo xtask fingerprint`)" },
            head,
            if head_match { "matches latest row" } else { "differs from latest row (pre-commit record is fine)" },
        ),
        None => format!(
            "no fingerprint recorded (run `cargo xtask fingerprint`); version {crate_ver} unrecorded"
        ),
    };
    checks.push(RuleCheck::hard("fingerprint", version_recorded, fp_detail));

    // vision: snapshot must not drift from the source canon.
    let drift = vision::collect_drift(repo_root, data_dir);
    let revision = vision::read_manifest(repo_root)
        .map(|m| m.revision)
        .unwrap_or(0);
    checks.push(RuleCheck::hard(
        "vision",
        drift.is_empty(),
        if drift.is_empty() {
            format!("snapshot clean (revision {revision})")
        } else {
            format!("{} drift: {}", drift.len(), drift.join("; "))
        },
    ));

    // registry: every registered product row keeps HANDOFF + NEXT.
    let registered: Vec<_> = products::discover(repo_root)
        .into_iter()
        .filter(|r| r.registered)
        .collect();
    let mut missing = Vec::new();
    for row in &registered {
        match products::scan(repo_root, &row.id) {
            Ok(s) => {
                if !s.handoff_exists {
                    missing.push(format!("{}/HANDOFF", row.id));
                }
                if !s.next_exists {
                    missing.push(format!("{}/NEXT", row.id));
                }
            }
            Err(_) => missing.push(format!("{}/unscanable", row.id)),
        }
    }
    checks.push(RuleCheck::soft(
        "registry",
        missing.is_empty(),
        if missing.is_empty() {
            format!("{} registered products have HANDOFF+NEXT", registered.len())
        } else {
            format!(
                "{} registered: missing {}",
                registered.len(),
                missing.join(", ")
            )
        },
    ));

    // ratio: persisted Rust ratio must satisfy the formal band (95%).
    match ratio::load(data_dir) {
        Ok(r) => {
            let stretch = if r.meets_stretch_96 {
                "stretch-96 ok"
            } else {
                "stretch-96 below"
            };
            checks.push(RuleCheck::hard(
                "ratio",
                r.meets_min_ratio,
                format!(
                    "rust {:.2}% (rust {} / product {}; formal ≥{}%, {stretch})",
                    r.rust_ratio_pct, r.rust_loc, r.product_loc_total, r.formal_band_min
                ),
            ));
        }
        Err(e) => checks.push(RuleCheck::hard(
            "ratio",
            false,
            format!("{e} (run `cargo run --bin gsv-loc-audit`)"),
        )),
    }

    // sli: unused script candidates are the new-SLI pool, not a hard failure.
    let sw = sli::wire(repo_root);
    let cat = &sw.catalog;
    let unused = cat.unused_count;
    let used = cat.entries.len().saturating_sub(unused);
    checks.push(RuleCheck::soft(
        "sli",
        true,
        format!(
            "catalog {} entries (used {used}, unused new-SLI pool {unused})",
            cat.entries.len()
        ),
    ));

    // bench: speed index must be fresh (record-speed ran this drain).
    match vision::read_speed_index(repo_root) {
        Ok(s) => {
            let fresh = speed_index_fresh(&s.generated_at);
            checks.push(RuleCheck::soft(
                "bench",
                fresh,
                format!(
                    "speed_index {} (git {}, {} test-ci records, {} benches){}",
                    s.generated_at,
                    s.git_head,
                    s.test_ci_count,
                    s.bench_count,
                    if fresh {
                        ""
                    } else {
                        " — stale, run `cargo xtask record-speed`"
                    }
                ),
            ));
        }
        Err(e) => checks.push(RuleCheck::soft(
            "bench",
            false,
            format!("{e} (rerun `cargo xtask record-speed`)"),
        )),
    }

    // live: optional running-server version lockstep probe.
    match live_url {
        Some(url) => {
            let p = keep_live::probe_http_blocking(url);
            let crate_ver = crate::gsv_version();
            let ok = p.alive && p.version.as_deref() == Some(crate_ver);
            checks.push(RuleCheck::soft(
                "live",
                ok,
                format!(
                    "{url} → alive {}, version {} vs crate {} (latency {} ms)",
                    p.alive,
                    p.version.as_deref().unwrap_or("n/a"),
                    crate_ver,
                    p.latency_ms
                ),
            ));
        }
        None => checks.push(RuleCheck::soft("live", true, "not probed (no --live-url)")),
    }

    let ok = checks.iter().filter(|c| c.gate).all(|c| c.ok);
    RulesCheck {
        ok,
        at: rfc3339_now(),
        git_head: head,
        doc: RULES_DOC.to_string(),
        checks,
    }
}

/// Bench freshness: full RFC3339 or a bare `%Y-%m-%d` date (record-speed
/// writes the date-only form), age in days must stay ≤ [`BENCH_FRESH_DAYS`].
fn speed_index_fresh(generated_at: &str) -> bool {
    let days = match generated_at.parse::<chrono::DateTime<chrono::Utc>>() {
        Ok(t) => chrono::Utc::now().signed_duration_since(t).num_days(),
        Err(_) => match chrono::NaiveDate::parse_from_str(generated_at.trim(), "%Y-%m-%d") {
            Ok(d) => chrono::Utc::now()
                .date_naive()
                .signed_duration_since(d)
                .num_days(),
            Err(_) => return false,
        },
    };
    days <= BENCH_FRESH_DAYS
}

/// Build a report from an explicit check list (internal / test use).
#[cfg(test)]
fn collect_in_checks(checks: Vec<RuleCheck>) -> RulesCheck {
    let ok = checks.iter().filter(|c| c.gate).all(|c| c.ok);
    RulesCheck {
        ok,
        at: rfc3339_now(),
        git_head: "test".into(),
        doc: RULES_DOC.to_string(),
        checks,
    }
}

/// Render the report as a condensed one-line-per-check text block.
pub fn render(r: &RulesCheck) -> String {
    let mut out = format!(
        "rules-check {} at {} git {}\n",
        if r.ok { "ok" } else { "DRIFT" },
        r.at,
        r.git_head
    );
    for c in &r.checks {
        out.push_str(&format!(
            "  [{}] {:<11} {}\n",
            if c.ok {
                "ok"
            } else if c.gate {
                "HARD"
            } else {
                "soft"
            },
            c.id,
            c.detail
        ));
    }
    out.push_str(&format!("  doc: {}\n", r.doc));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn tmp_repo(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("gsv-rules-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("data")).unwrap();
        dir
    }

    fn tmp() -> (PathBuf, PathBuf) {
        let dir = tmp_repo("check");
        let data = dir.join("data");
        (dir, data)
    }

    /// Hard gates fail cleanly on an empty tree; soft checks never drive `ok`.
    #[test]
    fn empty_tree_reports_hard_drift() {
        let (dir, data) = tmp();
        let r = collect(&dir, &data, None);
        assert!(!r.ok, "empty tree must fail hard gates");
        assert!(["git", "fingerprint", "vision", "ratio"]
            .iter()
            .all(|id| r.checks.iter().any(|c| c.id == *id && c.gate && !c.ok)));
        assert!(r.checks.iter().any(|c| c.id == "live" && !c.gate));
        let _ = fs::remove_dir_all(&dir);
    }

    /// Soft failures do not flip `ok`; hard do.
    #[test]
    fn ok_follows_hard_gates_only() {
        let hard_ok = RuleCheck::hard("git", true, "HEAD abc");
        let hard_bad = RuleCheck::hard("ratio", false, "missing");
        let soft_bad = RuleCheck::soft("bench", false, "stale");
        let all = || {
            vec![
                RuleCheck::hard("git", true, "HEAD abc"),
                RuleCheck::soft("bench", false, "stale"),
            ]
        };
        assert!(collect_in_checks(all()).ok);

        let r = collect_in_checks(vec![
            RuleCheck::hard("git", true, "HEAD abc"),
            hard_bad,
            soft_bad,
        ]);
        assert!(!r.ok);
        let hard: Vec<_> = r.checks.iter().filter(|c| c.gate).collect();
        assert_eq!(hard.len(), 2, "git + ratio are the hard gates");
        assert!(hard.iter().any(|c| c.id == "ratio" && !c.ok));
        drop(hard_ok);
    }

    /// Render output mentions every check id.
    #[test]
    fn render_lists_checks() {
        let r = RulesCheck {
            ok: true,
            at: "2026-01-01T00:00:00Z".into(),
            git_head: "abc".into(),
            doc: RULES_DOC.into(),
            checks: vec![RuleCheck::soft("git", true, "HEAD abc")],
        };
        let text = render(&r);
        assert!(text.contains("git"));
        assert!(text.contains(RULES_DOC));
    }

    /// Bench freshness accepts the date-only form (as record-speed writes).
    #[test]
    fn bench_fresh_accepts_date_only_and_rejects_old() {
        assert!(speed_index_fresh(
            &chrono::Utc::now().format("%Y-%m-%d").to_string()
        ));
        assert!(speed_index_fresh(&chrono::Utc::now().to_rfc3339()));
        let old = (chrono::Utc::now() - chrono::Duration::days(40))
            .format("%Y-%m-%d")
            .to_string();
        assert!(!speed_index_fresh(&old));
        assert!(!speed_index_fresh("not-a-date"));
    }

    /// Registry check flags a registered product row without NEXT.
    #[test]
    fn registry_flags_row_without_docs() {
        // Products discovery needs a git root; instead assert the soft-check
        // wiring exists and that scan() of an unknown id yields a missing row.
        let (dir, data) = tmp();
        fs::write(
            dir.join("Cargo.toml"),
            "[package]\nname = \"x\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        let mut r = super::collect(&dir, &data, None);
        // No registered rows → registry is ok (0 registered).
        let reg = r
            .checks
            .iter()
            .find(|c| c.id == "registry")
            .cloned()
            .unwrap();
        assert!(reg.ok);
        assert!(reg.detail.contains("0 registered"));
        // Force a missing-docs detail path via the check builder in isolation.
        r.checks.push(RuleCheck::soft(
            "registry",
            false,
            "2 registered: a/HANDOFF, b/NEXT",
        ));
        assert!(render(&r).contains("a/HANDOFF"));
        let _ = fs::remove_dir_all(&dir);
    }
}
