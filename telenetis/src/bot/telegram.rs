use crate::config::Config;
use crate::error::TelenetisError;
use reqwest::Client;
use serde_json::{json, Value};

#[derive(Clone)]
pub struct TelegramBot {
    http: Client,
    api_base: String,
}

impl TelegramBot {
    pub fn new(config: &Config) -> Self {
        Self {
            http: Client::new(),
            api_base: format!("https://api.telegram.org/bot{}", config.bot_token),
        }
    }

    pub fn api_base(&self) -> &str {
        &self.api_base
    }

    async fn post(&self, method: &str, body: Value) -> Result<Value, TelenetisError> {
        let url = format!("{}/{}", self.api_base, method);
        let resp = self.http.post(&url).json(&body).send().await?;
        let status = resp.status();
        if !status.is_success() {
            return Err(TelenetisError::Telegram(format!(
                "HTTP {status} from Telegram {method}"
            )));
        }
        let body: Value = resp.json().await?;
        if body.get("ok").and_then(|v| v.as_bool()) != Some(true) {
            let desc = body
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            return Err(TelenetisError::Telegram(format!(
                "Telegram {method}: {desc}"
            )));
        }
        Ok(body)
    }

    pub async fn send_message(&self, chat_id: i64, text: &str) -> Result<Value, TelenetisError> {
        match self
            .post(
                "sendMessage",
                json!({
                    "chat_id": chat_id,
                    "text": text,
                    "parse_mode": "Markdown",
                }),
            )
            .await
        {
            Ok(v) => Ok(v),
            Err(_) => {
                self.post("sendMessage", json!({ "chat_id": chat_id, "text": text }))
                    .await
            }
        }
    }

    pub async fn send_mini_app(
        &self,
        chat_id: i64,
        text: &str,
        web_app_url: &str,
    ) -> Result<Value, TelenetisError> {
        self.post(
            "sendMessage",
            json!({
                "chat_id": chat_id,
                "text": text,
                "reply_markup": {
                    "inline_keyboard": [[{
                        "text": "Open Telenetis",
                        "web_app": { "url": web_app_url }
                    }]]
                }
            }),
        )
        .await
    }

    pub async fn answer_callback(
        &self,
        callback_query_id: &str,
        text: &str,
    ) -> Result<Value, TelenetisError> {
        self.post(
            "answerCallbackQuery",
            json!({
                "callback_query_id": callback_query_id,
                "text": text,
            }),
        )
        .await
    }

    pub async fn set_webhook(&self, url: &str) -> Result<Value, TelenetisError> {
        self.post("setWebhook", json!({ "url": url })).await
    }

    /// Disable any active webhook so `getUpdates` polling can be used from
    /// behind NAT without a public tunnel.
    pub async fn delete_webhook(&self) -> Result<Value, TelenetisError> {
        self.post("deleteWebhook", json!({})).await
    }

    /// Long-poll Telegram for updates (alternative to webhooks). Returns the
    /// `result` array of update objects.
    pub async fn get_updates(
        &self,
        offset: i64,
        timeout_secs: u64,
    ) -> Result<Value, TelenetisError> {
        self.post(
            "getUpdates",
            json!({
                "offset": offset,
                "timeout": timeout_secs,
                "allowed_updates": ["message", "callback_query", "my_chat_member"],
            }),
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> Config {
        Config {
            bot_token: "test_token_123:ABC".to_string(),
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
    fn new_builds_api_base() {
        let bot = TelegramBot::new(&test_config());
        assert_eq!(
            bot.api_base(),
            "https://api.telegram.org/bottest_token_123:ABC"
        );
    }

    #[test]
    fn api_base_is_url() {
        let bot = TelegramBot::new(&test_config());
        assert!(bot.api_base().starts_with("https://api.telegram.org/bot"));
    }
}
