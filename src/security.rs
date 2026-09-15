//! Local-bind, mutate, and HTTP response guards for `gsv-server`.
//!
//! Default listen is loopback (`127.0.0.1`). Mutating POSTs from a non-local
//! `Origin` or `Sec-Fetch-Site: cross-site` are rejected. With LAN mode on
//! (server bound beyond loopback, band 233) the machine's own local address
//! and private-LAN origins are accepted too. Data files are an allowlist of
//! basenames under `data_dir`. Responses carry CSP / nosniff / frame-deny
//! headers; POST bodies are capped at [`MAX_BODY_BYTES`].

use std::net::IpAddr;
use std::sync::atomic::{AtomicBool, Ordering};

/// JSON snapshots served from `GET /data/{file}` (aliases map onto these).
/// Secrets (`omni.toml`, `gsv_settings.json`) are not on this list.
pub const DATA_FILES: &[&str] = &[
    "gsv_tracker.json",
    "gsv_sli.json",
    "gsv_toolchain.json",
    "gsv_manifest.json",
    "gsv_feed.json",
    "gsv_extensions.json",
    "gsv_speed_index.json",
    "gsv_rust_diagnostics.json",
    "gsv_usage.json",
    "rust_ratio.json",
];

/// Loopback hosts the server binds to by default.
pub fn is_loopback_host(host: &str) -> bool {
    let h = host
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .trim();
    h.eq_ignore_ascii_case("localhost") || h == "127.0.0.1" || h == "::1" || h == "localhost."
}

/// LAN origin mode (band 233): set once by the server when it binds beyond
/// loopback so phone / VM / edge-host UIs can mutate over the local network.
static LAN_MODE: AtomicBool = AtomicBool::new(false);

/// Open (or close) LAN origin acceptance for the POST gate.
pub fn set_lan_mode(on: bool) {
    LAN_MODE.store(on, Ordering::Relaxed);
}

/// Whether the POST gate currently accepts private-LAN origins.
pub fn lan_mode() -> bool {
    LAN_MODE.load(Ordering::Relaxed)
}

/// RFC1919-style private ranges: 10/8, 172.16/12, 192.168/16, link-local
/// 169.254/16, IPv6 ULA fc00::/7 and link-local fe80::/10.
pub fn is_private_lan_host(host: &str) -> bool {
    let h = host
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .trim();
    match h.parse::<IpAddr>() {
        Ok(IpAddr::V4(v4)) => {
            let o = v4.octets();
            o[0] == 10
                || (o[0] == 172 && (16..=31).contains(&o[1]))
                || (o[0] == 192 && o[1] == 168)
                || (o[0] == 169 && o[1] == 254)
        }
        Ok(IpAddr::V6(v6)) => {
            let s = v6.segments()[0];
            s & 0xfe00 == 0xfc00 || s & 0xffc0 == 0xfe80
        }
        Err(_) => false,
    }
}

/// Host is this machine over LAN mode: its resolved local address or a
/// private-LAN range (band 233 — local address, not `127.0.0.1`, for all
/// services so VM / phone / edge peers can reach it).
pub fn is_local_origin_host(host: &str) -> bool {
    if is_loopback_host(host) {
        return true;
    }
    if !lan_mode() {
        return false;
    }
    let h = host
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .trim();
    h == crate::net::local_addr() || is_private_lan_host(h)
}

/// Refuse non-loopback `--host` unless `--allow-lan` was passed.
pub fn ensure_bind_host(host: &str, allow_lan: bool) -> Result<(), String> {
    if allow_lan || is_loopback_host(host) {
        Ok(())
    } else {
        Err(format!(
            "refusing to bind {host}: pass --allow-lan to listen beyond loopback"
        ))
    }
}

/// Host of an `Origin` header (`http://127.0.0.1:9999` → `127.0.0.1`).
pub fn host_from_origin(origin: &str) -> Option<&str> {
    let rest = origin
        .strip_prefix("http://")
        .or_else(|| origin.strip_prefix("https://"))?;
    if let Some(rest) = rest.strip_prefix('[') {
        return rest.split(']').next();
    }
    rest.split([':', '/']).next()
}

/// True when `Origin` points at loopback (same-machine UI).
pub fn origin_is_loopback(origin: &str) -> bool {
    host_from_origin(origin)
        .map(is_loopback_host)
        .unwrap_or(false)
}

/// True when `Origin` points at this machine: loopback always, plus (LAN
/// mode) the resolved local address or any private-LAN host (band 233).
pub fn origin_is_local(origin: &str) -> bool {
    host_from_origin(origin)
        .map(is_local_origin_host)
        .unwrap_or(false)
}

/// Access transport class of a request `Host` value (band 235 PH-S2990):
/// `"loopback"` (on-box), `"lan"` (own address / private range), or
/// `"tunnel"` (everything else — ngrok domains, unknown hosts; fail-closed
/// to view-only semantics). Server mutations are still gated by
/// [`gate_post`]; this classification only informs the UI and poll economy.
pub fn transport_mode(host: Option<&str>) -> &'static str {
    let Some(h) = host.map(|h| h.trim()).filter(|h| !h.is_empty()) else {
        return "tunnel";
    };
    let without_port = match h.strip_prefix('[') {
        // "[::1]:9999" — bracketed IPv6 authority.
        Some(rest) => rest.split(']').next().unwrap_or(rest),
        // "host:8091" has exactly one colon; "::1" (bare IPv6) has more.
        None if h.matches(':').count() == 1 => h.split(':').next().unwrap_or(h),
        None => h,
    };
    let bare = without_port.trim_end_matches('.');
    if is_loopback_host(bare) {
        "loopback"
    } else if bare == crate::net::local_addr() || is_private_lan_host(bare) {
        "lan"
    } else {
        "tunnel"
    }
}

/// Gate for POST handlers: missing site/origin (curl, tests) is allowed;
/// browser cross-site or a foreign Origin is not. In LAN mode the machine's
/// own local address and private-LAN origins pass the gate.
pub fn gate_post(sec_fetch_site: Option<&str>, origin: Option<&str>) -> Result<(), String> {
    if let Some(site) = sec_fetch_site {
        if site.eq_ignore_ascii_case("cross-site") {
            return Err("cross-site POST rejected".to_string());
        }
    }
    if let Some(origin) = origin {
        if !origin_is_local(origin) {
            return Err("non-local origin rejected".to_string());
        }
    }
    Ok(())
}

/// Map a `/data/{file}` segment onto an allowlisted basename under `data_dir`.
pub fn data_file_name(file: &str) -> Result<String, String> {
    if file.is_empty()
        || file.contains("..")
        || file.contains('/')
        || file.contains('\\')
        || file.contains(':')
        || file.contains('\0')
    {
        return Err("illegal data file name".to_string());
    }
    let mapped = match file {
        "sprints.json" | "gsv_history.json" => "gsv_tracker.json",
        other => other,
    };
    if !DATA_FILES.contains(&mapped) {
        return Err(format!("unknown data file: {mapped}"));
    }
    Ok(mapped.to_string())
}

/// Hard cap for POST bodies (terminal / omni / toolchain). Missing
/// `Content-Length` is allowed; the axum `DefaultBodyLimit` layer still
/// enforces this while the body is read.
pub const MAX_BODY_BYTES: usize = 256 * 1024;

/// Content-Security-Policy for the embedded UI (inline script/style stay
/// `'unsafe-inline'` until the glue is external files).
pub const CSP: &str = concat!(
    "default-src 'self'; ",
    "script-src 'self' 'unsafe-inline'; ",
    "worker-src 'self'; ",
    "style-src 'self' 'unsafe-inline'; ",
    "img-src 'self' data:; ",
    "connect-src 'self'; ",
    "font-src 'self'; ",
    "object-src 'none'; ",
    "base-uri 'self'; ",
    "form-action 'self'; ",
    "frame-ancestors 'none'"
);

/// Response header pairs (lowercase names for HTTP/2 `HeaderName::from_static`).
pub const SECURITY_HEADERS: &[(&str, &str)] = &[
    ("content-security-policy", CSP),
    ("x-content-type-options", "nosniff"),
    ("x-frame-options", "DENY"),
    ("referrer-policy", "no-referrer"),
    (
        "permissions-policy",
        "camera=(), microphone=(), geolocation=()",
    ),
    ("cross-origin-opener-policy", "same-origin"),
    ("cross-origin-resource-policy", "same-origin"),
    ("cache-control", "no-store"),
];

/// Reject POST bodies that advertise a `Content-Length` over [`MAX_BODY_BYTES`].
pub fn gate_content_length(content_length: Option<u64>) -> Result<(), String> {
    match content_length {
        Some(n) if n > MAX_BODY_BYTES as u64 => Err("request body too large".to_string()),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loopback_hosts_are_recognized() {
        assert!(is_loopback_host("127.0.0.1"));
        assert!(is_loopback_host("localhost"));
        assert!(is_loopback_host("LOCALHOST"));
        assert!(is_loopback_host("::1"));
        assert!(is_loopback_host("[::1]"));
        assert!(!is_loopback_host("0.0.0.0"));
        assert!(!is_loopback_host("192.168.1.10"));
    }

    #[test]
    fn bind_requires_allow_lan_off_loopback() {
        assert!(ensure_bind_host("127.0.0.1", false).is_ok());
        assert!(ensure_bind_host("0.0.0.0", false).is_err());
        assert!(ensure_bind_host("0.0.0.0", true).is_ok());
    }

    #[test]
    fn origin_loopback_from_live_ui() {
        assert!(origin_is_loopback("http://127.0.0.1:9999"));
        assert!(origin_is_loopback("http://localhost:9999/"));
        assert!(origin_is_loopback("http://[::1]:9999"));
        assert!(!origin_is_loopback("https://example.com"));
        assert!(!origin_is_loopback("http://192.168.0.2:9999"));
    }

    #[test]
    fn gate_post_allows_local_and_missing() {
        assert!(gate_post(None, None).is_ok());
        assert!(gate_post(Some("same-origin"), Some("http://127.0.0.1:9999")).is_ok());
        assert!(gate_post(Some("cross-site"), None).is_err());
        assert!(gate_post(None, Some("https://example.com")).is_err());
    }

    #[test]
    fn lan_mode_opens_own_private_origins_and_closes_cleanly() {
        assert!(gate_post(Some("same-origin"), Some("http://192.168.2.238:9999")).is_err());
        set_lan_mode(true);
        assert!(lan_mode());
        assert!(is_private_lan_host("10.1.2.3"));
        assert!(is_private_lan_host("172.16.0.1"));
        assert!(is_private_lan_host("172.31.255.255"));
        assert!(!is_private_lan_host("172.32.0.1"));
        assert!(is_private_lan_host("192.168.56.1"));
        assert!(is_private_lan_host("169.254.1.1"));
        assert!(is_private_lan_host("fd12:3456::78"));
        assert!(is_private_lan_host("fe80::1"));
        assert!(!is_private_lan_host("8.8.8.8"));
        assert!(!is_private_lan_host("example.com"));
        assert!(origin_is_local("http://192.168.2.238:9999"));
        assert!(gate_post(Some("same-origin"), Some("http://192.168.2.238:9999")).is_ok());
        assert!(gate_post(Some("same-origin"), Some("http://[fd12:3456::78]:9999")).is_ok());
        // Internet origins stay rejected even in LAN mode; loopback always ok.
        assert!(gate_post(Some("same-origin"), Some("https://example.com")).is_err());
        assert!(gate_post(Some("same-origin"), Some("http://127.0.0.1:9999")).is_ok());
        set_lan_mode(false);
        assert!(gate_post(Some("same-origin"), Some("http://192.168.2.238:9999")).is_err());
    }

    #[test]
    fn data_file_allowlist_and_aliases() {
        assert_eq!(
            data_file_name("gsv_tracker.json").unwrap(),
            "gsv_tracker.json"
        );
        assert_eq!(data_file_name("sprints.json").unwrap(), "gsv_tracker.json");
        assert!(data_file_name("..").is_err());
        assert!(data_file_name("foo.json").is_err());
        assert!(data_file_name("gsv_tracker.json/../x").is_err());
        assert!(data_file_name("omni.toml").is_err());
    }

    #[test]
    fn security_headers_cover_csp_nosniff_and_no_store() {
        let names: Vec<&str> = SECURITY_HEADERS.iter().map(|(n, _)| *n).collect();
        assert!(names.contains(&"content-security-policy"));
        assert!(names.contains(&"x-content-type-options"));
        assert!(names.contains(&"x-frame-options"));
        assert!(names.contains(&"cache-control"));
        assert!(CSP.contains("frame-ancestors 'none'"));
        assert!(CSP.contains("default-src 'self'"));
        let nosniff = SECURITY_HEADERS
            .iter()
            .find(|(n, _)| *n == "x-content-type-options")
            .map(|(_, v)| *v);
        assert_eq!(nosniff, Some("nosniff"));
        let cache = SECURITY_HEADERS
            .iter()
            .find(|(n, _)| *n == "cache-control")
            .map(|(_, v)| *v);
        assert_eq!(cache, Some("no-store"));
    }

    #[test]
    fn transport_mode_classes_loopback_lan_and_tunnel() {
        assert_eq!(transport_mode(Some("127.0.0.1:9999")), "loopback");
        assert_eq!(transport_mode(Some("localhost")), "loopback");
        assert_eq!(transport_mode(Some("[::1]:9999")), "loopback");
        assert_eq!(transport_mode(Some("::1")), "loopback");
        assert_eq!(transport_mode(Some("192.168.2.238:9999")), "lan");
        assert_eq!(transport_mode(Some("10.1.2.3")), "lan");
        assert_eq!(transport_mode(Some("[fd12::3456]")), "lan");
        assert_eq!(
            transport_mode(Some("atonable-alibi-unwilling.ngrok-free.app")),
            "tunnel"
        );
        assert_eq!(transport_mode(Some("example.com:443")), "tunnel");
        assert_eq!(transport_mode(None), "tunnel", "missing host fails closed");
        assert_eq!(transport_mode(Some("  ")), "tunnel");
        // The resolved local address is lan-class whenever it is not loopback
        // (loopback wins by the explicit rule above; under the test harness
        // local_addr() is 127.0.0.1, so assert the non-loopback form).
        let local = crate::net::local_addr();
        let want = if is_loopback_host(&local) {
            "loopback"
        } else {
            "lan"
        };
        assert_eq!(transport_mode(Some(&local)), want);
    }

    #[test]
    fn content_length_gate_caps_at_max_body_bytes() {
        assert!(gate_content_length(None).is_ok());
        assert!(gate_content_length(Some(0)).is_ok());
        assert!(gate_content_length(Some(MAX_BODY_BYTES as u64)).is_ok());
        assert!(gate_content_length(Some(MAX_BODY_BYTES as u64 + 1)).is_err());
    }
}
