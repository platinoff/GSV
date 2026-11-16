//! Live log capture (always-on `gsv-server` and `gsv-mcp`).
//!
//! Mirrors telenetis' rolling file sink: `target/live/logs/<prefix>.log.<date>`
//! (daily rotation, keep [`KEEP_LOG_FILES`]), resolved from `GSV_LOG_DIR` or the
//! crate root — never from CWD (the VBS/watchdog launchers do not guarantee it).
//!
//! * `gsv-server` writes stdout + file (server diagnostics kept for ops);
//! * `gsv-mcp` writes stderr + file (stdio is the protocol channel — stdout
//!   would corrupt it — stderr stays for client consoles that surface it).
//!
//! The returned guard must be held for the process lifetime (it flushes the
//! file sink on drop); `main` holds it to the end of `main`.

use std::path::PathBuf;

/// Rolling log files to keep.
pub const KEEP_LOG_FILES: usize = 7;

/// Rolling file sink directory: explicit env wins, else the crate's
/// `target/live/logs` (gitignored scratch, sits next to the live exe).
pub fn log_dir() -> PathBuf {
    std::env::var_os("GSV_LOG_DIR")
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/live/logs"))
}

/// Install console + rolling-file tracing for a binary; returns the file-sink
/// guard (kept for the process lifetime). `stderr` selects the console sink.
pub fn init_logging_named(
    prefix: &str,
    default_directive: &str,
    stderr: bool,
) -> tracing_appender::non_blocking::WorkerGuard {
    use tracing_subscriber::{fmt, prelude::__tracing_subscriber_SubscriberExt, EnvFilter};

    let dir = log_dir();
    let _ = std::fs::create_dir_all(&dir);
    let appender = tracing_appender::rolling::RollingFileAppender::builder()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_prefix(prefix)
        .max_log_files(KEEP_LOG_FILES)
        .build(&dir)
        .unwrap_or_else(|_| {
            // A wedged log dir must never take the service down —
            // fall back to temp (still better than silent stdout).
            tracing_appender::rolling::daily(std::env::temp_dir(), "gsv-fallback.log")
        });
    let (file_writer, guard) = tracing_appender::non_blocking(appender);
    let make_filter = || {
        std::env::var("RUST_LOG")
            .ok()
            .and_then(|_| EnvFilter::try_from_default_env().ok())
            .unwrap_or_else(|| EnvFilter::new(default_directive))
    };
    if stderr {
        let subscriber = fmt()
            .with_env_filter(make_filter())
            .with_writer(std::io::stderr)
            .with_ansi(true)
            .finish()
            .with(fmt::layer().with_writer(file_writer).with_ansi(false));
        if tracing::subscriber::set_global_default(subscriber).is_err() {
            eprintln!("tracing global default already set — rolling file sink skipped");
        }
    } else {
        let subscriber = fmt()
            .with_env_filter(make_filter())
            .with_ansi(true)
            .finish()
            .with(fmt::layer().with_writer(file_writer).with_ansi(false));
        if tracing::subscriber::set_global_default(subscriber).is_err() {
            eprintln!("tracing global default already set — rolling file sink skipped");
        }
    }
    tracing::info!(dir = %dir.display(), file = prefix, "gsv file log");
    guard
}

/// `gsv-server`: stdout + rolling `target/live/logs/gsv-server.log*`.
pub fn init_logging() -> tracing_appender::non_blocking::WorkerGuard {
    init_logging_named("gsv-server.log", "info,gsv=debug", false)
}

/// `gsv-mcp`: stderr + rolling `target/live/logs/gsv-mcp.log*` (stdio is the
/// protocol channel — must stay clean).
pub fn init_logging_mcp() -> tracing_appender::non_blocking::WorkerGuard {
    init_logging_named("gsv-mcp.log", "warn,gsv=info", true)
}
