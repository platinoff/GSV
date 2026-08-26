//! `gsv-xtask` — product automation in Rust (`cargo xtask <task>`).
//!
//! Replaces `scripts/*.sh` and `bin/*.sh`. Tests, benches, and kit scripts
//! live in `.rs`. JSON is data (vision snapshots, MCP client configs), not
//! a harness.

use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use gsv::boxes::fingerprint;
use gsv::boxes::vision;
use gsv::boxes::xtask;
use gsv::{DEFAULT_HOST, DEFAULT_PORT};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn main() -> ExitCode {
    let mut args = env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty()
        || args
            .iter()
            .any(|a| a == "--help" || a == "-h" || a == "help")
    {
        print!("{}", xtask::help_text());
        return ExitCode::SUCCESS;
    }
    let cmd = args.remove(0);
    let root = repo_root();
    match cmd.as_str() {
        "catalog" => {
            println!(
                "{}",
                serde_json::to_string_pretty(&xtask::catalog_wire()).unwrap_or_default()
            );
            ExitCode::SUCCESS
        }
        "products" => {
            print!("{}", xtask::products_tsv(&root));
            ExitCode::SUCCESS
        }
        "disk" => {
            let enforce = args.iter().any(|a| a == "--enforce")
                || env::var("GSV_ENFORCE_DISK_LIMIT").as_deref() == Ok("1");
            let do_clean = args.iter().any(|a| a == "--clean");
            let target_dir = std::env::var("CARGO_TARGET_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|_| root.join("target"));
            if do_clean {
                let c = xtask::clean_debug_cache(&target_dir);
                eprintln!(
                    "check_target_disk: clean removed [{}] kept_live={}",
                    c.removed.join(", "),
                    c.kept_live
                );
            }
            let r = xtask::disk_report(&root, enforce);
            let free = match (r.free_gb, r.free_mb) {
                (Some(0), Some(mb)) => format!("{mb} MiB"),
                (Some(gb), Some(mb)) if gb < 2 => format!("{mb} MiB"),
                (Some(gb), _) => format!("{gb} GiB"),
                _ => "?".into(),
            };
            eprintln!(
                "check_target_disk: repo={} target_dir={} (size ~{} GiB / {} MiB) free ~{} (min {} GiB)",
                r.repo,
                r.target_dir,
                r.target_gb,
                r.target_mb,
                free,
                r.min_free_gb
            );
            for n in &r.notes {
                eprintln!("check_target_disk: {n}");
            }
            if r.ok {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        "live" => {
            let host = env::var("GSV_HOST").unwrap_or_else(|_| DEFAULT_HOST.into());
            let port = env::var("GSV_PORT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(DEFAULT_PORT);
            match xtask::run_live(&root, &host, port) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("gsv-live: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        "watchdog" => match xtask::detach_watchdog(&root) {
            Ok(m) => {
                println!("{m}");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("gsv-watchdog: {e}");
                ExitCode::FAILURE
            }
        },
        "watchdog-install" => match xtask::install_watchdog(&root) {
            Ok(m) => {
                println!("{m}");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("gsv-watchdog-install: {e}");
                ExitCode::FAILURE
            }
        },
        "push" => match xtask::git_push_only(&root) {
            Ok(m) => {
                print!("{m}");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprint!("{e}");
                ExitCode::FAILURE
            }
        },
        "git" => match xtask::git_cli(&root, &args) {
            Ok(m) => {
                print!("{m}");
                if !m.ends_with('\n') {
                    println!();
                }
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprint!("{e}");
                ExitCode::FAILURE
            }
        },
        "tunnel" => {
            let mut host = env::var("GSV_HOST").unwrap_or_else(|_| DEFAULT_HOST.into());
            let mut port = env::var("GSV_PORT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(DEFAULT_PORT);
            let mut i = 0;
            while i < args.len() {
                if args[i] == "--port" {
                    i += 1;
                    if let Some(v) = args.get(i).and_then(|s| s.parse().ok()) {
                        port = v;
                    }
                } else if args[i] == "--host" {
                    i += 1;
                    if let Some(v) = args.get(i) {
                        host = v.clone();
                    }
                }
                i += 1;
            }
            match xtask::tunnel_cli(&host, port) {
                Ok(m) => {
                    println!("{m}");
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("gsv-tunnel: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        "mirrors" => match xtask::sync_skill_mirrors(&root) {
            Ok(names) => {
                for n in names {
                    println!("mirrored {n}");
                }
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("mirrors: {e}");
                ExitCode::FAILURE
            }
        },
        "bump" => {
            let mut band: Option<u32> = env::var("GSV_BAND").ok().and_then(|s| s.parse().ok());
            let mut toml = root.join("Cargo.toml");
            let mut i = 0;
            while i < args.len() {
                if args[i] == "--band" {
                    i += 1;
                    band = args.get(i).and_then(|s| s.parse().ok());
                } else if !args[i].starts_with('-') {
                    toml = PathBuf::from(&args[i]);
                }
                i += 1;
            }
            let Some(band) = band else {
                eprintln!("gsv-bump-version: missing --band (or GSV_BAND)");
                return ExitCode::FAILURE;
            };
            match fingerprint::bump_package_version(&toml, band) {
                Ok(ver) => match vision::lockstep_queue_for_band(&root, band) {
                    Ok((last, next)) => {
                        println!("gsv-bump-version: {ver}");
                        println!("gsv-bump-version: queue last {last} next {next}");
                        ExitCode::SUCCESS
                    }
                    Err(e) => {
                        eprintln!("gsv-bump-version: {ver} (queue lockstep failed: {e})");
                        ExitCode::FAILURE
                    }
                },
                Err(e) => {
                    eprintln!("gsv-bump-version: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        "fingerprint" => {
            let mut model: Option<String> = None;
            let mut i = 0;
            while i < args.len() {
                if args[i] == "--model" {
                    i += 1;
                    model = args.get(i).cloned();
                }
                i += 1;
            }
            match fingerprint::record_from_env(&root, None, None, model.as_deref()) {
                Ok((_, msg)) => {
                    print!("{msg}");
                    let _ = std::io::Write::flush(&mut std::io::stdout());
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("{e}");
                    ExitCode::FAILURE
                }
            }
        }
        "fingerprint-recheck" => {
            let issues = fingerprint::recheck(&root);
            println!("issues: {}", issues.len());
            for issue in &issues {
                println!(
                    "  {} line={} head={:?} {}",
                    issue.kind, issue.line, issue.git_head, issue.detail
                );
            }
            ExitCode::SUCCESS
        }
        "fingerprint-dedup" => match fingerprint::dedup_jsonl(&root) {
            Ok(removed) => {
                println!("dedup: removed {removed} duplicate rows");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("fingerprint-dedup: {e}");
                ExitCode::FAILURE
            }
        },
        "record-speed" => {
            let skip = args.iter().any(|a| a == "--skip-run");
            match xtask::record_speed(&root, skip) {
                Ok(0) => ExitCode::SUCCESS,
                Ok(_) => ExitCode::FAILURE,
                Err(e) => {
                    eprintln!("record-speed: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        "record-rust" => {
            let skip = args.iter().any(|a| a == "--skip-run");
            let ci = args.iter().any(|a| a == "--ci");
            match xtask::record_rust(&root, skip, ci) {
                Ok(0) => ExitCode::SUCCESS,
                Ok(_) => ExitCode::FAILURE,
                Err(e) => {
                    eprintln!("record-rust: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        "record-scenario-bench" => match xtask::record_scenario_bench(&root) {
            Ok(0) => ExitCode::SUCCESS,
            Ok(_) => ExitCode::FAILURE,
            Err(e) => {
                eprintln!("record-scenario-bench: {e}");
                ExitCode::FAILURE
            }
        },
        "sync" => {
            let check = args.iter().any(|a| a == "--check");
            match xtask::vision_sync(&root, check) {
                Ok(m) => {
                    println!("{m}");
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprint!("{e}");
                    ExitCode::FAILURE
                }
            }
        }
        "vault-note" => {
            let mut band: Option<u32> = None;
            let mut title = String::from("Drain Note");
            let mut summary = String::new();
            let mut i = 0;
            while i < args.len() {
                match args[i].as_str() {
                    "--band" => {
                        i += 1;
                        band = args.get(i).and_then(|s| s.parse().ok());
                        i += 1;
                    }
                    flag @ ("--title" | "--summary") => {
                        i += 1;
                        let mut parts: Vec<String> = Vec::new();
                        while i < args.len() && !args[i].starts_with('-') {
                            parts.push(args[i].clone());
                            i += 1;
                        }
                        let joined = parts.join(" ");
                        if !joined.is_empty() {
                            if flag == "--title" {
                                title = joined;
                            } else {
                                summary = joined;
                            }
                        }
                    }
                    _ => i += 1,
                }
            }
            match xtask::vault_note(&root, band, &title, &summary) {
                Ok(m) => {
                    println!("{m}");
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("vault-note: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        other => {
            eprintln!("gsv-xtask: unknown task '{other}'\n{}", xtask::help_text());
            ExitCode::FAILURE
        }
    }
}
