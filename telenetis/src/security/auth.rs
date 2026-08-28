use axum::http::header;

use super::initdata::{self, verify_init_data};

pub const MAX_BODY_BYTES: usize = 64 * 1024;

/// Convenience boolean wrapper over [`verify_init_data`]: true only when the
/// `initData` handshake carries a valid HMAC-SHA256 signature for `bot_token`
/// and its `auth_date` is within the default freshness window of `now_unix`.
pub fn csrf_check(init_data: &str, bot_token: &str, now_unix: i64) -> bool {
    verify_init_data(
        init_data,
        bot_token,
        now_unix,
        initdata::DEFAULT_MAX_AGE_SECS,
    )
    .is_ok()
}

pub fn security_headers(response: &mut axum::response::Response) {
    let headers = response.headers_mut();
    headers
        .entry(header::CONTENT_TYPE)
        .or_insert(header::HeaderValue::from_static("application/json"));
    headers.insert("X-Content-Type-Options", "nosniff".parse().unwrap());
    headers.insert("Cache-Control", "no-store".parse().unwrap());
    headers.insert(
        "Content-Security-Policy",
        "default-src 'self'; script-src 'self' https://telegram.org; style-src 'self' 'unsafe-inline'; connect-src 'self' ws: wss:; img-src 'self' data:"
            .parse()
            .unwrap(),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csrf_check_empty_fails() {
        assert!(!csrf_check("", "token", 1_750_000_000));
    }

    #[test]
    fn csrf_check_tampered_fails() {
        // A non-empty but unsigned initData must NOT pass CSRF now that the
        // check is a real HMAC verification (not a length test).
        assert!(!csrf_check(
            "auth_date=1&user=%7B%7D&hash=ffff",
            "token",
            1_750_000_000
        ));
    }

    #[test]
    fn max_body_is_64k() {
        assert_eq!(MAX_BODY_BYTES, 64 * 1024);
    }

    #[test]
    fn security_headers_sets_nosniff() {
        let mut resp = axum::response::Response::new(axum::body::Body::empty());
        security_headers(&mut resp);
        assert_eq!(
            resp.headers().get("X-Content-Type-Options").unwrap(),
            "nosniff"
        );
        assert_eq!(resp.headers().get("Cache-Control").unwrap(), "no-store");
    }

    #[test]
    fn security_headers_preserves_existing_content_type() {
        let mut resp = axum::response::Response::new(axum::body::Body::empty());
        resp.headers_mut()
            .insert(header::CONTENT_TYPE, "text/html".parse().unwrap());
        security_headers(&mut resp);
        assert_eq!(
            resp.headers().get(header::CONTENT_TYPE).unwrap(),
            "text/html"
        );
    }

    #[test]
    fn security_headers_sets_json_when_missing() {
        let mut resp = axum::response::Response::new(axum::body::Body::empty());
        security_headers(&mut resp);
        assert_eq!(
            resp.headers().get(header::CONTENT_TYPE).unwrap(),
            "application/json"
        );
    }
}
