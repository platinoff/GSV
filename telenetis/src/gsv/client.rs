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

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    async fn get_json(&self, path: &str) -> Result<serde_json::Value, TelenetisError> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self.http.get(&url).send().await?;
        let status = resp.status();
        if !status.is_success() {
            return Err(TelenetisError::Gsv(format!("HTTP {status} from {url}")));
        }
        Ok(resp.json().await?)
    }

    async fn post_json(
        &self,
        path: &str,
        body: &serde_json::Value,
    ) -> Result<serde_json::Value, TelenetisError> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self.http.post(&url).json(body).send().await?;
        let status = resp.status();
        if !status.is_success() {
            return Err(TelenetisError::Gsv(format!("HTTP {status} from {url}")));
        }
        Ok(resp.json().await?)
    }

    pub async fn health(&self) -> Result<serde_json::Value, TelenetisError> {
        self.get_json("/api/health").await
    }

    pub async fn tickets(&self) -> Result<serde_json::Value, TelenetisError> {
        self.get_json("/api/tickets/list").await
    }

    pub async fn presence(&self) -> Result<serde_json::Value, TelenetisError> {
        self.get_json("/api/tickets/presence").await
    }

    pub async fn telegram_status(&self) -> Result<serde_json::Value, TelenetisError> {
        self.get_json("/api/telegram/status").await
    }

    pub async fn bus_poll(&self, limit: u8) -> Result<serde_json::Value, TelenetisError> {
        self.get_json(&format!("/api/telegram/bus?limit={limit}"))
            .await
    }

    pub async fn vision_summary(&self) -> Result<serde_json::Value, TelenetisError> {
        self.get_json("/api/vision/summary").await
    }

    /// Forward a board action (claim/done/error) to GSV's ticket wire. The
    /// verb+id mapping plus the JSON payload live in [`crate::actions`].
    pub async fn board_action(
        &self,
        action: crate::actions::BoardAction,
        id: &str,
        note: Option<&str>,
    ) -> Result<serde_json::Value, TelenetisError> {
        let body = serde_json::json!({ "id": id, "note": note });
        self.post_json(action.gsv_path(), &body).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> Config {
        Config {
            bot_token: "test".to_string(),
            gsv_url: "http://127.0.0.1:9999".to_string(),
            port: 9800,
            jail_id: "test-jail".to_string(),
            godfather_channel_id: 0,
            webhook_url: None,
            public_url: None,
            tunnel_enabled: false,
            ngrok_bin: None,
        }
    }

    #[test]
    fn new_strips_trailing_slash() {
        let mut cfg = test_config();
        cfg.gsv_url = "http://127.0.0.1:9999/".to_string();
        let client = GsvClient::new(&cfg);
        assert_eq!(client.base_url(), "http://127.0.0.1:9999");
    }

    #[test]
    fn new_preserves_url_without_slash() {
        let client = GsvClient::new(&test_config());
        assert_eq!(client.base_url(), "http://127.0.0.1:9999");
    }
}
