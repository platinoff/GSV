//! Godfather Telegram channel bind (band 167).
//!
//! On-demand `getMe` + `getChat`. Tests and `X-Telegram-Dry-Run: 1` use an
//! in-process stub (no sockets). Live Bot API is enabled only from `gsv-server`
//! / `gsv-mcp` via [`enable_live_api`]. Poller default off — no boot probe.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use axum::http::HeaderMap;
use serde_json::{json, Value};

use super::settings::{self, SettingsFile};

/// Header that forces the dry-run stub even on a live server.
pub const DRY_RUN_HEADER: &str = "x-telegram-dry-run";
/// Process env that forces the stub (`1` / `true`).
pub const DRY_RUN_ENV: &str = "GSV_TELEGRAM_DRY_RUN";

const STUB_BOT: &str = "gsv_godfather_bot";
const STUB_CHAT: &str = "GSV Godfather (dry-run)";
const API_ROOT: &str = "https://api.telegram.org";

static LIVE_API: AtomicBool = AtomicBool::new(false);

/// Allow outbound Bot API (call from `gsv-server` / `gsv-mcp` mains only).
pub fn enable_live_api() {
    LIVE_API.store(true, Ordering::SeqCst);
}

/// True after [`enable_live_api`]. Cargo tests leave this false.
pub fn live_api_enabled() -> bool {
    LIVE_API.load(Ordering::SeqCst)
}

/// Band 167: Galaxy boot must not probe Telegram.
pub fn boot_should_probe() -> bool {
    false
}

/// Opt-in poller: `godfather.poll` or co-workflow `telegram-relay`.
pub fn poller_wanted(file: &SettingsFile) -> bool {
    file.godfather.poll
        || file
            .workflows
            .enabled
            .iter()
            .any(|id| id == "telegram-relay")
}

/// `X-Telegram-Dry-Run: 1`.
pub fn header_dry_run(headers: &HeaderMap) -> bool {
    headers
        .get(DRY_RUN_HEADER)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v == "1")
}

/// `GSV_TELEGRAM_DRY_RUN=1` (or `true`).
pub fn env_dry_run() -> bool {
    std::env::var(DRY_RUN_ENV)
        .ok()
        .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true"))
}

fn resolved_token(file: &SettingsFile) -> Option<String> {
    settings::env_token().or_else(|| {
        let t = file.godfather.bot_token.trim();
        if t.is_empty() {
            None
        } else {
            Some(t.to_string())
        }
    })
}

/// Strip a token substring from an error string.
pub fn redact(msg: &str, token: &str) -> String {
    let t = token.trim();
    if t.is_empty() {
        msg.to_string()
    } else {
        msg.replace(t, "[redacted]")
    }
}

fn fail(
    error: &str,
    token: &str,
    token_set: bool,
    polling: bool,
    dry_run: bool,
    channel_id: &str,
) -> Value {
    json!({
        "ok": false,
        "error": redact(error, token),
        "channel_id": channel_id,
        "token_set": token_set,
        "bot_username": "",
        "chat_title": "",
        "last_probe": "",
        "polling": polling,
        "dry_run": dry_run,
    })
}

/// Map a probe failure to redacted `{ok:false,error}` (never includes `bot_token`).
pub fn map_probe_error(msg: &str, token: &str) -> Value {
    fail(msg, token, !token.trim().is_empty(), false, false, "")
}

fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn stub_ok(channel_id: &str, polling: bool) -> Value {
    json!({
        "ok": true,
        "dry_run": true,
        "channel_id": channel_id,
        "token_set": true,
        "bot_username": STUB_BOT,
        "chat_title": STUB_CHAT,
        "last_probe": now_rfc3339(),
        "polling": polling,
    })
}

fn use_stub(explicit_dry_run: bool) -> bool {
    explicit_dry_run || env_dry_run() || !live_api_enabled()
}

/// `GET /api/telegram` / MCP `gsv_telegram`.
pub async fn status(data_dir: &Path, dry_run: bool) -> Value {
    match settings::load_result(data_dir) {
        Ok(file) => status_loaded(&file, dry_run).await,
        Err(e) => fail(&e, "", false, false, use_stub(dry_run), ""),
    }
}

async fn status_loaded(file: &SettingsFile, explicit_dry: bool) -> Value {
    let dry = use_stub(explicit_dry);
    let polling = poller_wanted(file);
    let channel = file.godfather.channel_id.trim();
    let token = resolved_token(file);
    let token_set = token.is_some();
    if channel.is_empty() {
        return fail(
            "godfather channel_id is not set",
            token.as_deref().unwrap_or(""),
            token_set,
            polling,
            dry,
            "",
        );
    }
    let Some(token) = token else {
        return fail(
            "godfather bot token is not set",
            "",
            false,
            polling,
            dry,
            channel,
        );
    };
    if dry {
        return stub_ok(channel, polling);
    }
    probe_live(&token, channel, polling).await
}

async fn probe_live(token: &str, channel: &str, polling: bool) -> Value {
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return fail(
                &format!("telegram client: {e}"),
                token,
                true,
                polling,
                false,
                channel,
            )
        }
    };
    let me_url = format!("{API_ROOT}/bot{token}/getMe");
    let me = match client.get(&me_url).send().await {
        Ok(r) => r,
        Err(e) => return fail(&format!("getMe: {e}"), token, true, polling, false, channel),
    };
    let me_json = match me.json::<Value>().await {
        Ok(v) => v,
        Err(e) => {
            return fail(
                &format!("getMe body: {e}"),
                token,
                true,
                polling,
                false,
                channel,
            )
        }
    };
    if me_json.get("ok").and_then(Value::as_bool) != Some(true) {
        let desc = me_json
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or("getMe failed");
        return fail(desc, token, true, polling, false, channel);
    }
    let bot_username = me_json["result"]["username"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let chat_url = format!("{API_ROOT}/bot{token}/getChat");
    let chat = match client
        .get(&chat_url)
        .query(&[("chat_id", channel)])
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            return fail(
                &format!("getChat: {e}"),
                token,
                true,
                polling,
                false,
                channel,
            )
        }
    };
    let chat_json = match chat.json::<Value>().await {
        Ok(v) => v,
        Err(e) => {
            return fail(
                &format!("getChat body: {e}"),
                token,
                true,
                polling,
                false,
                channel,
            )
        }
    };
    if chat_json.get("ok").and_then(Value::as_bool) != Some(true) {
        let desc = chat_json
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or("getChat failed");
        return fail(desc, token, true, polling, false, channel);
    }
    let title = chat_json["result"]["title"].as_str().unwrap_or("");
    let chat_title = if title.is_empty() {
        chat_json["result"]["username"]
            .as_str()
            .unwrap_or("")
            .to_string()
    } else {
        title.to_string()
    };
    json!({
        "ok": true,
        "dry_run": false,
        "channel_id": channel,
        "token_set": true,
        "bot_username": bot_username,
        "chat_title": chat_title,
        "last_probe": now_rfc3339(),
        "polling": polling,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_api_off_in_lib_tests() {
        assert!(!live_api_enabled());
        assert!(!boot_should_probe());
    }

    #[test]
    fn redact_replaces_token_only() {
        assert_eq!(redact("ok", ""), "ok");
        assert_eq!(redact("hit 1:abc/getMe", "1:abc"), "hit [redacted]/getMe");
    }
}
