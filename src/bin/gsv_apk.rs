//! Phone APK client contract (`gsv-apk`).
//!
//! Prints disk + identity, or the register JSON the native APK posts to
//! hub `/api/edge`. Never dials `:8091`. Never prints the edge token.
//!
//! ```text
//! cargo run --bin gsv-apk
//! cargo run --bin gsv-apk -- --json
//! cargo run --bin gsv-apk -- register --json
//! cargo run --bin gsv-apk -- check-hub http://192.168.2.238:9999
//! cargo run --bin gsv-apk -- telegram auth --json
//! cargo run --bin gsv-apk -- service-account --json
//! ```

use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use gsv::boxes::apk::{self, CLASS_EDGE, ORIGIN, ROLE};
use serde_json::{json, Value};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn want_json(args: &[String]) -> bool {
    args.iter().any(|a| a == "--json" || a == "-j")
}

fn print_json(v: &Value) -> ExitCode {
    match serde_json::to_string_pretty(v) {
        Ok(s) => {
            println!("{s}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("gsv-apk: encode failed: {e}");
            ExitCode::FAILURE
        }
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let json = want_json(&args);
    let root = repo_root();
    let hub = apk::hub_from_env();
    let peer = apk::peer_id();

    if args.iter().any(|a| a == "check-hub") {
        let url = args
            .iter()
            .find(|a| a.starts_with("http://") || a.starts_with("https://"))
            .cloned()
            .unwrap_or_else(|| hub.clone());
        return match apk::check_hub(&url) {
            Ok(()) => {
                if json {
                    return print_json(&json!({ "ok": true, "hub": url }));
                }
                println!("gsv-apk check-hub ok hub={url}");
                ExitCode::SUCCESS
            }
            Err(e) => {
                if json {
                    let _ = print_json(&json!({ "ok": false, "hub": url, "error": e.error }));
                    return ExitCode::FAILURE;
                }
                eprintln!("gsv-apk check-hub fail hub={url} error={}", e.error);
                ExitCode::FAILURE
            }
        };
    }

    if args.iter().any(|a| a == "register") {
        let disk = gsv::boxes::xtask::disk_report(&root, false);
        match apk::check_hub(&hub) {
            Ok(()) => {}
            Err(e) => {
                eprintln!("gsv-apk register fail hub={hub} error={}", e.error);
                return ExitCode::FAILURE;
            }
        }
        let addr = env::var("GSV_APK_ADDR")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "192.168.2.89".into());
        let mut urls = Vec::new();
        for rel in apk::register_paths(&peer) {
            match apk::edge_url(&hub, &rel) {
                Ok(u) => urls.push(u),
                Err(e) => {
                    eprintln!("gsv-apk register fail path={rel} error={}", e.error);
                    return ExitCode::FAILURE;
                }
            }
        }
        let payload = json!({
            "ok": true,
            "hub": hub,
            "peer_id": peer,
            "origin": ORIGIN,
            "role": ROLE,
            "class": CLASS_EDGE,
            "token_header": apk::token_header(),
            "urls": urls,
            "register": apk::registration_body(&peer, &addr, 0, &disk),
            "join": apk::pool_join_body(),
        });
        if json {
            return print_json(&payload);
        }
        println!(
            "gsv-apk register origin={ORIGIN} role={ROLE} class={CLASS_EDGE} peer={peer} hub={hub} urls={}",
            urls.join(" ")
        );
        return ExitCode::SUCCESS;
    }

    if args.iter().any(|a| a == "telegram") {
        let kind = args
            .iter()
            .find(|a| {
                let s = a.as_str();
                s != "telegram"
                    && s != "register"
                    && s != "check-hub"
                    && s != "service-account"
                    && s != "--json"
                    && s != "-j"
                    && !s.starts_with("http://")
                    && !s.starts_with("https://")
            })
            .cloned()
            .unwrap_or_default();
        return match apk::telegram_passthrough(&kind) {
            Ok(fwd) => {
                if json {
                    match serde_json::to_value(&fwd) {
                        Ok(v) => print_json(&v),
                        Err(e) => {
                            eprintln!("gsv-apk: encode failed: {e}");
                            ExitCode::FAILURE
                        }
                    }
                } else {
                    println!(
                        "gsv-apk telegram ok kind={} to={} origin={} action={}",
                        fwd.kind, fwd.to, fwd.origin, fwd.action
                    );
                    ExitCode::SUCCESS
                }
            }
            Err(e) => {
                if json {
                    let _ = print_json(&json!({
                        "ok": false,
                        "kind": kind,
                        "error": e.error
                    }));
                    ExitCode::FAILURE
                } else {
                    eprintln!("gsv-apk telegram fail kind={kind} error={}", e.error);
                    ExitCode::FAILURE
                }
            }
        };
    }

    if args.iter().any(|a| a == "service-account") {
        let v = gsv::boxes::edge::service_account_wire();
        if json {
            return print_json(&v);
        }
        println!(
            "gsv-apk service-account kind={} header={} hub={} login_blocked=true",
            v["kind"].as_str().unwrap_or("edge_token"),
            v["header"].as_str().unwrap_or("x-gsv-edge-token"),
            v["hub"].as_str().unwrap_or("/api/edge")
        );
        return ExitCode::SUCCESS;
    }

    let r = apk::report(&root, &hub, &peer);
    if json {
        match serde_json::to_string_pretty(&r) {
            Ok(s) => println!("{s}"),
            Err(e) => {
                eprintln!("gsv-apk: encode failed: {e}");
                return ExitCode::FAILURE;
            }
        }
    } else {
        println!(
            "gsv-apk ok={} origin={} role={} class={} peer={} hub={} wifi_debug={} disk_ok={} free_mb={:?} model_cache=none",
            r.ok,
            r.identity.origin,
            r.identity.role,
            r.identity.class,
            r.identity.peer_id,
            r.hub,
            r.settings.wifi_debug,
            r.disk.ok,
            r.disk.free_mb
        );
    }
    if r.ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
