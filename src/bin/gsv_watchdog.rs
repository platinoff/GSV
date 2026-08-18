//! `gsv-watchdog` — keep `:9999` up when `gsv-live` (or Cursor) dies.
//!
//! Probes `GET /api/health`. After consecutive failures, copies
//! `target/debug/gsv-server.exe` → `target/live/` and spawns a detached
//! listener. Writes `target/live/watchdog.json` so Galaxy health can show
//! `watchdog_alive`.
//!
//! ```text
//! cargo run --quiet --bin gsv-watchdog
//! cargo run --quiet --bin gsv-watchdog -- --once
//! cargo xtask watchdog
//! cargo xtask watchdog-install
//! ```

use std::path::PathBuf;
use std::time::Duration;

use gsv::boxes::watchdog::{
    self, epoch_now, heartbeat_fresh, heartbeat_path, read_heartbeat, write_heartbeat, Heartbeat,
    SpawnOutcome, DEFAULT_COOLDOWN_SECS, DEFAULT_FAIL_THRESHOLD, DEFAULT_INTERVAL_SECS,
    DEFAULT_MAX_AGE_SECS,
};
use gsv::{DEFAULT_HOST, DEFAULT_PORT};

struct Cfg {
    host: String,
    port: u16,
    interval: u64,
    threshold: u32,
    cooldown: u64,
    repo_root: PathBuf,
    once: bool,
}

fn parse_args() -> Cfg {
    let mut host = DEFAULT_HOST.to_string();
    let mut port = DEFAULT_PORT;
    let mut interval = DEFAULT_INTERVAL_SECS;
    let mut threshold = DEFAULT_FAIL_THRESHOLD;
    let mut cooldown = DEFAULT_COOLDOWN_SECS;
    let mut repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut once = false;
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
            "--interval" => interval = args.next().and_then(|v| v.parse().ok()).unwrap_or(interval),
            "--fail-threshold" => {
                threshold = args
                    .next()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(threshold)
            }
            "--cooldown" => cooldown = args.next().and_then(|v| v.parse().ok()).unwrap_or(cooldown),
            "--repo-root" => {
                if let Some(p) = args.next() {
                    repo_root = PathBuf::from(p);
                }
            }
            "--once" => once = true,
            "--help" | "-h" => {
                println!(
                    "Usage: gsv-watchdog [--host H] [--port N] [--interval S] [--fail-threshold N] [--cooldown S] [--repo-root P] [--once]"
                );
                std::process::exit(0);
            }
            _ => {}
        }
    }
    Cfg {
        host,
        port,
        interval,
        threshold,
        cooldown,
        repo_root,
        once,
    }
}

async fn probe(client: &reqwest::Client, url: &str) -> bool {
    match client.get(url).send().await {
        Ok(res) => {
            let status = res.status().as_u16();
            let body = res.text().await.unwrap_or_default();
            watchdog::parse_health_ok(status, &body)
        }
        Err(_) => false,
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = parse_args();
    let url = watchdog::health_url(&cfg.host, cfg.port);
    let hb_path = heartbeat_path(&cfg.repo_root);
    let now = epoch_now();
    if let Some(existing) = read_heartbeat(&hb_path) {
        if heartbeat_fresh(&existing, now, DEFAULT_MAX_AGE_SECS)
            && existing.pid != std::process::id()
            && !cfg.once
        {
            eprintln!(
                "gsv-watchdog: already running (pid {} heartbeat {}s old)",
                existing.pid,
                now.saturating_sub(existing.epoch_secs)
            );
            return Ok(());
        }
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .no_proxy()
        .build()?;

    let mut failures = 0u32;
    let mut last_respawn = 0u64;
    loop {
        let ok = probe(&client, &url).await;
        let t = watchdog::tick(failures, ok, cfg.threshold);
        failures = t.consecutive_failures;
        let now = epoch_now();
        let mut action = if ok { "probe-ok" } else { "probe-fail" };
        if t.respawn
            && watchdog::should_respawn(failures, cfg.threshold, last_respawn, now, cfg.cooldown)
        {
            match watchdog::spawn_live(&cfg.repo_root, &cfg.host, cfg.port) {
                Ok(SpawnOutcome::Spawned) => {
                    action = "respawn";
                    last_respawn = now;
                    eprintln!(
                        "gsv-watchdog: respawned live copy on {}:{}",
                        cfg.host, cfg.port
                    );
                }
                Ok(SpawnOutcome::MissingDebug) => {
                    action = "missing-debug";
                    eprintln!(
                        "gsv-watchdog: missing target/debug/{}",
                        watchdog::server_exe_name()
                    );
                }
                Ok(SpawnOutcome::HarnessSkipped) => action = "harness",
                Err(e) => {
                    action = "spawn-err";
                    eprintln!("gsv-watchdog: spawn failed: {e}");
                }
            }
        }
        let hb = Heartbeat {
            ts: gsv::vision::rfc3339_now(),
            epoch_secs: now,
            pid: std::process::id(),
            last_ok: ok,
            consecutive_failures: failures,
            last_action: action.into(),
            host: cfg.host.clone(),
            port: cfg.port,
        };
        if let Err(e) = write_heartbeat(&hb_path, &hb) {
            eprintln!("gsv-watchdog: heartbeat: {e}");
        }
        if cfg.once {
            break;
        }
        tokio::time::sleep(Duration::from_secs(cfg.interval.max(1))).await;
    }
    Ok(())
}
