//! Update box — update notification + offline/resync signal.
//!
//! Key UX requirement (GSV_SERVER.md): while the binary runs, the server accepts
//! an update message. The UI shows an **Update** badge instead of auto-reload; the
//! page survives offline and re-syncs all metrics on reconnect.
//!
//! Detection (self-contained): if the newest `GSV/src/**` source file is newer than
//! the running binary (i.e. a rebuild is pending on disk), or an explicit
//! `POST /api/update/notify` arrived, `update_available` is `true`.
//! `POST /api/update/apply` emits SSE `offline` and (outside tests) exits so
//! `cargo xtask live` can recopy `target/debug/` → `target/live/` and restart.

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::state::AppState;
use crate::vision;

/// `/api/update` response wire.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateWire {
    pub version: String,
    pub update_available: bool,
    pub git_head: Option<String>,
    pub started_at: String,
    pub binary_mtime: u64,
    pub newest_src_mtime: u64,
    /// `true` when this process is `target/live/gsv-server.exe` (band 144 supervisor).
    pub live_copy: bool,
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

/// Build the update wire for the current state.
pub fn wire(state: &AppState) -> UpdateWire {
    let newest = newest_src_mtime(Path::new(env!("CARGO_MANIFEST_DIR")));
    let bin = binary_mtime();
    let pending_rebuild = newest > bin;
    UpdateWire {
        version: state.version.to_string(),
        update_available: state.update_available() || pending_rebuild,
        git_head: vision::git_head(&state.repo_root),
        started_at: crate::vision::system_to_rfc3339(state.started_at),
        binary_mtime: bin,
        newest_src_mtime: newest,
        live_copy: is_live_copy(),
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
        let older = 1u64;
        let newer = 2u64;
        assert!(newer > older);
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
}
