//! `gsv-live` — always-on supervisor (copy debug → live, loop restart on :9999).
//!
//! Prefer `cargo xtask live`. This bin exists so the listener is not
//! `target/debug/gsv-server.exe` (Windows file lock during `cargo test`).

use std::path::PathBuf;

use gsv::boxes::xtask;
use gsv::{DEFAULT_HOST, DEFAULT_PORT};

fn main() {
    let mut host = std::env::var("GSV_HOST").unwrap_or_else(|_| DEFAULT_HOST.into());
    let mut port = std::env::var("GSV_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_PORT);
    let mut repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--host" => host = args.next().unwrap_or(host),
            "--port" => {
                port = args
                    .next()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(DEFAULT_PORT)
            }
            "--repo-root" => {
                if let Some(p) = args.next() {
                    repo_root = PathBuf::from(p);
                }
            }
            "--help" | "-h" => {
                println!(
                    "Usage: gsv-live [--host H] [--port N] [--repo-root P]\nCanon: cargo xtask live"
                );
                return;
            }
            _ => {}
        }
    }
    if let Err(e) = xtask::run_live(&repo_root, &host, port) {
        eprintln!("gsv-live: {e}");
        std::process::exit(1);
    }
}
