use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use std::fmt;

#[derive(Debug)]
pub enum TelenetisError {
    Telegram(String),
    Gsv(String),
    Config(String),
    Serialization(String),
    Tunnel(String),
    Io(std::io::Error),
    Reqwest(reqwest::Error),
}

impl fmt::Display for TelenetisError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Telegram(msg) => write!(f, "Telegram: {msg}"),
            Self::Gsv(msg) => write!(f, "GSV: {msg}"),
            Self::Config(msg) => write!(f, "Config: {msg}"),
            Self::Serialization(msg) => write!(f, "Serialization: {msg}"),
            Self::Tunnel(msg) => write!(f, "Tunnel: {msg}"),
            Self::Io(err) => write!(f, "IO: {err}"),
            Self::Reqwest(err) => write!(f, "HTTP: {err}"),
        }
    }
}

impl IntoResponse for TelenetisError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            Self::Telegram(msg) => (StatusCode::BAD_GATEWAY, msg.clone()),
            Self::Gsv(msg) => (StatusCode::BAD_GATEWAY, msg.clone()),
            Self::Config(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
            Self::Serialization(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
            Self::Tunnel(msg) => (StatusCode::BAD_GATEWAY, msg.clone()),
            Self::Io(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
            Self::Reqwest(err) => (StatusCode::BAD_GATEWAY, err.to_string()),
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

impl From<reqwest::Error> for TelenetisError {
    fn from(e: reqwest::Error) -> Self {
        Self::Reqwest(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;

    #[test]
    fn telegram_maps_to_bad_gateway() {
        let resp = TelenetisError::Telegram("tg err".to_string()).into_response();
        assert_eq!(resp.status(), StatusCode::BAD_GATEWAY);
    }

    #[test]
    fn config_maps_to_internal_error() {
        let resp = TelenetisError::Config("cfg err".to_string()).into_response();
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn io_from_std_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "not found");
        let te: TelenetisError = io_err.into();
        assert!(matches!(te, TelenetisError::Io(_)));
    }

    #[tokio::test]
    async fn reqwest_maps_to_bad_gateway() {
        let client = reqwest::Client::new();
        let err = client
            .get("http://invalid.example.test")
            .send()
            .await
            .unwrap_err();
        let resp = TelenetisError::Reqwest(err).into_response();
        assert_eq!(resp.status(), StatusCode::BAD_GATEWAY);
    }
}
