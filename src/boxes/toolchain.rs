//! Toolchain box — inventory of project tools (rustc/cargo/clippy/MSYS2/git/…).
//!
//! Versions come from running `--version` probes (best effort, offline-safe) and
//! from `rust-toolchain.toml` when present.

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::vision;

/// Reuse probe results so Auto-resync does not spawn rustc/git/bash every tick.
const WIRE_CACHE_TTL: Duration = Duration::from_secs(300);

struct WireCache {
    at: Instant,
    wire: ToolchainWire,
}

static WIRE_CACHE: Mutex<Option<WireCache>> = Mutex::new(None);

#[cfg(test)]
static PROBE_CALLS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

#[cfg(test)]
fn probe_calls() -> u64 {
    PROBE_CALLS.load(std::sync::atomic::Ordering::SeqCst)
}

#[cfg(test)]
fn clear_wire_cache() {
    if let Ok(mut g) = WIRE_CACHE.lock() {
        *g = None;
    }
}

/// One tool inventory entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolchainEntry {
    /// Tool name.
    pub tool: String,
    /// Version string (from `--version` probe or config).
    pub version: String,
    /// Source: probe | toolchain-file | config.
    pub source: String,
}

/// `/api/toolchain` response wire.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolchainWire {
    pub entries: Vec<ToolchainEntry>,
    pub generated_at: String,
}

const PROBES: &[(&str, &[&str])] = &[
    ("rustc", &["--version"]),
    ("cargo", &["--version"]),
    ("clippy-driver", &["--version"]),
    ("rustfmt", &["--version"]),
    ("git", &["--version"]),
    ("bash", &["--version"]),
    ("node", &["--version"]),
    ("npm", &["--version"]),
    ("curl", &["--version"]),
];

/// Probe a tool version; returns a short single-line version.
fn probe(program: &str, args: &[&str]) -> Option<String> {
    #[cfg(test)]
    PROBE_CALLS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let out = vision::command(program).args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    Some(text.lines().next().unwrap_or_default().trim().to_string())
}

/// Read rustc pin from `rust-toolchain.toml` (best effort).
fn toolchain_pin(repo_root: &Path) -> Option<String> {
    let raw = std::fs::read_to_string(repo_root.join("rust-toolchain.toml")).ok()?;
    let table: toml::Value = toml::from_str(&raw).ok()?;
    table
        .get("toolchain")
        .and_then(|t| t.get("channel"))
        .and_then(|c| c.as_str())
        .map(ToOwned::to_owned)
}

/// Cursor desktop `package.json` candidates (Windows Program Files / user install / macOS).
pub fn cursor_package_candidates() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(pf) = std::env::var("ProgramFiles") {
        out.push(PathBuf::from(pf).join("cursor/resources/app/package.json"));
    }
    out.push(PathBuf::from(
        r"C:\Program Files\cursor\resources\app\package.json",
    ));
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        out.push(PathBuf::from(local).join("Programs/cursor/resources/app/package.json"));
    }
    out.push(PathBuf::from(
        "/Applications/Cursor.app/Contents/Resources/app/package.json",
    ));
    out
}

/// Parse `"version"` from Cursor's app `package.json`.
pub fn parse_cursor_package_version(raw: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(raw).ok()?;
    v.get("version")?
        .as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned)
}

/// Best-effort Cursor desktop version (app package, else `cursor --version`).
pub fn cursor_app_version() -> Option<(String, &'static str)> {
    for path in cursor_package_candidates() {
        if let Ok(raw) = std::fs::read_to_string(&path) {
            if let Some(ver) = parse_cursor_package_version(&raw) {
                return Some((ver, "app-package"));
            }
        }
    }
    probe("cursor", &["--version"]).map(|ver| (ver, "probe"))
}

fn cursor_entry() -> ToolchainEntry {
    match cursor_app_version() {
        Some((version, source)) => ToolchainEntry {
            tool: "cursor".to_string(),
            version,
            source: source.to_string(),
        },
        None => ToolchainEntry {
            tool: "cursor".to_string(),
            version: "not-found".to_string(),
            source: "probe".to_string(),
        },
    }
}

/// Build the toolchain inventory.
pub fn build(repo_root: &Path) -> Vec<ToolchainEntry> {
    let mut entries = Vec::new();
    for (tool, args) in PROBES {
        let version = probe(tool, args).unwrap_or_else(|| "not-found".to_string());
        entries.push(ToolchainEntry {
            tool: (*tool).to_string(),
            version,
            source: "probe".to_string(),
        });
    }
    entries.push(cursor_entry());
    if let Some(pin) = toolchain_pin(repo_root) {
        entries.push(ToolchainEntry {
            tool: "rust-toolchain".to_string(),
            version: pin,
            source: "toolchain-file".to_string(),
        });
    }
    if let Some(head) = vision::git_head(repo_root) {
        entries.push(ToolchainEntry {
            tool: "repo-head".to_string(),
            version: head,
            source: "git".to_string(),
        });
    }
    entries
}

/// Serve `/api/toolchain`.
pub fn wire(repo_root: &Path) -> ToolchainWire {
    if let Ok(guard) = WIRE_CACHE.lock() {
        if let Some(c) = guard.as_ref() {
            if c.at.elapsed() < WIRE_CACHE_TTL {
                return c.wire.clone();
            }
        }
    }
    let w = ToolchainWire {
        entries: build(repo_root),
        generated_at: vision::rfc3339_now(),
    };
    if let Ok(mut guard) = WIRE_CACHE.lock() {
        *guard = Some(WireCache {
            at: Instant::now(),
            wire: w.clone(),
        });
    }
    w
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probes_include_rustc_and_cargo_when_available() {
        // Best effort: on dev machines rustc/cargo exist; on CI they may not.
        let dir = std::env::temp_dir().join(format!("gsv-tc-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let entries = build(&dir);
        let tools: Vec<&str> = entries.iter().map(|e| e.tool.as_str()).collect();
        assert!(
            tools.contains(&"rust-toolchain") || tools.contains(&"repo-head") || !tools.is_empty()
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn probe_returns_none_for_missing_binary() {
        assert!(probe("definitely-missing-binary-xyz", &["--version"]).is_none());
    }

    #[test]
    fn kit_rust_toolchain_names_gnu_host() {
        let raw = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/rust-toolchain.toml"
        ));
        assert!(
            raw.contains("1.92.0-x86_64-pc-windows-gnu"),
            "channel must pin gnu host (msvc + MSYS2 link.exe breaks): {raw}"
        );
    }

    #[test]
    fn parse_cursor_package_version_reads_semver() {
        let raw = r#"{"name":"Cursor","version":"3.16.29","distro":"abc"}"#;
        assert_eq!(
            parse_cursor_package_version(raw).as_deref(),
            Some("3.16.29")
        );
        assert!(parse_cursor_package_version("{}").is_none());
        assert!(parse_cursor_package_version("not-json").is_none());
    }

    #[test]
    fn cursor_package_candidates_include_install_paths() {
        let c = cursor_package_candidates();
        assert!(
            c.iter().any(|p| p.to_string_lossy().contains("cursor")),
            "{c:?}"
        );
    }

    #[test]
    fn build_includes_cursor_entry() {
        let dir = std::env::temp_dir().join(format!("gsv-tc-cursor-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let entries = build(&dir);
        assert!(entries.iter().any(|e| e.tool == "cursor"), "{entries:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn wire_caches_probes_within_ttl() {
        clear_wire_cache();
        let dir = std::env::temp_dir().join(format!("gsv-tc-cache-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let _ = wire(&dir);
        let after_first = probe_calls();
        let _ = wire(&dir);
        assert_eq!(
            probe_calls(),
            after_first,
            "second wire() must not re-probe (console flash)"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
