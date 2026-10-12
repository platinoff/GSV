//! RustSec/OSV radar — surface crate security advisories for the dependencies in
//! `Cargo.lock`, cached to `data/gsv_rustsec.json` so `cargo xtask rules-check`
//! and (in #A.2) the Galaxy card read it **offline** — no network in tests/CI.
//!
//! Zero-install: the OSV.dev `querybatch` endpoint (one HTTPS POST per ≤128 deps)
//! instead of shelling out to `cargo-audit`. The live fetch runs **only** from
//! `cargo xtask intel` (owner / cadence tick); everything else is fail-open on the
//! cached report.
//!
//! ```text
//! cargo xtask intel            # live OSV sweep → data/gsv_rustsec.json
//! cargo xtask intel --dry-run  # canned advisories (offline demo / tests)
//! cargo xtask intel --json     # machine-readable report
//! ```
//!
//! Severity is taken from an OSV record's `database_specific.severity` when the
//! upstream advisory carries it, else `unknown`. The **HARD** `rules-check` gate
//! (fail the drain on CRITICAL/HIGH) is #A.2, once CVSS base-score bucketing is
//! reliable; #A keeps the signal **soft** so an offline or noisy-lock run never
//! blocks the drain.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// One advisory surfaced from an OSV record.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Advisory {
    pub id: String,
    pub package: String,
    pub version: String,
    /// critical | high | moderate | low | unknown
    pub severity: String,
    pub summary: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub url: String,
}

/// Severity bucket counts.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct IntelCounts {
    pub total: usize,
    pub critical: usize,
    pub high: usize,
    pub moderate: usize,
    pub low: usize,
    pub unknown: usize,
}

/// Full radar report. `ok` is always true (fail-open, like keep-live `disk_ok`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IntelReport {
    pub ok: bool,
    /// dry-run | osv | unreachable | no-lock | none
    pub source: String,
    pub generated_at: String,
    pub deps_total: usize,
    pub queries_attempted: usize,
    pub counts: IntelCounts,
    pub advisories: Vec<Advisory>,
    #[serde(default)]
    pub hints: Vec<String>,
}

const OSV_BATCH_URL: &str = "https://api.osv.dev/v1/querybatch";
const OSV_VULN_URL: &str = "https://api.osv.dev/v1/vulns/";
const OSV_ECOSYSTEM: &str = "crates.io";
/// Deps per querybatch call (OSV accepts a few hundred; stay modest).
const BATCH: usize = 128;
/// Max per-advisory detail fetches to resolve a missing severity (bounds the
/// blast radius; `querybatch` omits the `severity` CVSS vector that `/v1/vulns`
/// carries, and most scans surface only a handful).
const ENRICH_CAP: usize = 40;
/// Cached report is considered fresh for this many days.
pub const REPORT_FRESH_DAYS: i64 = 7;

/// Parse `[[package]] name/version` pairs from `Cargo.lock` (line scan, robust to
/// lock-format drift; a crate may repeat with different versions).
pub fn parse_lock_deps(repo_root: &Path) -> Vec<(String, String)> {
    let Ok(raw) = fs::read_to_string(repo_root.join("Cargo.lock")) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut name: Option<String> = None;
    let mut version: Option<String> = None;
    let mut saw_package = false;
    for line in raw.lines() {
        let l = line.trim();
        if l == "[[package]]" {
            if let (Some(n), Some(v)) = (name.take(), version.take()) {
                if saw_package {
                    out.push((n, v));
                }
            }
            saw_package = true;
        } else if let Some(rest) = l.strip_prefix("name = ") {
            name = unquote(rest);
        } else if let Some(rest) = l.strip_prefix("version = ") {
            version = unquote(rest);
        }
    }
    if let (Some(n), Some(v)) = (name.take(), version.take()) {
        if saw_package {
            out.push((n, v));
        }
    }
    out
}

fn unquote(s: &str) -> Option<String> {
    let s = s.trim();
    s.strip_prefix('"')
        .and_then(|x| x.strip_suffix('"'))
        .map(|x| x.to_string())
}

/// OSV `querybatch` request body for a slice of deps.
pub fn build_batch_body(deps: &[(String, String)]) -> Value {
    let queries: Vec<Value> = deps
        .iter()
        .map(|(name, version)| {
            json!({
                "package": { "name": name, "ecosystem": OSV_ECOSYSTEM },
                "version": version
            })
        })
        .collect();
    json!({ "queries": queries })
}

/// Map a CVSS:3.1 base score to a qualitative band (OSV/RustSec thresholds):
/// <4 low, <7 moderate, <9 high, else critical.
fn bucket_from_score(score: f64) -> &'static str {
    if score < 4.0 {
        "low"
    } else if score < 7.0 {
        "moderate"
    } else if score < 9.0 {
        "high"
    } else {
        "critical"
    }
}

/// CVSS v3.1 base-score from a vector string (`CVSS:3.1/AV:N/AC:L/...`). Returns
/// `None` when the vector is missing metrics or unparsable (fail-open → unknown).
/// Metric weights per the FIRST.org CVSS v3.1 spec; roundup to 1 decimal.
pub fn cvss_v3_base_score(vector: &str) -> Option<f64> {
    let mut av = f64::NAN;
    let mut ac = f64::NAN;
    let mut pr = f64::NAN;
    let mut ui = f64::NAN;
    let mut scope_changed = false;
    let mut c = f64::NAN;
    let mut i = f64::NAN;
    let mut a = f64::NAN;
    let metrics = vector.split('/').skip_while(|p| p.starts_with("CVSS:"));
    for tok in metrics {
        let mut kv = tok.split(':');
        let (k, v) = (kv.next()?, kv.next()?);
        match k {
            "AV" => {
                av = match v {
                    "N" => 0.85,
                    "A" => 0.62,
                    "L" => 0.55,
                    "P" => 0.20,
                    _ => return None,
                }
            }
            "AC" => {
                ac = match v {
                    "L" => 0.77,
                    "H" => 0.44,
                    _ => return None,
                }
            }
            "UI" => {
                ui = match v {
                    "N" => 0.85,
                    "R" => 0.62,
                    _ => return None,
                }
            }
            "S" => scope_changed = v == "C",
            "C" => {
                c = match v {
                    "H" => 0.56,
                    "L" => 0.22,
                    "N" => 0.0,
                    _ => return None,
                }
            }
            "I" => {
                i = match v {
                    "H" => 0.56,
                    "L" => 0.22,
                    "N" => 0.0,
                    _ => return None,
                }
            }
            "A" => {
                a = match v {
                    "H" => 0.56,
                    "L" => 0.22,
                    "N" => 0.0,
                    _ => return None,
                }
            }
            "PR" => {
                pr = match (v, scope_changed) {
                    ("N", _) => 0.85,
                    ("L", false) => 0.62,
                    ("L", true) => 0.68,
                    ("H", false) => 0.27,
                    ("H", true) => 0.50,
                    _ => return None,
                }
            }
            _ => {}
        }
    }
    if [av, ac, pr, ui, c, i, a].iter().any(|x| x.is_nan()) {
        return None;
    }
    let iss = 1.0 - ((1.0 - c) * (1.0 - i) * (1.0 - a));
    let impact = if scope_changed {
        (7.52 * (iss - 0.029) - 3.25 * (iss - 0.029).powi(15)).max(0.0)
    } else {
        6.42 * iss
    };
    let exploit = 8.22 * av * ac * pr * ui;
    Some(roundup((impact + exploit).min(10.0)))
}

/// CVSS "roundup": smallest number, to one decimal, >= x (first.org Roundup).
fn roundup(x: f64) -> f64 {
    (x * 10.0).ceil() / 10.0
}

/// Severity for an OSV record: `database_specific.severity` when present (GHSA /
/// many crates.io entries carry it), else the max CVSS v3 base score across the
/// `severity[]` vectors bucketed, else `unknown`.
fn severity_from_vuln(v: &Value) -> String {
    let raw = v
        .pointer("/database_specific/severity")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_ascii_uppercase();
    match raw.as_str() {
        "CRITICAL" => return "critical".into(),
        "HIGH" => return "high".into(),
        "MODERATE" | "MEDIUM" => return "moderate".into(),
        "LOW" => return "low".into(),
        _ => {}
    }
    if let Some(arr) = v.get("severity").and_then(Value::as_array) {
        let mut best: Option<f64> = None;
        for s in arr {
            if let Some(score_str) = s.get("score").and_then(Value::as_str) {
                if let Some(sc) = cvss_v3_base_score(score_str) {
                    best = Some(best.map_or(sc, |b| b.max(sc)));
                }
            }
        }
        if let Some(sc) = best {
            return bucket_from_score(sc).to_string();
        }
    }
    "unknown".to_string()
}

fn bump(counts: &mut IntelCounts, sev: &str) {
    counts.total += 1;
    match sev {
        "critical" => counts.critical += 1,
        "high" => counts.high += 1,
        "moderate" => counts.moderate += 1,
        "low" => counts.low += 1,
        _ => counts.unknown += 1,
    }
}

/// Map an OSV querybatch response onto advisories, aligned by index to `deps`.
pub fn parse_osv(body: &Value, deps: &[(String, String)]) -> Vec<Advisory> {
    let mut out = Vec::new();
    let results = match body.get("results").and_then(Value::as_array) {
        Some(r) => r,
        None => return out,
    };
    for (i, res) in results.iter().enumerate() {
        let Some((pkg, ver)) = deps.get(i) else {
            continue;
        };
        let Some(vulns) = res.get("vulns").and_then(Value::as_array) else {
            continue;
        };
        for v in vulns {
            let id = v.get("id").and_then(Value::as_str).unwrap_or("");
            if id.is_empty() {
                continue;
            }
            let aliases = v
                .get("aliases")
                .and_then(Value::as_array)
                .map(|a| {
                    a.iter()
                        .filter_map(|x| x.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();
            let url = v
                .get("references")
                .and_then(Value::as_array)
                .and_then(|r| {
                    r.iter()
                        .find_map(|x| x.get("url").and_then(Value::as_str).map(|s| s.to_string()))
                })
                .unwrap_or_default();
            out.push(Advisory {
                id: id.to_string(),
                package: pkg.clone(),
                version: ver.clone(),
                severity: severity_from_vuln(v),
                summary: v
                    .get("summary")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                aliases,
                url,
            });
        }
    }
    out
}

/// Build a report from gathered advisories (computes severity counts).
pub fn finalize(
    deps_total: usize,
    queries_attempted: usize,
    advisories: Vec<Advisory>,
    source: &str,
    hints: Vec<String>,
) -> IntelReport {
    let mut counts = IntelCounts::default();
    for a in &advisories {
        bump(&mut counts, &a.severity);
    }
    IntelReport {
        ok: true,
        source: source.to_string(),
        generated_at: crate::vision::rfc3339_now(),
        deps_total,
        queries_attempted,
        counts,
        advisories,
        hints,
    }
}

/// Canned dry-run report (2 advisories) so the card / rules-check have a stable
/// offline fixture and `--dry-run` demos without network.
pub fn collect_dry() -> IntelReport {
    let advisories = vec![
        Advisory {
            id: "RUSTSEC-2023-0000".into(),
            package: "example-dep".into(),
            version: "1.0.0".into(),
            severity: "high".into(),
            summary: "dry-run: sample high advisory".into(),
            aliases: vec!["GHSA-xxxx-yyyy-zzzz".into()],
            url: "https://osv.dev/vulnerability/RUSTSEC-2023-0000".into(),
        },
        Advisory {
            id: "RUSTSEC-2022-0000".into(),
            package: "another-dep".into(),
            version: "0.5.1".into(),
            severity: "moderate".into(),
            summary: "dry-run: sample moderate advisory".into(),
            aliases: vec![],
            url: "https://osv.dev/vulnerability/RUSTSEC-2022-0000".into(),
        },
    ];
    finalize(
        advisories.len(),
        advisories.len(),
        advisories,
        "dry-run",
        vec![],
    )
}

/// Live OSV sweep over `Cargo.lock`. Never panics / never blocks the drain: any
/// network or shape failure yields an `unreachable` report with hints.
pub async fn collect_live(repo_root: &Path) -> IntelReport {
    let deps = parse_lock_deps(repo_root);
    if deps.is_empty() {
        return finalize(
            0,
            0,
            Vec::new(),
            "no-lock",
            vec!["Cargo.lock has no packages".into()],
        );
    }
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return finalize(
                deps.len(),
                0,
                Vec::new(),
                "unreachable",
                vec![format!("client: {e}")],
            )
        }
    };
    let mut advisories = Vec::new();
    let mut attempted = 0usize;
    let mut hints = Vec::new();
    for chunk in deps.chunks(BATCH) {
        attempted += chunk.len();
        let body = build_batch_body(chunk);
        let outcome = async {
            let resp = client
                .post(OSV_BATCH_URL)
                .json(&body)
                .send()
                .await
                .map_err(|e| format!("osv net: {e}"))?;
            let status = resp.status();
            if !status.is_success() {
                return Err(format!("osv {status}"));
            }
            resp.json::<Value>()
                .await
                .map_err(|e| format!("osv body: {e}"))
        }
        .await;
        match outcome {
            Ok(v) => advisories.extend(parse_osv(&v, chunk)),
            Err(msg) => hints.push(msg),
        }
    }
    // `querybatch` omits the CVSS `severity` vector; resolve any still-unknown
    // advisory from `/v1/vulns/{id}` (bounded) so critical/high are real, not 0.
    enrich_severities(&client, &mut advisories).await;
    let source = if hints.len() * 2 >= deps.chunks(BATCH).count() {
        // majority of chunks failed → treat as unreachable but keep whatever parsed
        "unreachable"
    } else {
        "osv"
    };
    finalize(deps.len(), attempted, advisories, source, hints)
}

/// Resolve missing severities via `/v1/vulns/{id}` for advisories still `unknown`.
/// Bounded by [`ENRICH_CAP`]; each fetch is best-effort (fail-open → stays
/// `unknown`). Mutates the `severity`/`summary`/`url` of enriched rows.
async fn enrich_severities(client: &reqwest::Client, advisories: &mut [Advisory]) {
    let mut budget = ENRICH_CAP;
    for a in advisories.iter_mut() {
        if a.severity != "unknown" || a.id.is_empty() {
            continue;
        }
        if budget == 0 {
            break;
        }
        budget -= 1;
        let url = format!("{OSV_VULN_URL}{}", a.id);
        let Ok(resp) = client.get(&url).send().await else {
            continue;
        };
        if !resp.status().is_success() {
            continue;
        }
        let Ok(v) = resp.json::<Value>().await else {
            continue;
        };
        let sev = severity_from_vuln(&v);
        if sev != "unknown" {
            a.severity = sev;
            if a.summary.is_empty() {
                a.summary = v.get("summary").and_then(Value::as_str).unwrap_or("").to_string();
            }
            if a.url.is_empty() {
                a.url = url;
            }
        }
    }
}

/// Durable cache path.
pub fn path(data_dir: &Path) -> PathBuf {
    data_dir.join("gsv_rustsec.json")
}

/// Persist (atomic tmp + rename, like the tracker store).
pub fn save(data_dir: &Path, report: &IntelReport) -> Result<(), String> {
    fs::create_dir_all(data_dir).map_err(|e| format!("create data dir: {e}"))?;
    let raw = serde_json::to_string_pretty(report).map_err(|e| format!("serialize: {e}"))?;
    let target = path(data_dir);
    let tmp = data_dir.join("gsv_rustsec.json.tmp");
    let mut f = fs::File::create(&tmp).map_err(|e| format!("create tmp: {e}"))?;
    f.write_all(raw.as_bytes())
        .map_err(|e| format!("write: {e}"))?;
    fs::rename(&tmp, &target).map_err(|e| format!("rename: {e}"))
}

/// Load the cached report (None when missing / unparsable).
pub fn load(data_dir: &Path) -> Option<IntelReport> {
    let raw = fs::read_to_string(path(data_dir)).ok()?;
    serde_json::from_str::<IntelReport>(&raw).ok()
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
pub fn is_fresh(report: &IntelReport) -> bool {
    age_days(&report.generated_at).is_some_and(|d| d <= REPORT_FRESH_DAYS)
}

/// Read-only wire for `/api/rust-intel` (cached; no network). `ok` is always true.
pub fn wire(data_dir: &Path) -> Value {
    match load(data_dir) {
        Some(r) => json!({
            "ok": true,
            "fresh": is_fresh(&r),
            "report": r,
        }),
        None => json!({
            "ok": true,
            "fresh": false,
            "source": "none",
            "counts": IntelCounts::default(),
            "advisories": [],
            "hint": "not run — `cargo xtask intel`",
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("gsv-intel-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn parse_lock_deps_reads_package_pairs() {
        let d = tmp("lock");
        fs::write(
            d.join("Cargo.lock"),
            r#"version = 3

[[package]]
name = "serde"
version = "1.0.200"

[[package]]
name = "reqwest"
version = "0.12.9"

[[package]]
name = "serde"
version = "1.0.201"
"#,
        )
        .unwrap();
        let deps = parse_lock_deps(&d);
        assert_eq!(
            deps,
            vec![
                ("serde".to_string(), "1.0.200".to_string()),
                ("reqwest".to_string(), "0.12.9".to_string()),
                ("serde".to_string(), "1.0.201".to_string()),
            ]
        );
        // top-level `version = 3` line is ignored (no [[package]] before it).
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn build_batch_body_shape() {
        let deps = vec![("reqwest".to_string(), "0.12.9".to_string())];
        let b = build_batch_body(&deps);
        let q = b["queries"].as_array().unwrap();
        assert_eq!(q.len(), 1);
        assert_eq!(q[0]["package"]["name"], "reqwest");
        assert_eq!(q[0]["package"]["ecosystem"], "crates.io");
        assert_eq!(q[0]["version"], "0.12.9");
    }

    #[test]
    fn parse_osv_aligns_and_buckets() {
        let deps = vec![
            ("reqwest".to_string(), "0.12.9".to_string()),
            ("clean".to_string(), "1.0.0".to_string()),
        ];
        let body = json!({
            "results": [
                { "vulns": [
                    {
                        "id": "GHSA-abcd-efgh-ijkl",
                        "summary": "Denial of service",
                        "aliases": ["RUSTSEC-2024-0399"],
                        "database_specific": { "severity": "HIGH" },
                        "references": [{ "type": "WEB", "url": "https://osv.dev/vulnerability/GHSA-abcd-efgh-ijkl" }]
                    }
                ]},
                {}
            ]
        });
        let ad = parse_osv(&body, &deps);
        assert_eq!(ad.len(), 1);
        assert_eq!(ad[0].package, "reqwest");
        assert_eq!(ad[0].version, "0.12.9");
        assert_eq!(ad[0].severity, "high");
        assert!(ad[0].aliases.iter().any(|a| a == "RUSTSEC-2024-0399"));
        let rep = finalize(deps.len(), deps.len(), ad, "osv", vec![]);
        assert_eq!(rep.counts.high, 1);
        assert_eq!(rep.counts.total, 1);
    }

    #[test]
    fn severity_defaults_unknown_without_database_specific_or_vector() {
        let deps = vec![("x".to_string(), "0.1.0".to_string())];
        let body = json!({ "results": [ { "vulns": [ { "id": "RUSTSEC-1", "summary": "s" } ] } ] });
        let ad = parse_osv(&body, &deps);
        assert_eq!(ad[0].severity, "unknown");
    }

    #[test]
    fn cvss_v3_base_score_matches_reference() {
        // rustls 2026 advisory vector → 5.3 moderate.
        assert_eq!(
            cvss_v3_base_score("CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:L/I:N/A:N"),
            Some(5.3)
        );
        // full scope-changed critical → 10.0.
        assert_eq!(
            cvss_v3_base_score("CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:C/C:H/I:H/A:H"),
            Some(10.0)
        );
        // malformed metric value → None (fail-open to unknown).
        assert_eq!(
            cvss_v3_base_score("CVSS:3.1/AV:Z/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H"),
            None
        );
        assert_eq!(bucket_from_score(9.0), "critical");
        assert_eq!(bucket_from_score(7.0), "high");
        assert_eq!(bucket_from_score(5.3), "moderate");
        assert_eq!(bucket_from_score(2.9), "low");
    }

    #[test]
    fn parse_osv_buckets_from_cvss_vector_when_no_database_severity() {
        let deps = vec![("rustls".to_string(), "0.23.44".to_string())];
        let body = json!({
            "results": [ { "vulns": [ {
                "id": "RUSTSEC-2026-0285",
                "summary": "PKCS#1 parsing panic",
                "severity": [{ "type": "CVSS_V3", "score": "CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:L/I:N/A:N" }]
            } ] } ]
        });
        let ad = parse_osv(&body, &deps);
        assert_eq!(ad[0].severity, "moderate");
    }

    #[test]
    fn collect_dry_is_deterministic_two_advisories() {
        let r = collect_dry();
        assert_eq!(r.source, "dry-run");
        assert!(r.ok);
        assert_eq!(r.counts.total, 2);
        assert_eq!(r.counts.high, 1);
        assert_eq!(r.counts.moderate, 1);
    }

    #[test]
    fn save_load_roundtrip_and_wire_no_cache() {
        let d = tmp("roundtrip");
        assert!(load(&d).is_none());
        let w = wire(&d);
        assert_eq!(w["ok"], true);
        assert_eq!(w["source"], "none");
        let rep = collect_dry();
        save(&d, &rep).unwrap();
        let got = load(&d).unwrap();
        assert_eq!(got.counts.total, 2);
        assert!(wire(&d)["fresh"].as_bool().unwrap(), "just-saved is fresh");
        let _ = fs::remove_dir_all(&d);
    }
}
