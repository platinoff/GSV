use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub bot_token: String,
    pub gsv_url: String,
    pub port: u16,
    pub jail_id: String,
    pub godfather_channel_id: i64,
    pub webhook_url: Option<String>,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            bot_token: env::var("TELENETIS_BOT_TOKEN").expect("TELENETIS_BOT_TOKEN required"),
            gsv_url: env::var("TELENETIS_GSV_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:9999".to_string()),
            port: env::var("TELENETIS_PORT")
                .unwrap_or_else(|_| "9800".to_string())
                .parse()
                .expect("TELENETIS_PORT must be a number"),
            jail_id: env::var("TELENETIS_JAIL_ID").unwrap_or_else(|_| "telenetis-01".to_string()),
            godfather_channel_id: env::var("TELENETIS_GODFATHER_CHANNEL_ID")
                .unwrap_or_default()
                .parse()
                .unwrap_or(0),
            webhook_url: env::var("TELENETIS_WEBHOOK_URL").ok(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_from_env_reads_vars() {
        std::env::set_var("TELENETIS_BOT_TOKEN", "test_token_123");
        std::env::set_var("TELENETIS_GSV_URL", "http://127.0.0.1:9999");
        std::env::set_var("TELENETIS_PORT", "9800");
        std::env::set_var("TELENETIS_JAIL_ID", "test-jail");
        let cfg = Config::from_env();
        assert_eq!(cfg.bot_token, "test_token_123");
        assert_eq!(cfg.gsv_url, "http://127.0.0.1:9999");
        assert_eq!(cfg.port, 9800);
        assert_eq!(cfg.jail_id, "test-jail");
    }

    #[test]
    fn config_defaults_when_optional_missing() {
        std::env::set_var("TELENETIS_BOT_TOKEN", "tok");
        std::env::remove_var("TELENETIS_GSV_URL");
        std::env::remove_var("TELENETIS_WEBHOOK_URL");
        let cfg = Config::from_env();
        assert_eq!(cfg.gsv_url, "http://127.0.0.1:9999");
        assert!(cfg.webhook_url.is_none());
    }
}
