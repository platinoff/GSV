//! Light memory, disk, and speed app (`gsv-mds`).
//!
//! ```text
//! cargo run --bin gsv-mds
//! cargo run --bin gsv-mds -- --json
//! ```

use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn main() -> ExitCode {
    let json = env::args().any(|a| a == "--json" || a == "-j");
    let root = repo_root();
    let report = gsv::boxes::mds::report(&root);
    if json {
        match serde_json::to_string_pretty(&report) {
            Ok(s) => println!("{s}"),
            Err(e) => {
                eprintln!("gsv-mds: encode failed: {e}");
                return ExitCode::FAILURE;
            }
        }
    } else {
        println!(
            "gsv-mds ok={} mem sample={}B alloc_ns={} avail={:?} disk free_mb={:?} target_mb={} speed {} ns/iter",
            report.ok,
            report.memory.sample_bytes,
            report.memory.alloc_ns,
            report.memory.avail_bytes,
            report.disk.free_mb,
            report.disk.target_mb,
            report.speed.ns_per_iter
        );
    }
    if report.ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
