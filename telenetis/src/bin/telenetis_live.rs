//! `telenetis-live` — always-on supervisor for the Telenetis Telegram bot.
//!
//! Copies the debug binary to `target/live` (so `cargo build` / `cargo test`
//! can overwrite debug without a Windows file lock) and respawns it on exit.
//! This keeps the Telegram webhook reachable so squad bots can always
//! coordinate while someone is online in the Godfather channel.
//!
//! Debug-binary discovery (bug-hunt 3): `CARGO_TARGET_DIR` wins when set;
//! else the outermost ancestor holding `Cargo.toml` (workspace target —
//! `S:/rust/GSV/target/debug` for this kit, where cargo really builds,
//! including via the parent `.cargo/config.toml` `target-dir`); else the
//! crate-local `target/` (detached checkouts). The refresh runs on every
//! respawn, not just at startup, so a rebuild lands live on the next bot
//! exit with no manual copy.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;
#[cfg(windows)]
const DETACHED_PROCESS: u32 = 0x0000_0008;
#[cfg(windows)]
const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;

/// Debug binary for a crate checkout: explicit env override, else the
/// workspace target above it, else the crate-local target. Mirrored (same
/// rule, opposite direction) by GSV `watchdog::telenetis_debug_exe` — keep
/// the two in sync.
fn debug_exe(crate_dir: &Path, exe: &str) -> PathBuf {
    if let Some(dir) = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
    {
        return dir.join("debug").join(exe);
    }
    let mut root = crate_dir.to_path_buf();
    while let Some(parent) = root.parent() {
        if parent.join("Cargo.toml").is_file() {
            root = parent.to_path_buf();
        } else {
            break;
        }
    }
    if root != crate_dir {
        root.join("target").join("debug").join(exe)
    } else {
        crate_dir.join("target").join("debug").join(exe)
    }
}

fn refresh_live_copy(debug_exe: &Path, live_exe: &Path) {
    if !debug_exe.exists() {
        return;
    }
    let modified_at = |p: &Path| std::fs::metadata(p).and_then(|m| m.modified()).ok();
    if live_exe.exists() && modified_at(debug_exe) == modified_at(live_exe) {
        return;
    }
    if let Err(e) = std::fs::copy(debug_exe, live_exe) {
        eprintln!("telenetis-live: failed to copy debug -> live: {e}");
    } else {
        println!("telenetis-live: refreshed live copy");
    }
}

fn main() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let live_dir = repo_root.join("target/live");
    let _ = std::fs::create_dir_all(&live_dir);
    let live_exe = live_dir.join("telenetis.exe");
    let debug = debug_exe(&repo_root, "telenetis.exe");

    refresh_live_copy(&debug, &live_exe);

    loop {
        if !live_exe.exists() {
            eprintln!("telenetis-live: missing {:?} — rebuild first", live_exe);
            std::thread::sleep(Duration::from_secs(5));
            continue;
        }
        println!("telenetis-live: spawning {:?}", live_exe);
        let mut cmd = Command::new(&live_exe);
        cmd.stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP);
        }
        match cmd.spawn() {
            Ok(mut child) => {
                let _ = child.wait();
            }
            Err(e) => {
                eprintln!("telenetis-live: spawn failed: {e}");
            }
        }
        // The bot just exited (crash/restart): pick up a rebuild, if any,
        // before respawning — no manual copy step.
        refresh_live_copy(&debug, &live_exe);
        std::thread::sleep(Duration::from_secs(2));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Env-mutating tests run in parallel threads — serialize them.
    static ENV_SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn sandbox(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "tns-live-resolve-{}-{}-{}",
            std::process::id(),
            tag,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn debug_exe_prefers_workspace_target() {
        // crate/ nested under a workspace root: builds land in the
        // ancestor target (this kit's layout), not the crate target.
        let _serial = ENV_SERIAL.lock().unwrap();
        let prev = std::env::var_os("CARGO_TARGET_DIR");
        std::env::remove_var("CARGO_TARGET_DIR");
        let ws = sandbox("ws");
        let krate = ws.join("telenetis");
        std::fs::create_dir_all(&krate).unwrap();
        std::fs::write(ws.join("Cargo.toml"), b"[workspace]\n").unwrap();
        std::fs::write(krate.join("Cargo.toml"), b"[package]\n").unwrap();
        assert_eq!(
            debug_exe(&krate, "telenetis.exe"),
            ws.join("target/debug/telenetis.exe")
        );
        let _ = std::fs::remove_dir_all(&ws);
        match prev {
            Some(v) => std::env::set_var("CARGO_TARGET_DIR", v),
            None => std::env::remove_var("CARGO_TARGET_DIR"),
        }
    }

    #[test]
    fn debug_exe_honors_cargo_target_dir_override() {
        let _serial = ENV_SERIAL.lock().unwrap();
        let prev = std::env::var_os("CARGO_TARGET_DIR");
        let custom = sandbox("custom");
        std::env::set_var("CARGO_TARGET_DIR", &custom);
        let krate = sandbox("ws2").join("telenetis");
        assert_eq!(
            debug_exe(&krate, "telenetis.exe"),
            custom.join("debug/telenetis.exe")
        );
        match prev {
            Some(v) => std::env::set_var("CARGO_TARGET_DIR", v),
            None => std::env::remove_var("CARGO_TARGET_DIR"),
        }
        let _ = std::fs::remove_dir_all(&custom);
    }

    #[test]
    fn debug_exe_falls_back_to_crate_target_when_detached() {
        // No ancestor Cargo.toml: detached checkout, crate-local target.
        let _serial = ENV_SERIAL.lock().unwrap();
        let prev = std::env::var_os("CARGO_TARGET_DIR");
        std::env::remove_var("CARGO_TARGET_DIR");
        let krate = sandbox("detached").join("telenetis");
        std::fs::create_dir_all(&krate).unwrap();
        assert_eq!(
            debug_exe(&krate, "telenetis.exe"),
            krate.join("target/debug/telenetis.exe")
        );
        let _ = std::fs::remove_dir_all(krate.parent().unwrap());
        match prev {
            Some(v) => std::env::set_var("CARGO_TARGET_DIR", v),
            None => std::env::remove_var("CARGO_TARGET_DIR"),
        }
    }

    #[test]
    fn refresh_copies_newer_debug_over_live() {
        let dir = sandbox("refresh");
        std::fs::create_dir_all(&dir).unwrap();
        let debug = dir.join("telenetis.exe");
        let live = dir.join("live-exe");
        std::fs::write(&debug, b"new").unwrap();
        // Sleep-free mtime ordering: live missing → copy.
        refresh_live_copy(&debug, &live);
        assert_eq!(std::fs::read(&live).unwrap(), b"new");
        // Same content, same mtime source → second call keeps the copy
        // without erroring (idempotent respawn path).
        refresh_live_copy(&debug, &live);
        assert_eq!(std::fs::read(&live).unwrap(), b"new");
        // Missing debug → no-op, never deletes live.
        std::fs::remove_file(&debug).unwrap();
        refresh_live_copy(&debug, &live);
        assert!(live.is_file());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
