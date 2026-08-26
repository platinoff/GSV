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
