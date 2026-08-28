use axum::http::header;

pub const MAX_BODY_BYTES: usize = 64 * 1024;

pub fn csrf_check(init_data: &str) -> bool {
    !init_data.is_empty()
}

pub fn security_headers(response: &mut axum::response::Response) {
    let headers = response.headers_mut();
    headers.entry(header::CONTENT_TYPE).or_insert(
        header::HeaderValue::from_static("application/json"),
    );
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
