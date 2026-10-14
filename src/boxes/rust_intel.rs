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

/// One direct-dependency freshness row (#B).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CrateUpdate {
    pub name: String,
    /// installed version from `Cargo.lock`
    pub installed: String,
    /// `max_stable_version` from crates.io (empty when lookup failed)
    pub latest: String,
    /// current | patch | minor | major | unknown
    pub kind: String,
}

/// Freshness bucket counts (#B).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct UpdateCounts {
    pub current: usize,
    pub patch: usize,
    pub minor: usize,
    pub major: usize,
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
    /// #B framework freshness over direct `Cargo.toml` deps (crates.io latest).
    #[serde(default)]
    pub deps_direct: usize,
    #[serde(default)]
    pub updates: Vec<CrateUpdate>,
    #[serde(default)]
    pub update_counts: UpdateCounts,
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
        deps_direct: 0,
        updates: Vec::new(),
        update_counts: UpdateCounts::default(),
    }
}

const CRATES_API: &str = "https://crates.io/api/v1/crates/";

/// Direct dependency names from the `dependencies` / `dev-dependencies` /
/// `build-dependencies` tables of `Cargo.toml` (excludes workspace/path entries
/// that have no crates.io identity). Best-effort: empty on a parse miss.
pub fn parse_cargo_direct_deps(repo_root: &Path) -> Vec<String> {
    let Ok(raw) = fs::read_to_string(repo_root.join("Cargo.toml")) else {
        return Vec::new();
    };
    let Ok(doc) = raw.parse::<toml::Table>() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for table in ["dependencies", "dev-dependencies", "build-dependencies"] {
        if let Some(dep) = doc.get(table).and_then(|v| v.as_table()) {
            for name in dep.keys() {
                out.push(name.clone());
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

/// `name -> installed version` from `Cargo.lock` (first occurrence wins).
pub fn installed_versions(repo_root: &Path) -> std::collections::HashMap<String, String> {
    let mut m = std::collections::HashMap::new();
    for (n, v) in parse_lock_deps(repo_root) {
        m.entry(n).or_insert(v);
    }
    m
}

/// `.crate.max_stable_version` from a crates.io `/api/v1/crates/{name}` body.
pub fn latest_from_crates(body: &Value) -> Option<String> {
    body.get("crate")?
        .get("max_stable_version")?
        .as_str()
        .map(|s| s.to_string())
}

fn semver(v: &str) -> Option<(u64, u64, u64)> {
    let core = v.split(['-', '+']).next().unwrap_or(v);
    let mut it = core.split('.');
    let a = it.next()?.parse().ok()?;
    let b = it.next().unwrap_or("0").parse().unwrap_or(0);
    let c = it.next().unwrap_or("0").parse().unwrap_or(0);
    Some((a, b, c))
}

/// Classify an installed→latest gap: current | patch | minor | major | unknown.
pub fn semver_kind(installed: &str, latest: &str) -> &'static str {
    let (ia, ib, ic) = match semver(installed) {
        Some(x) => x,
        None => return "unknown",
    };
    let (la, lb, lc) = match semver(latest) {
        Some(x) => x,
        None => return "unknown",
    };
    if (ia, ib, ic) >= (la, lb, lc) {
        "current"
    } else if ia != la {
        "major"
    } else if ib != lb {
        "minor"
    } else {
        "patch"
    }
}

/// Pure freshness builder: for each direct dep with an installed version, map
/// its crates.io latest (via `latest_of`) into a classified `CrateUpdate`. A dep
/// with no lookup yields kind `unknown`. Testable without network.
pub fn build_updates<F: Fn(&str) -> Option<String>>(
    deps: &[String],
    installed: &std::collections::HashMap<String, String>,
    latest_of: F,
) -> (Vec<CrateUpdate>, UpdateCounts) {
    let mut updates = Vec::new();
    let mut counts = UpdateCounts::default();
    for name in deps {
        let Some(inst) = installed.get(name) else {
            continue;
        };
        let latest = latest_of(name).unwrap_or_default();
        let kind = if latest.is_empty() {
            "unknown"
        } else {
            semver_kind(inst, &latest)
        };
        match kind {
            "current" => counts.current += 1,
            "patch" => counts.patch += 1,
            "minor" => counts.minor += 1,
            "major" => counts.major += 1,
            _ => counts.unknown += 1,
        }
        updates.push(CrateUpdate {
            name: name.clone(),
            installed: inst.clone(),
            latest,
            kind: kind.to_string(),
        });
    }
    (updates, counts)
}

/// Fetch crates.io `max_stable_version` for one crate (best-effort, UA required
/// by crates.io). None on any failure.
async fn crates_latest(client: &reqwest::Client, name: &str) -> Option<String> {
    let url = format!("{CRATES_API}{name}");
    let resp = client
        .get(&url)
        .header(
            reqwest::header::USER_AGENT,
            "gsv-radar/1.0 (security radar)",
        )
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let body = resp.json::<Value>().await.ok()?;
    latest_from_crates(&body)
}

/// #B freshness over direct deps: crates.io latest per dep, classified. Bounded
/// to [`ENRICH_CAP`] lookups (most direct dep sets are well under that).
async fn collect_freshness(
    client: &reqwest::Client,
    repo_root: &Path,
) -> (Vec<CrateUpdate>, UpdateCounts, usize) {
    let deps = parse_cargo_direct_deps(repo_root);
    let installed = installed_versions(repo_root);
    let mut latests = std::collections::HashMap::new();
    for name in deps.iter().take(ENRICH_CAP) {
        if let Some(v) = crates_latest(client, name).await {
            latests.insert(name.clone(), v);
        }
    }
    let (updates, counts) = build_updates(&deps, &installed, |n| latests.get(n).cloned());
    (updates, counts, deps.len())
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
    let mut report = finalize(
        advisories.len(),
        advisories.len(),
        advisories,
        "dry-run",
        vec![],
    );
    report.deps_direct = 3;
    report.updates = vec![
        CrateUpdate {
            name: "example-current".into(),
            installed: "1.2.3".into(),
            latest: "1.2.3".into(),
            kind: "current".into(),
        },
        CrateUpdate {
            name: "axum".into(),
            installed: "0.7.0".into(),
            latest: "0.8.9".into(),
            kind: "minor".into(),
        },
        CrateUpdate {
            name: "legacy".into(),
            installed: "1.0.0".into(),
            latest: "2.1.0".into(),
            kind: "major".into(),
        },
    ];
    report.update_counts = UpdateCounts {
        current: 1,
        patch: 0,
        minor: 1,
        major: 1,
        unknown: 0,
    };
    report
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
    // #B framework freshness (same client, bounded, fail-open).
    let (updates, update_counts, deps_direct) = collect_freshness(&client, repo_root).await;
    let source = if hints.len() * 2 >= deps.chunks(BATCH).count() {
        // majority of chunks failed → treat as unreachable but keep whatever parsed
        "unreachable"
    } else {
        "osv"
    };
    let mut report = finalize(deps.len(), attempted, advisories, source, hints);
    report.updates = updates;
    report.update_counts = update_counts;
    report.deps_direct = deps_direct;
    report
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
                a.summary = v
                    .get("summary")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
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

/// Read-only wire for `GET /api/rust-intel` and MCP `gsv_rust_updates` (cached;
/// no network). One flat shape whether or not a cache exists; `ok` always true.
pub fn wire(data_dir: &Path) -> Value {
    match load(data_dir) {
        Some(r) => json!({
            "ok": true,
            "fresh": is_fresh(&r),
            "source": r.source,
            "generated_at": r.generated_at,
            "deps_total": r.deps_total,
            "deps_direct": r.deps_direct,
            "queries_attempted": r.queries_attempted,
            "counts": r.counts,
            "update_counts": r.update_counts,
            "advisories": r.advisories,
            "updates": r.updates,
            "hints": r.hints,
        }),
        None => json!({
            "ok": true,
            "fresh": false,
            "source": "none",
            "deps_total": 0,
            "deps_direct": 0,
            "queries_attempted": 0,
            "counts": IntelCounts::default(),
            "update_counts": UpdateCounts::default(),
            "advisories": [],
            "updates": [],
            "hints": [],
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
        // #B freshness fixture present + counts add up.
        assert_eq!(r.deps_direct, 3);
        assert_eq!(r.updates.len(), 3);
        assert_eq!(r.update_counts.current, 1);
        assert_eq!(r.update_counts.minor, 1);
        assert_eq!(r.update_counts.major, 1);
    }

    #[test]
    fn semver_kind_matrix() {
        assert_eq!(semver_kind("1.2.3", "1.2.3"), "current");
        assert_eq!(semver_kind("1.2.3", "1.2.9"), "patch");
        assert_eq!(semver_kind("0.7.0", "0.8.9"), "minor");
        assert_eq!(semver_kind("1.9.9", "2.0.0"), "major");
        assert_eq!(semver_kind("2.0.0", "1.9.0"), "current"); // ahead → current
        assert_eq!(semver_kind("0.8", "0.8.9"), "patch"); // 0.8 == 0.8.0
        assert_eq!(semver_kind("garbage", "1.0.0"), "unknown");
        assert_eq!(semver_kind("1.0.0", ""), "unknown");
    }

    #[test]
    fn build_updates_classifies_and_counts() {
        use std::collections::HashMap;
        let deps = vec![
            "axum".to_string(),
            "tokio".to_string(),
            "mystery".to_string(),
        ];
        let mut installed = HashMap::new();
        installed.insert("axum".to_string(), "0.7.0".to_string());
        installed.insert("tokio".to_string(), "1.53.0".to_string());
        installed.insert("mystery".to_string(), "0.1.0".to_string());
        let (updates, counts) = build_updates(&deps, &installed, |n| match n {
            "axum" => Some("0.8.9".to_string()),
            "tokio" => Some("1.53.0".to_string()),
            _ => None, // mystery → lookup failed → unknown
        });
        assert_eq!(updates.len(), 3);
        assert_eq!(counts.minor, 1); // axum
        assert_eq!(counts.current, 1); // tokio
        assert_eq!(counts.unknown, 1); // mystery
        let axum = updates.iter().find(|u| u.name == "axum").unwrap();
        assert_eq!(axum.kind, "minor");
        assert_eq!(axum.latest, "0.8.9");
    }

    #[test]
    fn parse_cargo_direct_deps_reads_tables() {
        let d = tmp("cargo");
        fs::write(
            d.join("Cargo.toml"),
            r#"[package]
name = "x"
version = "0.1.0"

[dependencies]
axum = "0.8"
serde = { version = "1", features = ["derive"] }

[dev-dependencies]
tokio = "1"

[[bin]]
name = "x"
path = "src/main.rs"
"#,
        )
        .unwrap();
        let deps = parse_cargo_direct_deps(&d);
        assert_eq!(
            deps,
            vec!["axum".to_string(), "serde".to_string(), "tokio".to_string()]
        );
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn latest_from_crates_reads_max_stable() {
        let body =
            json!({ "crate": { "max_stable_version": "0.8.9", "newest_version": "0.9.0-beta" } });
        assert_eq!(latest_from_crates(&body).as_deref(), Some("0.8.9"));
        assert_eq!(latest_from_crates(&json!({})).as_deref(), None);
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
