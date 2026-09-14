//! Local-address resolver for LAN-first service URLs (band 233 — mirrors
//! `gsv::net::local_addr` in the sibling kit crate; telenetis stays a
//! standalone workspace, so the ~20 lines are copied rather than depended on).
//!
//! Order: `GSV_LOCAL_ADDR` env → `127.0.0.1` under the cargo-test harness
//! (deterministic contracts) → outbound LAN IPv4 (`edge::local_lan_ip`) →
//! loopback fallback.

use std::env;

/// Whether the running exe is a rustc test-harness artifact (`deps/`).
pub fn is_cargo_test_harness() -> bool {
    env::current_exe()
        .ok()
        .map(|p| p.to_string_lossy().replace('\\', "/").contains("/deps/"))
        .unwrap_or(false)
}

/// Env form of the override: a trimmed non-empty value wins; blank is
/// treated as unset (mirrors `gsv::net`). Pure so tests never `set_var`.
fn from_env_value(v: Option<String>) -> Option<String> {
    let t = v?.trim().to_string();
    if t.is_empty() {
        None
    } else {
        Some(t)
    }
}

/// This machine's local (LAN) address as a bare host string.
pub fn local_addr() -> String {
    if let Some(v) = from_env_value(env::var("GSV_LOCAL_ADDR").ok()) {
        return v;
    }
    if is_cargo_test_harness() {
        return "127.0.0.1".to_string();
    }
    crate::edge::local_lan_ip().unwrap_or_else(|| "127.0.0.1".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_form_trims_and_rejects_blank() {
        assert_eq!(from_env_value(None), None);
        assert_eq!(from_env_value(Some("  ".into())), None);
        assert_eq!(
            from_env_value(Some(" 10.11.12.13 ".into())),
            Some("10.11.12.13".into())
        );
    }

    #[test]
    fn harness_resolution_is_loopback() {
        if from_env_value(env::var("GSV_LOCAL_ADDR").ok()).is_some() {
            assert!(!local_addr().is_empty());
            return;
        }
        assert_eq!(local_addr(), "127.0.0.1");
    }
}
