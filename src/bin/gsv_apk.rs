//! Phone APK client contract (`gsv-apk`).
//!
//! Prints disk + identity, or the register JSON the native APK posts to
//! hub `/api/edge`. Never dials `:8091`. Never prints the edge token.
//!
//! ```text
//! cargo run --bin gsv-apk
//! cargo run --bin gsv-apk -- --json
//! cargo run --bin gsv-apk -- register --json
//! cargo run --bin gsv-apk -- join --json
//! cargo run --bin gsv-apk -- join --live --json
//! cargo run --bin gsv-apk -- check-hub http://192.168.2.238:9999
//! cargo run --bin gsv-apk -- telegram auth --json
//! cargo run --bin gsv-apk -- service-account --json
//! cargo run --bin gsv-apk -- package --json
//! cargo run --bin gsv-apk -- package --write
//! cargo run --bin gsv-apk -- disk --json
//! cargo run --bin gsv-apk -- settings --json
//! cargo run --bin gsv-apk -- adb --json
//! cargo run --bin gsv-apk -- freeze --json
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

fn skip_kind(s: &str) -> bool {
    matches!(
        s,
        "telegram"
            | "register"
            | "check-hub"
            | "service-account"
            | "freeze"
            | "join"
            | "package"
            | "disk"
            | "settings"
            | "adb"
            | "--json"
            | "-j"
            | "--live"
            | "--write"
            | "--manifest"
    ) || s.starts_with("http://")
        || s.starts_with("https://")
}

fn block_on<F: std::future::Future<Output = T>, T>(fut: F) -> T {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio")
        .block_on(fut)
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

    if args.iter().any(|a| a == "join") {
        let dry = !args.iter().any(|a| a == "--live");
        let disk = gsv::boxes::xtask::disk_report(&root, false);
        let addr = apk::peer_addr();
        let token = gsv::boxes::edge::env_token();
        let v = match block_on(apk::join_lan(
            &hub,
            &peer,
            &addr,
            &disk,
            dry,
            token.as_deref(),
        )) {
            Ok(v) => v,
            Err(e) => {
                if json {
                    let _ = print_json(&json!({
                        "ok": false,
                        "hub": hub,
                        "dry_run": dry,
                        "error": e.error
                    }));
                    return ExitCode::FAILURE;
                }
                eprintln!("gsv-apk join fail hub={hub} error={}", e.error);
                return ExitCode::FAILURE;
            }
        };
        let ok = v.get("ok").and_then(Value::as_bool) == Some(true);
        if json {
            let code = print_json(&v);
            return if ok { code } else { ExitCode::FAILURE };
        }
        println!(
            "gsv-apk join ok={} dry_run={} origin={} peer={} hub={} token_set={} steps={}",
            ok,
            v["dry_run"],
            ORIGIN,
            peer,
            hub,
            v["token_set"],
            v["steps"].as_array().map(|a| a.len()).unwrap_or(0)
        );
        return if ok {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
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
        let addr = apk::peer_addr();
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
            .find(|a| !skip_kind(a))
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

    if args.iter().any(|a| a == "freeze") {
        let v = apk::telenetis_surface_wire();
        if json {
            return print_json(&v);
        }
        println!(
            "gsv-apk freeze surface={} phone_worker={}",
            v["surface"].as_str().unwrap_or("shell"),
            v["phone_worker"].as_str().unwrap_or("apk_edge")
        );
        return ExitCode::SUCCESS;
    }

    if args.iter().any(|a| a == "package") {
        let v = apk::native_package_wire();
        if args.iter().any(|a| a == "--write") {
            let dir = root.join("target").join("live").join("apk");
            match apk::write_manifest(&dir) {
                Ok(path) => {
                    if json {
                        let mut out = v;
                        out["written"] = json!(path.display().to_string());
                        return print_json(&out);
                    }
                    println!(
                        "gsv-apk package write ok package={} path={}",
                        apk::PACKAGE_ID,
                        path.display()
                    );
                    return ExitCode::SUCCESS;
                }
                Err(e) => {
                    eprintln!("gsv-apk package write fail error={e}");
                    return ExitCode::FAILURE;
                }
            }
        }
        if json {
            return print_json(&v);
        }
        println!(
            "gsv-apk package ok native=true webview=false package={} entry={} gradle=false java=false",
            v["package"].as_str().unwrap_or(apk::PACKAGE_ID),
            v["entry"].as_str().unwrap_or("join_lan")
        );
        return ExitCode::SUCCESS;
    }

    if args.iter().any(|a| a == "disk" || a == "settings") {
        let v = apk::disk_settings_wire(&root);
        if json {
            return print_json(&v);
        }
        println!(
            "gsv-apk {} ok={} wifi_debug=true screenshot=false disk_ok={} free_mb={:?} model_cache={:?} adb_serial={}",
            if args.iter().any(|a| a == "settings") {
                "settings"
            } else {
                "disk"
            },
            v["ok"],
            v["disk"]["ok"],
            v["disk"]["free_mb"],
            v["settings"]["model_cache"],
            v["adb"]["serial"].as_str().unwrap_or("")
        );
        return if v.get("ok").and_then(Value::as_bool) == Some(true) {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        };
    }

    if args.iter().any(|a| a == "adb") {
        let v = apk::adb_plan(&root, &apk::adb_serial());
        if json {
            return print_json(&v);
        }
        println!(
            "gsv-apk adb ok={} wifi_debug=true screenshot=false serial={} bin_exists={}",
            v["ok"],
            v["serial"].as_str().unwrap_or(""),
            v["bin_exists"]
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
