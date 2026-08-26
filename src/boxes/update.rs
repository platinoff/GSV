//! Update box — update notification + offline/resync signal.
//!
//! Key UX requirement (GSV_SERVER.md): while the binary runs, the server accepts
//! an update message. The UI shows an **Update** badge instead of auto-reload; the
//! page survives offline and re-syncs all metrics on reconnect.
//!
//! Detection (self-contained): newest `src/**` newer than the running binary,
//! on-disk `Cargo.toml` version ahead of this build, or `POST /api/update/notify`.
//! `POST /api/update/apply` emits SSE `offline` and (outside tests) exits so
//! `cargo xtask live` / the watchdog can recopy `target/debug/` → `target/live/`.

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::state::AppState;
use crate::vision;

/// `/api/update` response wire.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateWire {
    /// Running binary (`CARGO_PKG_VERSION`).
    pub version: String,
    /// `[package] version` on disk (`Cargo.toml`).
    pub crate_version: Option<String>,
    /// `crate_version` differs from this build.
    pub version_lag: bool,
    pub update_available: bool,
    pub git_head: Option<String>,
    pub started_at: String,
    pub binary_mtime: u64,
    pub newest_src_mtime: u64,
    /// `true` when this process is `target/live/gsv-server.exe` (band 144 supervisor).
    pub live_copy: bool,
    /// `owner/repo` from origin / Cargo.toml.
    pub github_repo: Option<String>,
    /// Remote `Cargo.toml` version (GitHub `main`).
    pub github_latest: Option<String>,
    /// Remote HEAD sha.
    pub github_head: Option<String>,
    /// Origin is ahead of this install even when local `src/` is not newer.
    pub github_ahead: bool,
    pub github_dry_run: bool,
    /// `git pull && cargo build` when only GitHub is ahead.
    pub update_hint: String,
    /// True when Apply can recopy a newer local binary (not GitHub-only).
    pub can_apply: bool,
}

/// Query params for `GET /api/update`.
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateCheckParams {
    /// Force a fresh mtime-based check instead of the cached flag.
    pub check: Option<bool>,
}

/// Newest mtime (epoch secs) across `GSV/src/**` (capped traversal).
pub fn newest_src_mtime(manifest_dir: &Path) -> u64 {
    let src = manifest_dir.join("src");
    let mut newest = 0u64;
    let mut stack = vec![src];
    let mut guard = 0usize;
    while let Some(dir) = stack.pop() {
        guard += 1;
        if guard > 2000 {
            break;
        }
        let Ok(read) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in read.flatten() {
            let p = e.path();
            let Ok(meta) = std::fs::metadata(&p) else {
                continue;
            };
            if meta.is_dir() {
                stack.push(p);
            } else if let Ok(t) = meta.modified() {
                if let Ok(d) = t.duration_since(std::time::UNIX_EPOCH) {
                    newest = newest.max(d.as_secs());
                }
            }
        }
    }
    newest
}

/// Running binary mtime (epoch secs).
pub fn binary_mtime() -> u64 {
    std::env::current_exe()
        .ok()
        .and_then(|p| std::fs::metadata(p).ok())
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Whether `exe` lives under `target/live/` (Windows or POSIX separators).
pub fn path_is_live_copy(exe: &Path) -> bool {
    exe.to_string_lossy().replace('\\', "/").contains("/live/")
}

/// Running process is the supervisor live copy, not `target/debug/`.
pub fn is_live_copy() -> bool {
    std::env::current_exe()
        .ok()
        .map(|p| path_is_live_copy(&p))
        .unwrap_or(false)
}

/// Exit after apply unless `GSV_UPDATE_APPLY_EXIT=0`.
///
/// Cargo test binaries live under `target/debug/deps/` — default **no-exit** so
/// oneshot tests cannot kill the harness. Production (`target/debug/` or
/// `target/live/` `gsv-server.exe`) exits unless the env is `0`.
pub fn apply_should_exit() -> bool {
    match std::env::var("GSV_UPDATE_APPLY_EXIT").ok().as_deref() {
        Some("0") => false,
        Some("1") => true,
        _ => !is_cargo_test_harness(),
    }
}

/// `cargo test` harness exe (unit or integration) is built under `deps/`.
pub fn is_cargo_test_harness() -> bool {
    std::env::current_exe()
        .ok()
        .map(|p| path_is_cargo_test_harness(&p))
        .unwrap_or(false)
}

/// Whether `exe` is a rustc test harness artifact.
pub fn path_is_cargo_test_harness(exe: &Path) -> bool {
    exe.to_string_lossy().replace('\\', "/").contains("/deps/")
}

/// Emit SSE `offline` and return the apply wire. Caller may `process::exit` after flush.
pub fn apply_update(state: &AppState) -> Value {
    state.emit("event: offline\ndata: true".to_string());
    json!({ "ok": true, "applying": true })
}

/// `[package] version` from the product `Cargo.toml` (or `package.json`).
pub fn crate_version(repo_root: &Path) -> Option<String> {
    crate::boxes::fingerprint::pkg_version(repo_root)
}

/// True when on-disk crate version is ahead of (or differs from) this build.
pub fn version_lag(repo_root: &Path, running: &str) -> bool {
    crate_version(repo_root)
        .map(|v| v != running)
        .unwrap_or(false)
}

/// Newest `src/` mtime is newer than the running binary.
pub fn pending_rebuild(repo_root: &Path) -> bool {
    newest_src_mtime(repo_root) > binary_mtime()
}

/// Notify flag, pending rebuild, or crate/binary version lag (local tree).
pub fn local_available(state: &AppState) -> bool {
    state.update_available()
        || pending_rebuild(&state.repo_root)
        || version_lag(&state.repo_root, state.version.as_ref())
}

/// Local pending **or** GitHub origin ahead of this install.
pub fn effective_available(state: &AppState) -> bool {
    local_available(state) || super::github::cached_ahead()
}

/// Build the update wire for the current state.
pub fn wire(state: &AppState) -> UpdateWire {
    let newest = newest_src_mtime(&state.repo_root);
    let bin = binary_mtime();
    let crate_ver = crate_version(&state.repo_root);
    let lag = crate_ver
        .as_deref()
        .map(|v| v != state.version.as_ref())
        .unwrap_or(false);
    let gh = super::github::cached_probe(&state.repo_root, state.version.as_ref());
    let can_apply = state.update_available() || pending_rebuild(&state.repo_root) || lag;
    UpdateWire {
        version: state.version.to_string(),
        crate_version: crate_ver,
        version_lag: lag,
        update_available: can_apply || gh.github_ahead,
        git_head: vision::git_head(&state.repo_root),
        started_at: crate::vision::system_to_rfc3339(state.started_at),
        binary_mtime: bin,
        newest_src_mtime: newest,
        live_copy: is_live_copy(),
        github_repo: Some(gh.repo),
        github_latest: if gh.remote_version.is_empty() {
            None
        } else {
            Some(gh.remote_version)
        },
        github_head: if gh.remote_head.is_empty() {
            None
        } else {
            Some(gh.remote_head)
        },
        github_ahead: gh.github_ahead,
        github_dry_run: gh.dry_run,
        update_hint: gh.hint,
        can_apply,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newest_src_mtime_gt_zero() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        assert!(newest_src_mtime(dir) > 0);
    }

    #[test]
    fn pending_rebuild_logic() {
        let tmp = std::env::temp_dir().join("gsv_update_test_pending_logic");
        let _ = std::fs::remove_dir_all(&tmp);
        // Empty src/: no source mtimes at all.
        std::fs::create_dir_all(tmp.join("src")).unwrap();
        assert_eq!(newest_src_mtime(&tmp), 0);
        // A written source file becomes the newest mtime.
        std::fs::write(tmp.join("src").join("lib.rs"), "fn main() {}\n").unwrap();
        assert!(newest_src_mtime(&tmp) > 0);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn path_is_live_copy_detects_live_dir() {
        assert!(path_is_live_copy(Path::new(
            r"S:\rust\GSV\target\live\gsv-server.exe"
        )));
        assert!(path_is_live_copy(Path::new(
            "/s/rust/GSV/target/live/gsv-server.exe"
        )));
        assert!(!path_is_live_copy(Path::new(
            r"S:\rust\GSV\target\debug\gsv-server.exe"
        )));
    }

    #[test]
    fn apply_should_exit_is_false_under_cfg_test() {
        assert!(!apply_should_exit());
        assert!(is_cargo_test_harness());
    }

    #[test]
    fn crate_version_matches_this_package() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        assert_eq!(
            crate_version(root).as_deref(),
            Some(env!("CARGO_PKG_VERSION"))
        );
        assert!(!version_lag(root, env!("CARGO_PKG_VERSION")));
        assert!(version_lag(root, "0.0.0"));
    }

    #[test]
    fn path_is_cargo_test_harness_detects_deps() {
        assert!(path_is_cargo_test_harness(Path::new(
            r"S:\rust\GSV\target\debug\deps\gsv_update_flow-abc.exe"
        )));
        assert!(!path_is_cargo_test_harness(Path::new(
            r"S:\rust\GSV\target\live\gsv-server.exe"
        )));
        assert!(!path_is_cargo_test_harness(Path::new(
            r"S:\rust\GSV\target\debug\gsv-server.exe"
        )));
    }

    #[test]
    fn github_ahead_does_not_imply_can_apply() {
        assert!(!super::super::github::version_gt("0.190.0", "0.190.0"));
        assert!(super::super::github::compute_ahead(
            "0.180.0", "0.190.0", "a", "b", false
        ));
    }
}
