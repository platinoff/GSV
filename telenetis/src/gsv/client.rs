use crate::config::Config;
use crate::error::TelenetisError;
use reqwest::Client;

#[derive(Clone)]
pub struct GsvClient {
    http: Client,
    base_url: String,
}

impl GsvClient {
    pub fn new(config: &Config) -> Self {
        Self {
            http: Client::new(),
            base_url: config.gsv_url.trim_end_matches('/').to_string(),
        }
    }

    pub async fn health(&self) -> Result<serde_json::Value, TelenetisError> {
        let resp = self
            .http
            .get(format!("{}/api/health", self.base_url))
            .send()
            .await
            .map_err(|e| TelenetisError::Gsv(e.to_string()))?;
        let body = resp
            .json()
            .await
            .map_err(|e| TelenetisError::Gsv(e.to_string()))?;
        Ok(body)
    }

    pub async fn tickets(&self) -> Result<serde_json::Value, TelenetisError> {
        let resp = self
            .http
            .get(format!("{}/api/tickets/list", self.base_url))
            .send()
            .await
            .map_err(|e| TelenetisError::Gsv(e.to_string()))?;
        let body = resp
            .json()
            .await
            .map_err(|e| TelenetisError::Gsv(e.to_string()))?;
        Ok(body)
    }

    pub async fn presence(&self) -> Result<serde_json::Value, TelenetisError> {
        let resp = self
            .http
            .get(format!("{}/api/tickets/presence", self.base_url))
            .send()
            .await
            .map_err(|e| TelenetisError::Gsv(e.to_string()))?;
        let body = resp
            .json()
            .await
            .map_err(|e| TelenetisError::Gsv(e.to_string()))?;
        Ok(body)
    }

    pub async fn telegram_status(&self) -> Result<serde_json::Value, TelenetisError> {
        let resp = self
            .http
            .get(format!("{}/api/telegram/status", self.base_url))
            .send()
            .await
            .map_err(|e| TelenetisError::Gsv(e.to_string()))?;
        let body = resp
            .json()
            .await
            .map_err(|e| TelenetisError::Gsv(e.to_string()))?;
        Ok(body)
    }
}
