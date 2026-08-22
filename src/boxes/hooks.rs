//! Tests/bench hooks box — run status from `target/` artifacts WITHOUT recompiling.
//!
//! - `GET /api/hooks/tests` → status + discovered test binaries under
//!   `{repo_root}/target/debug/deps/` + latest rust diagnostics (warnings/errors).
//! - `GET /api/hooks/bench` → Criterion medians (read `target/criterion/` if
//!   present) + latest `speed_index.json` wall-clock.
//!
//! Read-only: never invokes `cargo build`/`cargo test`.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::vision;

/// `/api/hooks/tests` response wire.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HooksTestsWire {
    pub test_bins: Vec<String>,
    pub diagnostics: Option<DiagnosticsSummary>,
    pub status: String,
}

/// `/api/hooks/bench` response wire.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HooksBenchWire {
    pub criterion_dirs: Vec<String>,
    pub speed_index: Option<SpeedSummary>,
    pub status: String,
}

/// Latest Rust/Clippy diagnostics summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticsSummary {
    pub warnings: u64,
    pub errors: u64,
    pub ok: bool,
    pub recorded_at: Option<String>,
}

/// Latest `cargo test-ci` wall-clock summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeedSummary {
    pub test_ci_wall_secs: f64,
    pub test_ci_ok: bool,
    pub recorded_at: Option<String>,
}

/// List `target/debug/deps/*.exe` test binaries (read-only).
///
/// A deps artifact counts as a test harness unless its stem (minus the cargo
/// hash suffix) is one of the crate's declared lib / bin / bench stems. Legacy
/// `poolai*` / `test_*` prefixes stay accepted for cross-repo reuse.
pub fn test_bins(repo_root: &Path) -> Vec<String> {
    let dir = repo_root.join("target/debug/deps");
    let Ok(read) = fs::read_dir(&dir) else {
        return Vec::new();
    };
    let declared = declared_artifact_stems(repo_root);
    let mut bins: Vec<String> = read
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            if !name.ends_with(".exe") || name.contains('\\') {
                return None;
            }
            let stem = name.strip_suffix(".exe")?;
            is_test_artifact(artifact_base(stem), &declared).then_some(name)
        })
        .collect();
    bins.sort();
    bins
}

/// Lib + bin + bench artifact stems declared by the crate (never test harnesses).
fn declared_artifact_stems(repo_root: &Path) -> std::collections::HashSet<String> {
    let mut out = std::collections::HashSet::new();
    if let Ok(raw) = fs::read_to_string(repo_root.join("Cargo.toml")) {
        if let Ok(v) = toml::from_str::<toml::Value>(&raw) {
            if let Some(name) = v
                .get("package")
                .and_then(|p| p.get("name"))
                .and_then(toml::Value::as_str)
            {
                out.insert(name.replace('-', "_"));
            }
        }
    }
    for sub in ["src/bin", "benches"] {
        let Ok(read) = fs::read_dir(repo_root.join(sub)) else {
            continue;
        };
        for e in read.flatten() {
            let p = e.path();
            if p.extension().is_some_and(|x| x == "rs") {
                if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
                    out.insert(stem.replace('-', "_"));
                }
            }
        }
    }
    out
}

/// `gsv_tickets_contracts-3fa2b12c9d81e5f7` → `gsv_tickets_contracts`.
fn artifact_base(file_stem: &str) -> &str {
    match file_stem.rsplit_once('-') {
        Some((base, suffix))
            if suffix.len() >= 8 && suffix.chars().all(|c| c.is_ascii_hexdigit()) =>
        {
            base
        }
        _ => file_stem,
    }
}

/// True when a deps artifact stem is an integration-test harness.
fn is_test_artifact(base: &str, declared: &std::collections::HashSet<String>) -> bool {
    if base.starts_with("test_") || base.starts_with("poolai") {
        return true;
    }
    base.starts_with("gsv") && !declared.contains(base)
}

/// Read diagnostics summary from `docs/{development,vision}/rust_diagnostics.json`.
pub fn diagnostics(repo_root: &Path) -> Option<DiagnosticsSummary> {
    let v = vision::read_vision_json(repo_root, "rust_diagnostics.json")?;
    let latest = v.get("latest")?;
    Some(DiagnosticsSummary {
        warnings: latest.get("warnings")?.as_u64()?,
        errors: latest.get("errors")?.as_u64()?,
        ok: latest.get("ok")?.as_bool()?,
        recorded_at: latest
            .get("recorded_at")
            .and_then(|r| r.as_str())
            .map(ToOwned::to_owned),
    })
}

/// Read speed index summary from `docs/{development,vision}/speed_index.json`.
pub fn speed(repo_root: &Path) -> Option<SpeedSummary> {
    let v = vision::read_vision_json(repo_root, "speed_index.json")?;
    let latest = v.get("latest")?;
    Some(SpeedSummary {
        test_ci_wall_secs: latest.get("test_ci_wall_secs")?.as_f64()?,
        test_ci_ok: latest.get("test_ci_ok")?.as_bool()?,
        recorded_at: latest
            .get("test_ci_recorded_at")
            .and_then(|r| r.as_str())
            .map(ToOwned::to_owned),
    })
}

/// List `target/criterion/` benchmark dirs (read-only).
pub fn criterion_dirs(repo_root: &Path) -> Vec<String> {
    let dir = repo_root.join("target/criterion");
    let Ok(read) = fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut dirs: Vec<String> = read
        .flatten()
        .filter_map(|e| {
            e.path()
                .is_dir()
                .then(|| e.file_name().to_string_lossy().to_string())
        })
        .collect();
    dirs.sort();
    dirs
}

/// Serve `/api/hooks/tests`.
pub fn tests_wire(repo_root: &Path) -> HooksTestsWire {
    let diagnostics = diagnostics(repo_root);
    let test_bins = test_bins(repo_root);
    let status = if test_bins.is_empty() && diagnostics.is_none() {
        "no-artifacts"
    } else {
        "ready"
    };
    HooksTestsWire {
        test_bins,
        diagnostics,
        status: status.to_string(),
    }
}

/// Serve `/api/hooks/bench`.
pub fn bench_wire(repo_root: &Path) -> HooksBenchWire {
    let criterion_dirs = criterion_dirs(repo_root);
    let speed_index = speed(repo_root);
    let status = if criterion_dirs.is_empty() && speed_index.is_none() {
        "no-artifacts"
    } else {
        "ready"
    };
    HooksBenchWire {
        criterion_dirs,
        speed_index,
        status: status.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostics_reads_canon_or_none() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("parent");
        // The poolAI repo canon file may or may not exist during GSV-only builds.
        let _ = diagnostics(root);
        let _ = speed(root);
    }

    #[test]
    fn test_bins_no_panic_on_missing_target() {
        let tmp = std::env::temp_dir().join("gsv-no-target");
        assert!(test_bins(&tmp).is_empty());
        assert!(criterion_dirs(&tmp).is_empty());
    }

    #[test]
    fn artifact_base_strips_cargo_hash_suffix() {
        assert_eq!(
            artifact_base("gsv_tickets_contracts-3fa2b12c9d81e5f7"),
            "gsv_tickets_contracts"
        );
        // Not a hex-hash suffix → keep the whole stem.
        assert_eq!(artifact_base("gsv_dev"), "gsv_dev");
        assert_eq!(artifact_base("weird-1a"), "weird-1a");
    }

    #[test]
    fn is_test_artifact_excludes_declared_lib_bin_bench() {
        let declared: std::collections::HashSet<String> = ["gsv", "gsv_server", "gsv_dev"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert!(is_test_artifact("gsv_tickets_contracts", &declared));
        assert!(is_test_artifact("test_foo", &declared));
        assert!(is_test_artifact("poolai_loc_audit", &declared));
        assert!(!is_test_artifact("gsv_server", &declared));
        assert!(!is_test_artifact("gsv_dev", &declared));
        assert!(!is_test_artifact("build_script_build", &declared));
    }

    #[test]
    fn test_bins_lists_gsv_contracts_when_deps_exist() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let deps = root.join("target/debug/deps");
        if fs::read_dir(&deps).is_err() {
            return; // fresh checkout without target/
        }
        let bins = test_bins(root);
        assert!(
            bins.iter().any(|n| n.contains("contracts")),
            "deps exists but no contract harness listed: {bins:?}"
        );
        assert!(
            !bins
                .iter()
                .any(|n| n.starts_with("gsv_server-") || n.starts_with("build_script_build")),
            "bins/bench/build artifacts must not be listed: {bins:?}"
        );
    }
}
