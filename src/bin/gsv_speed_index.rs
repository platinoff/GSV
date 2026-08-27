//! Speed index for `cargo test` wall times and Criterion medians.
//!
//! Canonical JSON: [`docs/vision/speed_index.json`](../../docs/vision/speed_index.json)
//! (Galaxy Speed Index panel; `gsv-vision-sync` mirrors to `data/gsv_speed_index.json`).
//!
//! ```text
//! cargo run --bin gsv-speed-index -- --print
//! cargo run --bin gsv-speed-index -- --record-test --wall-secs 120 --ok
//! cargo run --bin gsv-speed-index -- --record-bench --bench runtime --group x --median-ns 1280
//! ```

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const DEFAULT_OUTPUT: &str = "docs/vision/speed_index.json";
const HISTORY_TEST_CAP: usize = 24;
const HISTORY_BENCH_CAP: usize = 80;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct SpeedIndex {
    #[serde(default)]
    schema_version: u32,
    #[serde(default)]
    generated_at: String,
    #[serde(default)]
    host_label: String,
    #[serde(default)]
    git_head: String,
    #[serde(default)]
    notes: Vec<String>,
    #[serde(default)]
    latest: LatestSpeeds,
    #[serde(default)]
    test_ci_history: Vec<TestEntry>,
    #[serde(default)]
    bench_history: Vec<BenchEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct LatestSpeeds {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    test_ci_wall_secs: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    test_ci_ok: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    test_ci_recorded_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    test_ci_command: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    last_bench_label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    last_bench_median_ns: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    last_bench_recorded_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestEntry {
    kind: String,
    command: String,
    wall_secs: f64,
    ok: bool,
    recorded_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    host_label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    git_head: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BenchEntry {
    kind: String,
    bench: String,
    group: String,
    median_ns: u64,
    #[serde(default)]
    profile: String,
    recorded_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    host_label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    git_head: Option<String>,
}

fn repo_root() -> PathBuf {
    env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

fn git_head_short(root: &Path) -> String {
    gsv::vision::git_head_short(root)
}

fn load_index(path: &Path) -> SpeedIndex {
    match fs::read_to_string(path) {
        Ok(raw) => serde_json::from_str(&raw).unwrap_or_default(),
        Err(_) => SpeedIndex {
            schema_version: 1,
            notes: vec![
                "Machine-readable wall-clock index for cargo test + Criterion medians.".into(),
                "Record via cargo xtask record-speed (абракадабра drain).".into(),
            ],
            ..Default::default()
        },
    }
}

fn write_index(path: &Path, index: &SpeedIndex) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let pretty = serde_json::to_string_pretty(index).map_err(|e| e.to_string())? + "\n";
    fs::write(path, pretty).map_err(|e| e.to_string())
}

fn record_test(
    index: &mut SpeedIndex,
    wall_secs: f64,
    ok: bool,
    command: String,
    host: String,
    head: String,
) {
    let now = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    index.schema_version = 1;
    index.generated_at = now[..10].to_string();
    index.host_label = host.clone();
    index.git_head = head.clone();
    index.latest.test_ci_wall_secs = Some(wall_secs);
    index.latest.test_ci_ok = Some(ok);
    index.latest.test_ci_recorded_at = Some(now.clone());
    index.latest.test_ci_command = Some(command.clone());
    index.test_ci_history.push(TestEntry {
        kind: "test_ci".into(),
        command,
        wall_secs,
        ok,
        recorded_at: now,
        host_label: Some(host),
        git_head: Some(head),
    });
    if index.test_ci_history.len() > HISTORY_TEST_CAP {
        let drop_n = index.test_ci_history.len() - HISTORY_TEST_CAP;
        index.test_ci_history.drain(0..drop_n);
    }
}

fn record_bench(
    index: &mut SpeedIndex,
    bench: String,
    group: String,
    median_ns: u64,
    profile: String,
    host: String,
    head: String,
) {
    let now = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    index.schema_version = 1;
    index.generated_at = now[..10].to_string();
    index.host_label = host.clone();
    index.git_head = head.clone();
    let label = format!("{bench}::{group}");
    index.latest.last_bench_label = Some(label);
    index.latest.last_bench_median_ns = Some(median_ns);
    index.latest.last_bench_recorded_at = Some(now.clone());
    index.bench_history.push(BenchEntry {
        kind: "criterion".into(),
        bench,
        group,
        median_ns,
        profile,
        recorded_at: now,
        host_label: Some(host),
        git_head: Some(head),
    });
    if index.bench_history.len() > HISTORY_BENCH_CAP {
        let drop_n = index.bench_history.len() - HISTORY_BENCH_CAP;
        index.bench_history.drain(0..drop_n);
    }
}

fn print_summary(index: &SpeedIndex) {
    println!("speed_index schema={}", index.schema_version.max(1));
    println!("generated_at={}", index.generated_at);
    println!("host={}", index.host_label);
    println!("git_head={}", index.git_head);
    match (
        index.latest.test_ci_wall_secs,
        index.latest.test_ci_ok,
        &index.latest.test_ci_recorded_at,
    ) {
        (Some(secs), Some(ok), Some(at)) => {
            println!(
                "latest test: {secs:.1}s ok={ok} at={at} cmd={}",
                index
                    .latest
                    .test_ci_command
                    .as_deref()
                    .unwrap_or("cargo test")
            );
        }
        _ => println!("latest test: (none)"),
    }
    match (
        &index.latest.last_bench_label,
        index.latest.last_bench_median_ns,
        &index.latest.last_bench_recorded_at,
    ) {
        (Some(label), Some(ns), Some(at)) => {
            println!("latest bench: {label} median_ns={ns} at={at}");
        }
        _ => println!("latest bench: (none)"),
    }
    println!(
        "history: test_ci={} bench={}",
        index.test_ci_history.len(),
        index.bench_history.len()
    );
}

fn usage() {
    eprintln!(
        "Usage:
  gsv-speed-index --print [--output PATH]
  gsv-speed-index --record-test --wall-secs SECS [--ok|--fail] [--command CMD] [--host LABEL] [--output PATH]
  gsv-speed-index --record-bench --bench NAME --group GROUP --median-ns N [--profile short] [--host LABEL] [--output PATH]"
    );
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() || args.iter().any(|a| a == "-h" || a == "--help") {
        usage();
        return ExitCode::SUCCESS;
    }

    let root = repo_root();
    let mut output = root.join(DEFAULT_OUTPUT);
    let mut wall_secs: Option<f64> = None;
    let mut ok = true;
    let mut command = "cargo test".to_string();
    let mut host = env::var("COMPUTERNAME")
        .or_else(|_| env::var("HOSTNAME"))
        .unwrap_or_else(|_| "local".into());
    let mut mode_print = false;
    let mut mode_test = false;
    let mut mode_bench = false;
    let mut bench_name: Option<String> = None;
    let mut group: Option<String> = None;
    let mut median_ns: Option<u64> = None;
    let mut profile = "short".to_string();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--print" => mode_print = true,
            "--record-test" | "--record-test-ci" => mode_test = true,
            "--record-bench" => mode_bench = true,
            "--ok" => ok = true,
            "--fail" => ok = false,
            "--wall-secs" => {
                i += 1;
                wall_secs = args.get(i).and_then(|s| s.parse().ok());
            }
            "--command" => {
                i += 1;
                if let Some(v) = args.get(i) {
                    command = v.clone();
                }
            }
            "--host" => {
                i += 1;
                if let Some(v) = args.get(i) {
                    host = v.clone();
                }
            }
            "--output" => {
                i += 1;
                if let Some(v) = args.get(i) {
                    output = root.join(v);
                }
            }
            "--bench" => {
                i += 1;
                bench_name = args.get(i).cloned();
            }
            "--group" => {
                i += 1;
                group = args.get(i).cloned();
            }
            "--median-ns" => {
                i += 1;
                median_ns = args.get(i).and_then(|s| s.parse().ok());
            }
            "--profile" => {
                i += 1;
                if let Some(v) = args.get(i) {
                    profile = v.clone();
                }
            }
            other => {
                eprintln!("unknown arg: {other}");
                usage();
                return ExitCode::FAILURE;
            }
        }
        i += 1;
    }

    let head = git_head_short(&root);
    let mut index = load_index(&output);

    if mode_test {
        let Some(secs) = wall_secs else {
            eprintln!("--record-test requires --wall-secs");
            return ExitCode::FAILURE;
        };
        record_test(&mut index, secs, ok, command, host, head);
        if let Err(e) = write_index(&output, &index) {
            eprintln!("write failed: {e}");
            return ExitCode::FAILURE;
        }
        print_summary(&index);
        return ExitCode::SUCCESS;
    }

    if mode_bench {
        let (Some(b), Some(g), Some(ns)) = (bench_name, group, median_ns) else {
            eprintln!("--record-bench requires --bench --group --median-ns");
            return ExitCode::FAILURE;
        };
        record_bench(&mut index, b, g, ns, profile, host, head);
        if let Err(e) = write_index(&output, &index) {
            eprintln!("write failed: {e}");
            return ExitCode::FAILURE;
        }
        print_summary(&index);
        return ExitCode::SUCCESS;
    }

    if !output.exists() && !mode_print {
        index.schema_version = 1;
        index.generated_at = Utc::now().format("%Y-%m-%d").to_string();
        index.host_label = host;
        index.git_head = head;
        let _ = write_index(&output, &index);
    }
    print_summary(&index);
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_test_updates_latest_and_caps_history() {
        let mut idx = SpeedIndex::default();
        for i in 0..30 {
            record_test(
                &mut idx,
                100.0 + i as f64,
                true,
                "cargo test".into(),
                "host".into(),
                "abc".into(),
            );
        }
        assert_eq!(idx.test_ci_history.len(), HISTORY_TEST_CAP);
        assert_eq!(idx.latest.test_ci_wall_secs, Some(129.0));
        assert_eq!(idx.latest.test_ci_ok, Some(true));
    }

    #[test]
    fn record_bench_updates_latest() {
        let mut idx = SpeedIndex::default();
        record_bench(
            &mut idx,
            "runtime".into(),
            "x".into(),
            42,
            "short".into(),
            "host".into(),
            "def".into(),
        );
        assert_eq!(idx.bench_history.len(), 1);
        assert_eq!(idx.latest.last_bench_median_ns, Some(42));
        assert!(idx
            .latest
            .last_bench_label
            .as_deref()
            .unwrap_or("")
            .contains("runtime"));
    }
}
