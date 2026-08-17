//! Local-bind, mutate, and HTTP response guards for `gsv-server`.
//!
//! Default listen is loopback (`127.0.0.1`). Mutating POSTs from a non-local
//! `Origin` or `Sec-Fetch-Site: cross-site` are rejected. Data files are an
//! allowlist of basenames under `data_dir`. Responses carry CSP / nosniff /
//! frame-deny headers; POST bodies are capped at [`MAX_BODY_BYTES`].

/// JSON snapshots served from `GET /data/{file}` (aliases map onto these).
pub const DATA_FILES: &[&str] = &[
    "gsv_tracker.json",
    "gsv_sli.json",
    "gsv_toolchain.json",
    "gsv_manifest.json",
    "gsv_feed.json",
    "gsv_extensions.json",
    "gsv_speed_index.json",
    "gsv_rust_diagnostics.json",
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

/// Gate for POST handlers: missing site/origin (curl, tests) is allowed;
/// browser cross-site or a non-loopback Origin is not.
pub fn gate_post(sec_fetch_site: Option<&str>, origin: Option<&str>) -> Result<(), String> {
    if let Some(site) = sec_fetch_site {
        if site.eq_ignore_ascii_case("cross-site") {
            return Err("cross-site POST rejected".to_string());
        }
    }
    if let Some(origin) = origin {
        if !origin_is_loopback(origin) {
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
    fn content_length_gate_caps_at_max_body_bytes() {
        assert!(gate_content_length(None).is_ok());
        assert!(gate_content_length(Some(0)).is_ok());
        assert!(gate_content_length(Some(MAX_BODY_BYTES as u64)).is_ok());
        assert!(gate_content_length(Some(MAX_BODY_BYTES as u64 + 1)).is_err());
    }
}
