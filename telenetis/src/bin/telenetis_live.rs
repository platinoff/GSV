//! `telenetis-live` — always-on supervisor for the Telenetis Telegram bot.
//!
//! Copies the debug binary to `target/live` (so `cargo build` / `cargo test`
//! can overwrite debug without a Windows file lock) and respawns it on exit.
//! This keeps the Telegram webhook reachable so squad bots can always
//! coordinate while someone is online in the Godfather channel.

use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

fn main() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let live_dir = repo_root.join("target/live");
    let _ = std::fs::create_dir_all(&live_dir);
    let live_exe = live_dir.join("telenetis.exe");
    let debug_exe = repo_root.join("target/debug/telenetis.exe");

    let modified_at = |p: &PathBuf| std::fs::metadata(p).and_then(|m| m.modified()).ok();

    if debug_exe.exists() {
        let refresh = !live_exe.exists() || modified_at(&debug_exe) != modified_at(&live_exe);
        if refresh {
            if let Err(e) = std::fs::copy(&debug_exe, &live_exe) {
                eprintln!("telenetis-live: failed to copy debug -> live: {e}");
            } else {
                println!("telenetis-live: refreshed live copy");
            }
        }
    }

    loop {
        if !live_exe.exists() {
            eprintln!("telenetis-live: missing {:?} — rebuild first", live_exe);
            std::thread::sleep(Duration::from_secs(5));
            continue;
        }
        println!("telenetis-live: spawning {:?}", live_exe);
        match Command::new(&live_exe)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
        {
            Ok(mut child) => {
                let _ = child.wait();
            }
            Err(e) => {
                eprintln!("telenetis-live: spawn failed: {e}");
            }
        }
        std::thread::sleep(Duration::from_secs(2));
    }
}
