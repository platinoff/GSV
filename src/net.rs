//! Local-address resolution for LAN-first service flows (band 233).
//!
//! Inter-host flows (llama-rs edge/VM/phone sessions) cannot see this box's
//! loopback, so advertised service URLs use the machine's own LAN address.
//! Resolution order: `GSV_LOCAL_ADDR` env → `127.0.0.1` under the cargo-test
//! harness (deterministic) → default-route source IP (UDP connect trick —
//! selects the egress interface, sends no packets) → loopback fallback.

use std::net::{SocketAddr, UdpSocket};

/// Env form of the override: a trimmed non-empty value wins; blank is
/// treated as unset. Split out from [`local_addr`] so tests stay pure — no
/// `set_var` racing against other tests that resolve defaults in-process.
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
    if let Some(v) = from_env_value(std::env::var("GSV_LOCAL_ADDR").ok()) {
        return v;
    }
    if crate::boxes::update::is_cargo_test_harness() {
        return "127.0.0.1".to_string();
    }
    default_route_addr().unwrap_or_else(|| "127.0.0.1".to_string())
}

fn default_route_addr() -> Option<String> {
    let sock = UdpSocket::bind("0.0.0.0:0").ok()?;
    // `connect` on a UDP socket only pins the default-route source address.
    sock.connect("8.8.8.8:53").ok()?;
    match sock.local_addr().ok()? {
        SocketAddr::V4(v4) if v4.ip().is_loopback() => None,
        SocketAddr::V4(v4) => Some(v4.ip().to_string()),
        SocketAddr::V6(v6) if v6.ip().is_loopback() => None,
        SocketAddr::V6(v6) => Some(v6.ip().to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_form_trims_and_rejects_blank() {
        assert_eq!(from_env_value(None), None);
        assert_eq!(from_env_value(Some("  ".into())), None);
        assert_eq!(
            from_env_value(Some(" 10.20.30.40 ".into())),
            Some("10.20.30.40".into())
        );
    }

    #[test]
    fn harness_resolution_is_loopback_and_never_empty() {
        // No set_var here: other in-process tests resolve default URLs.
        if from_env_value(std::env::var("GSV_LOCAL_ADDR").ok()).is_some() {
            // Owner set the override for live runs — local_addr must honour it.
            assert!(!local_addr().is_empty());
            return;
        }
        // Under the cargo-test harness `local_addr()` is pinned to loopback.
        assert_eq!(local_addr(), "127.0.0.1");
    }
}
