//! `gsv-keep-live-boot-verify` — post-boot smoke check for the always-on kit.
//!
//! Probes the four keep-live peers the way `boxes/keep_live` aggregates them:
//!
//! 1. **GSV**   — `GET {base}/api/health` must be `ok:true` with
//!    `keep_live.gsv.alive == true` (the supervisor is the boot target).
//! 2. **Telenetis** — `GET {telenetis}/health` `ok:true` (`:9800` by default).
//! 3. **llama-rs**  — heartbeat file fresh (age ≤ 60s) at `LLAMA_HEARTBEAT_PATH`.
//! 4. **OmniRoute** — fail-open probe (`ok:true` when up; a down peer is
//!    reported but does not fail the boot — same `disk_ok` style rule).
//!
//! Exit 0 when GSV is alive (1 if the supervisor is down, i.e. `--strict` when
//! any peer is down). A down peer is *graceful*: reported, never a crash.
//!
//! ```text
//! cargo run --bin gsv-keep-live-boot-verify
//! cargo run --bin gsv-keep-live-boot-verify -- --json --strict
//! ```
//!
//! env: `GSV_BASE_URL` (default `http://127.0.0.1:9999`), `GSV_KEEP_LIVE_TELENETIS_URL`,
//! `LLAMA_HEARTBEAT_PATH`, `OMNIROUTE_URL`.

use std::process::ExitCode;

use gsv::boxes::keep_live::{heartbeat_fresh, llama_heartbeat_path, omniroute_url, telenetis_url};
use serde::{Deserialize, Serialize};

const DEFAULT_BASE: &str = "http://127.0.0.1:9999";
const ENV_BASE: &str = "GSV_BASE_URL";

#[derive(Debug, Clone)]
struct Cli {
    json: bool,
    strict: bool,
    base_url: String,
}

fn parse_cli() -> Cli {
    let mut json = false;
    let mut strict = false;
    let mut base_url = std::env::var(ENV_BASE)
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_BASE.to_string());
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--json" => json = true,
            "--strict" => strict = true,
            "--base-url" => {
                if let Some(v) = args.next() {
                    base_url = v;
                }
            }
            "--help" | "-h" => {
                println!(
                    "Usage: gsv-keep-live-boot-verify [--base-url URL] [--json] [--strict]\n\
                     env: GSV_BASE_URL (default {DEFAULT_BASE}), GSV_KEEP_LIVE_TELENETIS_URL, LLAMA_HEARTBEAT_PATH, OMNIROUTE_URL"
                );
                std::process::exit(0);
            }
            _ => {}
        }
    }
    Cli {
        json,
        strict,
        base_url,
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct CheckResult {
    name: String,
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<String>,
}

/// Fetch and parse a JSON wire; a telenetis `/health` may be `{ ok: bool }`
/// shaped or `{ status: "ok" }` — `probe_http_blocking` already accepts both.
/// Reads one block at a time and stops as soon as the accumulated body parses
/// as JSON (the server may keep the connection open, so we must not await EOF).
fn fetch_json(base: &str, path: &str) -> Result<(u16, String), String> {
    let url = format!(
        "{}/{}",
        base.trim_end_matches('/'),
        path.trim_start_matches('/')
    );
    let parsed = url
        .strip_prefix("http://")
        .or_else(|| url.strip_prefix("https://"))
        .ok_or_else(|| format!("{url}: unsupported scheme"))?;
    let host_port = parsed.split('/').next().unwrap_or(parsed);
    let path_part = format!("/{}", parsed.split_once('/').map(|x| x.1).unwrap_or(""));
    let addr = if host_port.contains(':') {
        host_port.to_string()
    } else {
        format!("{host_port}:80")
    };
    let timeout = std::time::Duration::from_millis(3500);
    use std::io::{Read, Write};
    let mut stream = std::net::TcpStream::connect_timeout(
        &addr
            .parse()
            .unwrap_or_else(|_| "127.0.0.1:80".parse().unwrap()),
        timeout,
    )
    .map_err(|e| format!("{url}: connect {e}"))?;
    let _ = stream.set_read_timeout(Some(timeout));
    let _ = stream.set_write_timeout(Some(timeout));
    let req = format!("GET {path_part} HTTP/1.0\r\nHost: {host_port}\r\nConnection: close\r\n\r\n");
    stream
        .write_all(req.as_bytes())
        .map_err(|e| format!("{url}: write {e}"))?;
    let mut buf = Vec::with_capacity(8192);
    let mut chunk = [0u8; 2048];
    let mut status = 0u16;
    // Read one block at a time; the `keep_live` merge makes /api/health slow
    // (~1s: each peer is probed serially), so tolerate the first reads timing
    // out before any body byte arrives.
    let mut empty_reads = 0;
    loop {
        match stream.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                buf.extend_from_slice(&chunk[..n]);
                let text = String::from_utf8_lossy(&buf);
                if let Some(rest) = text.split_once("\r\n\r\n") {
                    status = text
                        .lines()
                        .next()
                        .and_then(|l| l.split_whitespace().nth(1))
                        .and_then(|s| s.parse::<u16>().ok())
                        .unwrap_or(0);
                    if serde_json::from_str::<serde_json::Value>(rest.0).is_ok() {
                        break;
                    }
                }
            }
            Err(_) => {
                empty_reads += 1;
                if empty_reads >= 4 {
                    break; // read timeout or reset: use whatever body we have
                }
            }
        }
    }
    let text = String::from_utf8_lossy(&buf).into_owned();
    if text.is_empty() && status == 0 {
        return Err(format!("{url}: empty response"));
    }
    let status = if status == 0 {
        text.lines()
            .next()
            .and_then(|l| l.split_whitespace().nth(1))
            .and_then(|s| s.parse::<u16>().ok())
            .unwrap_or(0)
    } else {
        status
    };
    Ok((
        status,
        text.split("\r\n\r\n").nth(1).unwrap_or("").to_string(),
    ))
}

fn check_gsv(base: &str) -> CheckResult {
    match fetch_json(base, "/api/health") {
        Err(e) => CheckResult {
            name: "gsv_health".into(),
            ok: false,
            detail: Some(e),
        },
        Ok((status, body)) => {
            let v: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
            let ok_flag = v.get("ok").and_then(serde_json::Value::as_bool) == Some(true);
            let gsv_alive = v
                .get("keep_live")
                .and_then(|k| k.get("gsv"))
                .and_then(|g| g.get("alive"))
                .and_then(serde_json::Value::as_bool)
                == Some(true);
            CheckResult {
                name: "gsv_health".into(),
                ok: status == 200 && ok_flag && gsv_alive,
                detail: Some(format!(
                    "status={status} ok={ok_flag} keep_live.gsv.alive={gsv_alive}{}",
                    v.get("crate_version")
                        .and_then(serde_json::Value::as_str)
                        .map(|v| format!(" version={v}"))
                        .unwrap_or_default()
                )),
            }
        }
    }
}

fn check_telenetis() -> CheckResult {
    let url = telenetis_url();
    let p = gsv::boxes::keep_live::probe_http_blocking(&url);
    CheckResult {
        name: "telenetis_health".into(),
        ok: p.alive,
        detail: Some(format!(
            "{} alive={}{}",
            url,
            p.alive,
            p.version
                .map(|v| format!(" version={v}"))
                .unwrap_or_default()
        )),
    }
}

fn check_llama() -> CheckResult {
    let path = llama_heartbeat_path();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let alive = heartbeat_fresh(&path, now);
    CheckResult {
        name: "llama_heartbeat".into(),
        ok: alive,
        detail: Some(format!("{} alive={}", path.to_string_lossy(), alive)),
    }
}

fn check_omniroute() -> CheckResult {
    let url = omniroute_url();
    let p = gsv::boxes::keep_live::probe_http_blocking(&url);
    // Fail-open: a down OmniRoute is *reported*, not a boot failure.
    CheckResult {
        name: "omniroute_fail_open".into(),
        ok: true,
        detail: Some(format!("{} reachable={}", url, p.alive)),
    }
}

fn run(cli: &Cli) -> (Vec<CheckResult>, bool) {
    let checks = vec![
        check_gsv(&cli.base_url),
        check_telenetis(),
        check_llama(),
        check_omniroute(),
    ];
    // Boot fails when the supervisor (GSV) is down. Peers down are graceful.
    let gsv_ok = checks
        .iter()
        .find(|c| c.name == "gsv_health")
        .map(|c| c.ok)
        .unwrap_or(false);
    let all_strict = cli.strict
        && checks
            .iter()
            .any(|c| c.name != "gsv_health" && c.name != "omniroute_fail_open" && !c.ok);
    (checks, gsv_ok && !all_strict)
}

fn main() -> ExitCode {
    let cli = parse_cli();
    let (checks, ok) = run(&cli);
    if cli.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "base_url": cli.base_url,
                "ok": ok,
                "strict": cli.strict,
                "checks": checks,
                "tool": "gsv-keep-live-boot-verify",
            }))
            .expect("encode")
        );
    } else {
        for c in &checks {
            let mark = if c.ok { "ok" } else { "FAIL" };
            println!(
                "[{mark}] {}{}",
                c.name,
                c.detail
                    .as_deref()
                    .map(|d| format!(" — {d}"))
                    .unwrap_or_default()
            );
        }
        println!(
            "gsv-keep-live-boot-verify: {} ok / {} fail (base {}){}",
            checks.iter().filter(|c| c.ok).count(),
            checks.iter().filter(|c| !c.ok).count(),
            cli.base_url,
            if cli.strict { " [strict]" } else { "" }
        );
    }
    if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn omniroute_is_always_graceful() {
        // Even pointing at a dead port, the fail-open check reports ok with detail.
        std::env::set_var("OMNIROUTE_URL", "http://127.0.0.1:59991");
        let c = check_omniroute();
        assert!(c.ok);
        assert!(c
            .detail
            .as_deref()
            .unwrap_or("")
            .contains("reachable=false"));
        std::env::remove_var("OMNIROUTE_URL");
    }

    #[test]
    fn llama_missing_file_is_fail_graceful() {
        std::env::set_var(
            "LLAMA_HEARTBEAT_PATH",
            "C:/definitely/not/a/heartbeat/llama_heartbeat.json",
        );
        let c = check_llama();
        assert!(!c.ok);
        std::env::remove_var("LLAMA_HEARTBEAT_PATH");
    }
}
