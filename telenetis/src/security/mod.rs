pub mod auth;

use tower_http::limit::RequestBodyLimitLayer;

/// Security stack layer: body cap 64 KiB.
/// Use `limit_layer()` in router composition if needed.
pub fn limit_layer() -> RequestBodyLimitLayer {
    RequestBodyLimitLayer::new(auth::MAX_BODY_BYTES)
}
