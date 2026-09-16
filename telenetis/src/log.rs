//! Live log capture (bug-hunt 2).
//!
//! The always-on `:9800` process runs detached (no console), so stdout
//! tracing was lost and live debugging ran blind. This module adds a
//! rolling file sink next to stdout: `target/live/logs/telenetis.log`
//! (daily rotation, keep [`KEEP_LOG_FILES`]), resolved from
//! `TELENETIS_LOG_DIR` or the compiled crate dir — never the process CWD,
//! which the supervisor/VBS launchers do not guarantee.

use std::path::PathBuf;

/// Rolled log files to keep (tracing-appender prunes older ones).
pub const KEEP_LOG_FILES: usize = 7;

/// Rolling file sink directory: explicit env wins, else the crate's
/// `target/live/logs` (sits next to the live exe the supervisor runs and
/// respawns; gitignored scratch, never staged).
pub fn log_dir() -> PathBuf {
    std::env::var_os("TELENETIS_LOG_DIR")
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/live/logs"))
}

/// Install stdout + rolling-file tracing. The returned guard must stay
/// alive for the process lifetime (dropping it flushes and stops the
/// file sink) — main holds it to the end of `main`.
pub fn init_logging() -> tracing_appender::non_blocking::WorkerGuard {
    use tracing_subscriber::{fmt, prelude::__tracing_subscriber_SubscriberExt, EnvFilter};

    let dir = log_dir();
    let _ = std::fs::create_dir_all(&dir);
    let appender = tracing_appender::rolling::RollingFileAppender::builder()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_prefix("telenetis.log")
        .max_log_files(KEEP_LOG_FILES)
        .build(&dir)
        .unwrap_or_else(|_| {
            // A wedged log dir must never take the service down —
            // fall back to temp (still better than silent stdout).
            tracing_appender::rolling::daily(std::env::temp_dir(), "telenetis-fallback.log")
        });
    let (file_writer, guard) = tracing_appender::non_blocking(appender);
    let subscriber = fmt()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive("telenetis=debug".parse().unwrap()),
        )
        .finish()
        .with(fmt::layer().with_writer(file_writer).with_ansi(false));
    if tracing::subscriber::set_global_default(subscriber).is_err() {
        eprintln!("tracing global default already set — file sink skipped");
    }
    tracing::info!("telenetis file log at {}", dir.display());
    guard
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_dir_defaults_next_to_live_exe() {
        let _guard = crate::ui::vendor::ENV_GUARD.blocking_lock();
        std::env::remove_var("TELENETIS_LOG_DIR");
        let dir = log_dir();
        assert!(dir.ends_with("target/live/logs") || dir.ends_with("target\\live\\logs"));
        assert!(dir.to_string_lossy().contains("telenetis"));
    }

    #[test]
    fn log_dir_env_override_wins() {
        let _guard = crate::ui::vendor::ENV_GUARD.blocking_lock();
        std::env::set_var("TELENETIS_LOG_DIR", "/tmp/tns-logs");
        assert_eq!(log_dir(), PathBuf::from("/tmp/tns-logs"));
        std::env::remove_var("TELENETIS_LOG_DIR");
    }

    #[test]
    fn log_dir_lives_next_to_live_exe() {
        let _guard = crate::ui::vendor::ENV_GUARD.blocking_lock();
        std::env::remove_var("TELENETIS_LOG_DIR");
        let dir = log_dir();
        let lossy = dir.to_string_lossy().into_owned();
        assert!(lossy.contains("live"), "{lossy}");
        assert!(lossy.ends_with("logs") || lossy.ends_with("logs/"));
    }
}
