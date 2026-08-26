use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

#[derive(Debug)]
pub enum TelenetisError {
    Telegram(String),
    Gsv(String),
    Config(String),
    Serialization(String),
    Io(std::io::Error),
}

impl IntoResponse for TelenetisError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            Self::Telegram(msg) => (StatusCode::BAD_GATEWAY, msg.clone()),
            Self::Gsv(msg) => (StatusCode::BAD_GATEWAY, msg.clone()),
            Self::Config(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
            Self::Serialization(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
            Self::Io(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
        };
        (status, message).into_response()
    }
}

impl From<std::io::Error> for TelenetisError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<serde_json::Error> for TelenetisError {
    fn from(e: serde_json::Error) -> Self {
        Self::Serialization(e.to_string())
    }
}
