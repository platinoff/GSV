//! cargo-deny license/ban radar (#C.3) — shell `cargo deny --format json
//! --target <pinned> check licenses bans advisories`, cache to
//! `data/gsv_deny.json` so `cargo xtask rules-check` and the Galaxy rust-intel
//! card read it **offline** — no cargo run in tests/CI.
//!
//! The live collection runs **only** from `cargo xtask intel` (owner / cadence
//! tick, alongside the OSV sweep); everything else is fail-open on the cached
//! report. When `cargo-deny` is not installed the report says so (`source
//! "not-installed"`) and the HARD `deny` rules-check gate stays advisory —
//! same escape discipline as `rustsec` (`GSV_DENY_HARD=off` turns the gate off
//! entirely).
//!
//! ```text
//! cargo xtask intel            # OSV sweep + cargo-deny stream → data/gsv_deny.json
//! cargo xtask rules-check      # reads the cached report (deny.toml at repo root)
//! ```
//!
//! Output shape (cargo-deny 0.20): ALL JSON lines land on **stderr** —
//! `{"type":"log"}` progress, `{"type":"diagnostic","fields":{severity,code,
//! graphs,labels,message,…}}` per finding, and one closing
//! `{"type":"summary","fields":{licenses:{errors,warnings},bans:{…},
//! advisories:{…}}}` that carries the authoritative per-stream counts.
//!
//! Policy canon: `deny.toml` (allowed licenses MIT/Apache-2.0/BSD/ISC + the
//! companions actually present in the tree; banned: `openssl` / `openssl-sys`
//! — the kit builds on native-tls=schannel, never the openssl C stack on the
//! pinned windows-gnu target). The check runs with `--target
//! x86_64-pc-windows-gnu` so unix/UEFI-only crates (openssl, r-efi) stay out
//! of the graph instead of failing licenses with crates that never compile
//! here.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::vision::rfc3339_now;

/// The kit-pinned triple (rust-toolchain.toml). Hardcoding keeps the deny
/// graph stable; a toolchain-triple drift is a separate gate's concern.
pub const DENY_TARGET: &str = "x86_64-pc-windows-gnu";

/// One parsed cargo-deny diagnostic (capped sample of the full stream).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DenyDiag {
    /// `fields.code` when present (e.g. "rejected"), else "diagnostic".
    pub kind: String,
    /// error | warning | note | help
    pub severity: String,
    /// `crate@version` when the diagnostic names one, else empty.
    pub krate: String,
    pub message: String,
}

/// Authoritative per-stream counts from the closing `type:"summary"` line.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct DenySummary {
    pub license_errors: usize,
    #[serde(default)]
    pub license_warnings: usize,
    pub ban_errors: usize,
    #[serde(default)]
    pub ban_warnings: usize,
    pub advisory_errors: usize,
    #[serde(default)]
    pub advisory_warnings: usize,
}

/// Full deny report. `ok` is always true (fail-open carrier, like the OSV
/// radar); `source` says what actually happened.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DenyReport {
    pub ok: bool,
    /// deny | not-installed | error | none
    pub source: String,
    pub generated_at: String,
    #[serde(flatten)]
    pub counts: DenySummary,
    #[serde(default)]
    pub diagnostics: Vec<DenyDiag>,
    #[serde(default)]
    pub hints: Vec<String>,
}

impl DenyReport {
    /// Whether the HARD rules-check gate must fail on this report.
    pub fn blocking(&self) -> bool {
        self.counts.license_errors > 0 || self.counts.ban_errors > 0
    }
}

/// Cache path: `data/gsv_deny.json`.
pub fn path(data_dir: &Path) -> PathBuf {
    data_dir.join("gsv_deny.json")
}

/// Parse cargo-deny JSON-lines (stderr). Returns the diagnostic samples, the
/// authoritative summary (None when the line is absent), and WARN/ERROR log
/// hints. Non-JSON lines are ignored so a future CLI banner never breaks the
/// parser.
pub fn parse_output(text: &str) -> (Vec<DenyDiag>, Option<DenySummary>, Vec<String>) {
    let mut diags = Vec::new();
    let mut summary = None;
    let mut logs = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        let Ok(v) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        let fields = v.get("fields").cloned().unwrap_or(Value::Null);
        match v.get("type").and_then(Value::as_str) {
            Some("summary") => {
                let f = |key: &str, sub: &str| {
                    fields
                        .get(key)
                        .and_then(|s| s.get(sub))
                        .and_then(Value::as_u64)
                        .unwrap_or(0) as usize
                };
                summary = Some(DenySummary {
                    license_errors: f("licenses", "errors"),
                    license_warnings: f("licenses", "warnings"),
                    ban_errors: f("bans", "errors"),
                    ban_warnings: f("bans", "warnings"),
                    advisory_errors: f("advisories", "errors"),
                    advisory_warnings: f("advisories", "warnings"),
                });
            }
            Some("diagnostic") => {
                let severity = fields
                    .get("severity")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string();
                let kind = fields
                    .get("code")
                    .and_then(Value::as_str)
                    .unwrap_or("diagnostic")
                    .to_string();
                let message = fields
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string();
                // Sample context: the first graph node + the first label span
                // (e.g. the rejected license id) keep the card readable without
                // the full parent chain.
                let krate = fields
                    .get("graphs")
                    .and_then(Value::as_array)
                    .and_then(|g| g.first())
                    .and_then(|n| n.get("Krate"))
                    .map(|k| {
                        format!(
                            "{}@{}",
                            k.get("name").and_then(Value::as_str).unwrap_or("?"),
                            k.get("version").and_then(Value::as_str).unwrap_or("?")
                        )
                    })
                    .unwrap_or_default();
                let span = fields
                    .get("labels")
                    .and_then(Value::as_array)
                    .and_then(|l| {
                        l.iter().find_map(|x| {
                            let m = x.get("message").and_then(Value::as_str).unwrap_or("");
                            let s = x.get("span").and_then(Value::as_str).unwrap_or("");
                            (!m.is_empty() && !s.is_empty()).then(|| format!("{m} ({s})"))
                        })
                    });
                let mut full = message;
                if let Some(sp) = span {
                    if !full.is_empty() {
                        full.push_str(" — ");
                    }
                    full.push_str(&sp);
                }
                if severity.is_empty() && full.is_empty() {
                    continue;
                }
                diags.push(DenyDiag {
                    kind,
                    severity,
                    krate,
                    message: full,
                });
            }
            Some("log") => {
                let level = fields.get("level").and_then(Value::as_str).unwrap_or("");
                if level == "WARN" || level == "ERROR" {
                    if let Some(m) = fields.get("message").and_then(Value::as_str) {
                        logs.push(format!("{level}: {m}"));
                    }
                }
            }
            _ => {}
        }
    }
    (diags, summary, logs)
}

/// Fallback counting over parsed diagnostics (used only when the summary line
/// is absent — older cargo-deny or truncated output).
pub fn count(diags: &[DenyDiag]) -> DenySummary {
    let mut s = DenySummary::default();
    for d in diags {
        let err = d.severity == "error";
        let warn = d.severity == "warning";
        match d.kind.as_str() {
            "rejected" | "license" => {
                if err {
                    s.license_errors += 1;
                } else if warn {
                    s.license_warnings += 1;
                }
            }
            "banned" | "duplicate" | "bans" => {
                if err {
                    s.ban_errors += 1;
                } else if warn {
                    s.ban_warnings += 1;
                }
            }
            "vulnerability" | "unmaintained" | "yanked" | "advisories" | "advisory" => {
                if err {
                    s.advisory_errors += 1;
                } else if warn {
                    s.advisory_warnings += 1;
                }
            }
            _ => {}
        }
    }
    s
}

/// Run `cargo deny --format json --target <pinned> check licenses bans
/// advisories` in the repo root. Fail-open: a missing subcommand or a spawn
/// error yields a `source`-marked report with hints, never a panic.
pub fn collect_live(repo_root: &Path) -> DenyReport {
    let out = Command::new("cargo")
        .args([
            "deny",
            "--format",
            "json",
            "--target",
            DENY_TARGET,
            "check",
            "licenses",
            "bans",
            "advisories",
        ])
        .current_dir(repo_root)
        .output();
    let out = match out {
        Ok(o) => o,
        Err(e) => {
            let not_installed = e.kind() == std::io::ErrorKind::NotFound;
            return DenyReport {
                ok: true,
                source: if not_installed {
                    "not-installed".to_string()
                } else {
                    "error".to_string()
                },
                generated_at: rfc3339_now(),
                counts: DenySummary::default(),
                diagnostics: Vec::new(),
                hints: vec![format!("spawn cargo deny: {e}")],
            };
        }
    };
    // cargo-deny 0.20 emits the JSON lines on stderr; fold stdout in so a
    // future stdout-emitting version still parses.
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let (diags, summary, mut hints) = parse_output(&text);
    let counts = summary.unwrap_or_else(|| count(&diags));
    let not_installed = String::from_utf8_lossy(&out.stderr)
        .lines()
        .any(|l| l.contains("no such command") || l.contains("no such subcommand"));
    if !out.status.success() && counts == DenySummary::default() && !not_installed {
        // Command-level failure with nothing parsed (bad deny.toml, missing
        // lockfile, …) — keep the tail so the owner sees why.
        let tail: Vec<String> = String::from_utf8_lossy(&out.stderr)
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(str::to_string)
            .collect();
        let start = tail.len().saturating_sub(6);
        hints.extend(tail[start..].iter().cloned());
        hints.insert(0, format!("cargo deny exited {}", out.status));
    }
    hints.truncate(6);
    DenyReport {
        ok: true,
        source: if not_installed {
            "not-installed".to_string()
        } else {
            "deny".to_string()
        },
        generated_at: rfc3339_now(),
        counts,
        diagnostics: diags.into_iter().take(DIAG_SAMPLE_CAP).collect(),
        hints,
    }
}

/// Max diagnostics kept as samples (bounds the JSON; parity with ENRICH_CAP).
pub const DIAG_SAMPLE_CAP: usize = 40;

/// Persist (atomic tmp + rename, like the OSV radar).
pub fn save(data_dir: &Path, report: &DenyReport) -> Result<(), String> {
    fs::create_dir_all(data_dir).map_err(|e| format!("create data dir: {e}"))?;
    let raw = serde_json::to_string_pretty(report).map_err(|e| format!("serialize: {e}"))?;
    let target = path(data_dir);
    let tmp = data_dir.join("gsv_deny.json.tmp");
    let mut f = fs::File::create(&tmp).map_err(|e| format!("create tmp: {e}"))?;
    f.write_all(raw.as_bytes())
        .map_err(|e| format!("write: {e}"))?;
    fs::rename(&tmp, &target).map_err(|e| format!("rename: {e}"))
}

/// Load the cached report (None when missing / unparsable).
pub fn load(data_dir: &Path) -> Option<DenyReport> {
    let raw = fs::read_to_string(path(data_dir)).ok()?;
    serde_json::from_str::<DenyReport>(&raw).ok()
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
pub fn is_fresh(report: &DenyReport) -> bool {
    age_days(&report.generated_at).is_some_and(|d| d <= REPORT_FRESH_DAYS)
}

/// Cached report is considered fresh for this many days (OSV radar parity).
pub const REPORT_FRESH_DAYS: i64 = 7;

/// Read-only wire merged into the rust-intel card + `gsv_rust_updates` (cached;
/// no cargo run). One flat shape whether or not a cache exists; `ok` always true.
pub fn wire(data_dir: &Path) -> Value {
    match load(data_dir) {
        Some(r) => json!({
            "ok": true,
            "fresh": is_fresh(&r),
            "source": r.source,
            "generated_at": r.generated_at,
            "license_errors": r.counts.license_errors,
            "license_warnings": r.counts.license_warnings,
            "ban_errors": r.counts.ban_errors,
            "ban_warnings": r.counts.ban_warnings,
            "advisory_errors": r.counts.advisory_errors,
            "advisory_warnings": r.counts.advisory_warnings,
            "diagnostics": r.diagnostics,
            "hints": r.hints,
        }),
        None => json!({
            "ok": true,
            "fresh": false,
            "source": "none",
            "license_errors": 0,
            "license_warnings": 0,
            "ban_errors": 0,
            "ban_warnings": 0,
            "advisory_errors": 0,
            "advisory_warnings": 0,
            "diagnostics": [],
            "hints": [],
            "hint": "not run — `cargo xtask intel` (needs cargo-deny installed)",
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("gsv-deny-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    /// Real cargo-deny 0.20 stderr shape (condensed graphs/labels).
    const SAMPLE: &str = concat!(
        r#"{"fields":{"level":"WARN","message":"unable to find a config path, falling back to default config"},"type":"log"}"#,
        "\n",
        r#"{"fields":{"code":"rejected","graphs":[{"Krate":{"name":"zeroize","version":"1.9.1"},"parents":[]}],"labels":[{"message":"rejected: license is not explicitly allowed","span":"Apache-2.0"}],"message":"failed to satisfy license requirements","severity":"error"},"type":"diagnostic"}"#,
        "\n",
        r#"{"fields":{"code":"banned","graphs":[{"Krate":{"name":"openssl","version":"0.10.82"},"parents":[]}],"labels":[],"message":"crate openssl is banned","severity":"error"},"type":"diagnostic"}"#,
        "\n",
        r#"{"fields":{"code":"vulnerability","graphs":[{"Krate":{"name":"x","version":"1"},"parents":[]}],"labels":[],"message":"vuln","severity":"warning"},"type":"diagnostic"}"#,
        "\n",
        "error: human noise, not json\n",
        r#"{"fields":{"advisories":{"errors":0,"warnings":1},"bans":{"errors":1,"warnings":0},"licenses":{"errors":1,"warnings":0}},"type":"summary"}"#,
        "\n",
    );

    #[test]
    fn parse_output_reads_real_shape() {
        let (diags, summary, logs) = parse_output(SAMPLE);
        assert_eq!(diags.len(), 3, "log/human lines are not diagnostics");
        assert_eq!(diags[0].kind, "rejected");
        assert_eq!(diags[0].severity, "error");
        assert_eq!(diags[0].krate, "zeroize@1.9.1");
        assert!(
            diags[0].message.contains("Apache-2.0"),
            "label span folded into the sample message: {}",
            diags[0].message
        );
        assert_eq!(diags[1].krate, "openssl@0.10.82");
        let s = summary.expect("summary line parsed");
        assert_eq!(s.license_errors, 1);
        assert_eq!(s.ban_errors, 1);
        assert_eq!(s.advisory_warnings, 1);
        assert_eq!(s.ban_warnings, 0);
        assert_eq!(logs.len(), 1, "WARN log becomes a hint");
        assert!(logs[0].contains("config path"));
    }

    #[test]
    fn count_falls_back_over_diagnostics() {
        let (diags, _, _) = parse_output(SAMPLE);
        let s = count(&diags);
        assert_eq!(
            (s.license_errors, s.ban_errors, s.advisory_warnings),
            (1, 1, 1)
        );
    }

    #[test]
    fn save_load_roundtrip_and_freshness() {
        let d = tmp("roundtrip");
        let r = DenyReport {
            ok: true,
            source: "deny".into(),
            generated_at: chrono::Utc::now().to_rfc3339(),
            counts: DenySummary {
                license_errors: 1,
                license_warnings: 0,
                ban_errors: 2,
                ban_warnings: 0,
                advisory_errors: 0,
                advisory_warnings: 1,
            },
            diagnostics: vec![DenyDiag {
                kind: "rejected".into(),
                severity: "error".into(),
                krate: "zeroize@1.9.1".into(),
                message: "x".into(),
            }],
            hints: vec![],
        };
        save(&d, &r).unwrap();
        let back = load(&d).unwrap();
        assert_eq!(back, r);
        assert!(is_fresh(&back));
        assert!(back.blocking(), "license+ban errors must block");
        // Stale date → not fresh.
        let old = DenyReport {
            generated_at: (chrono::Utc::now() - chrono::Duration::days(9)).to_rfc3339(),
            ..r.clone()
        };
        assert!(!is_fresh(&old));
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
}
