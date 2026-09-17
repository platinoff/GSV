use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub bot_token: String,
    pub gsv_url: String,
    /// poolAI coordinator base URL (edge workers register here; default
    /// :8091 because :8080 is llama_serve on this box).
    pub poolai_url: String,
    pub port: u16,
    pub jail_id: String,
    pub godfather_channel_id: i64,
    pub webhook_url: Option<String>,
    /// Optional `secret_token` sent to Telegram with `setWebhook` and
    /// required (in the `X-Telegram-Bot-Api-Secret-Token` header) on every
    /// inbound `/webhook` POST. When set, forged updates are rejected. Mutually
    /// tied to `webhook_url` — polling (`getUpdates`) has no secret.
    pub webhook_secret: Option<String>,
    /// Public HTTPS base for the Telegram WebApp button. Without an external
    /// host the Mini App's `web_app_url` (e.g. `http://127.0.0.1:9800`) will
    /// only open on the same machine's Telegram client — Telegram WebApp
    /// requires a reachable HTTPS URL to work from phones / remote clients.
    /// When empty, the tunnel manager auto-derives it from ngrok.
    pub public_url: Option<String>,
    /// Auto-start an ngrok tunnel when a public URL is needed (webhook /
    /// Mini App). Set `false` to disable.
    pub tunnel_enabled: bool,
    /// Optional explicit path to the ngrok binary. If empty, ngrok is looked
    /// up on PATH and a few well-known locations.
    pub ngrok_bin: Option<String>,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            bot_token: env::var("TELENETIS_BOT_TOKEN").unwrap_or_default(),
            // Band 233: defaults point at the machine's local (LAN) address,
            // not `127.0.0.1` — same for GSV :9999 and poolAI :8091 peers;
            // loopback under the test harness keeps asserts deterministic.
            gsv_url: env::var("TELENETIS_GSV_URL")
                .unwrap_or_else(|_| format!("http://{}:9999", crate::net::local_addr())),
            poolai_url: env::var("TELENETIS_POOLAI_URL")
                .unwrap_or_else(|_| format!("http://{}:8091", crate::net::local_addr())),
            port: env::var("TELENETIS_PORT")
                .unwrap_or_else(|_| "9800".to_string())
                .parse()
                .unwrap_or_else(|_| {
                    // A typo'd port must degrade to the default, not panic
                    // the always-on service at boot (found by bug-hunt audit).
                    eprintln!("TELENETIS_PORT is not a number — falling back to 9800");
                    9800
                }),
            jail_id: env::var("TELENETIS_JAIL_ID").unwrap_or_else(|_| "telenetis-01".to_string()),
            godfather_channel_id: env::var("TELENETIS_GODFATHER_CHANNEL_ID")
                .unwrap_or_default()
                .parse()
                .unwrap_or(0),
            webhook_url: env::var("TELENETIS_WEBHOOK_URL")
                .ok()
                .filter(|s| !s.is_empty()),
            webhook_secret: env::var("TELENETIS_WEBHOOK_SECRET")
                .ok()
                .filter(|s| !s.is_empty()),
            public_url: env::var("TELENETIS_PUBLIC_URL")
                .ok()
                .filter(|s| !s.is_empty()),
            tunnel_enabled: env::var("TELENETIS_TUNNEL_ENABLED")
                .map(|v| v != "0" && v.to_lowercase() != "false")
                .unwrap_or(true),
            ngrok_bin: env::var("TELENETIS_NGROK_BIN")
                .ok()
                .filter(|s| !s.is_empty()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Process env is shared across test threads: the tests below mutate it,
    /// so they serialize on this lock (otherwise parallel runs flake).
    static ENV_GUARD: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn config_from_env_reads_vars() {
        let _guard = ENV_GUARD.lock().unwrap();
        std::env::set_var("TELENETIS_BOT_TOKEN", "test_token_123");
        std::env::set_var("TELENETIS_GSV_URL", "http://127.0.0.1:9999");
        std::env::set_var("TELENETIS_POOLAI_URL", "http://127.0.0.1:8091");
        std::env::set_var("TELENETIS_PORT", "9800");
        std::env::set_var("TELENETIS_JAIL_ID", "test-jail");
        let cfg = Config::from_env();
        assert_eq!(cfg.bot_token, "test_token_123");
        assert_eq!(cfg.gsv_url, "http://127.0.0.1:9999");
        assert_eq!(cfg.poolai_url, "http://127.0.0.1:8091");
        assert_eq!(cfg.port, 9800);
        assert_eq!(cfg.jail_id, "test-jail");
    }

    #[test]
    fn config_bad_port_falls_back_instead_of_panicking() {
        // Bug-hunt: a garbage TELENETIS_PORT must not kill the service.
        let _guard = ENV_GUARD.lock().unwrap();
        std::env::set_var("TELENETIS_PORT", "not-a-port");
        let cfg = Config::from_env();
        assert_eq!(cfg.port, 9800);
        std::env::set_var("TELENETIS_PORT", "9800");
    }

    #[test]
    fn config_defaults_when_optional_missing() {
        let _guard = ENV_GUARD.lock().unwrap();
        std::env::set_var("TELENETIS_BOT_TOKEN", "tok");
        std::env::remove_var("TELENETIS_GSV_URL");
        std::env::remove_var("TELENETIS_POOLAI_URL");
        std::env::remove_var("TELENETIS_WEBHOOK_URL");
        let cfg = Config::from_env();
        assert_eq!(cfg.gsv_url, "http://127.0.0.1:9999");
        assert_eq!(cfg.poolai_url, "http://127.0.0.1:8091");
        assert!(cfg.webhook_url.is_none());
    }
}
