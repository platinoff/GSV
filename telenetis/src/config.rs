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
