use axum::http::header;

pub const MAX_BODY_BYTES: usize = 64 * 1024;

pub fn csrf_check(init_data: &str) -> bool {
    !init_data.is_empty()
}

pub fn security_headers(response: &mut axum::response::Response) {
    let headers = response.headers_mut();
    headers.insert(header::CONTENT_TYPE, "application/json".parse().unwrap());
    headers.insert("X-Content-Type-Options", "nosniff".parse().unwrap());
    headers.insert("Cache-Control", "no-store".parse().unwrap());
    headers.insert(
        "Content-Security-Policy",
        "default-src 'self'".parse().unwrap(),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csrf_check_empty_fails() {
        assert!(!csrf_check(""));
    }

    #[test]
    fn csrf_check_non_empty_passes() {
        assert!(csrf_check("some_init_data"));
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
}
