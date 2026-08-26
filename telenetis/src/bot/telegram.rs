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

    pub async fn send_message(&self, chat_id: i64, text: &str) -> Result<Value, TelenetisError> {
        let resp = self
            .http
            .post(format!("{}/sendMessage", self.api_base))
            .json(&json!({
                "chat_id": chat_id,
                "text": text,
                "parse_mode": "Markdown",
            }))
            .send()
            .await
            .map_err(|e| TelenetisError::Telegram(e.to_string()))?;
        let body = resp
            .json()
            .await
            .map_err(|e| TelenetisError::Telegram(e.to_string()))?;
        Ok(body)
    }

    pub async fn send_mini_app(
        &self,
        chat_id: i64,
        text: &str,
        web_app_url: &str,
    ) -> Result<Value, TelenetisError> {
        let resp = self
            .http
            .post(format!("{}/sendMessage", self.api_base))
            .json(&json!({
                "chat_id": chat_id,
                "text": text,
                "reply_markup": {
                    "inline_keyboard": [[{
                        "text": "Open Telenetis",
                        "web_app": { "url": web_app_url }
                    }]]
                }
            }))
            .send()
            .await
            .map_err(|e| TelenetisError::Telegram(e.to_string()))?;
        let body = resp
            .json()
            .await
            .map_err(|e| TelenetisError::Telegram(e.to_string()))?;
        Ok(body)
    }

    pub async fn answer_callback(
        &self,
        callback_query_id: &str,
        text: &str,
    ) -> Result<Value, TelenetisError> {
        let resp = self
            .http
            .post(format!("{}/answerCallbackQuery", self.api_base))
            .json(&json!({
                "callback_query_id": callback_query_id,
                "text": text,
            }))
            .send()
            .await
            .map_err(|e| TelenetisError::Telegram(e.to_string()))?;
        let body = resp
            .json()
            .await
            .map_err(|e| TelenetisError::Telegram(e.to_string()))?;
        Ok(body)
    }

    pub async fn set_webhook(&self, url: &str) -> Result<Value, TelenetisError> {
        let resp = self
            .http
            .post(format!("{}/setWebhook", self.api_base))
            .json(&json!({ "url": url }))
            .send()
            .await
            .map_err(|e| TelenetisError::Telegram(e.to_string()))?;
        let body = resp
            .json()
            .await
            .map_err(|e| TelenetisError::Telegram(e.to_string()))?;
        Ok(body)
    }
}
