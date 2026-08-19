//! Godfather Telegram channel bind (band 167) + MCP bus (band 169).
//!
//! On-demand `getMe` + `getChat`. Tests and `X-Telegram-Dry-Run: 1` use an
//! in-process stub (no sockets). Live Bot API is enabled only from `gsv-server`
//! / `gsv-mcp` via [`enable_live_api`]. Poller default off — no boot probe.
//!
//! Band 169 bus: JSON envelopes on the Godfather channel. No public webhook,
//! no Cloudflare. Dry-run uses a process-local queue.

use std::collections::VecDeque;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

use axum::http::HeaderMap;
use serde::{Deserialize, Serialize};
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
    let mut v = match settings::load_result(data_dir) {
        Ok(file) => status_loaded(&file, dry_run).await,
        Err(e) => fail(&e, "", false, false, use_stub(dry_run), ""),
    };
    merge_last_bus(&mut v);
    v
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

/// Max bus envelope `body` (2 KiB).
pub const BODY_CAP: usize = 2048;

const RATE_LIMIT: Duration = Duration::from_secs(1);
const DEFAULT_POLL_LIMIT: usize = 8;
const MAX_POLL_LIMIT: usize = 32;

/// Channel-as-bus envelope (`kind` other than `bus` is rejected in v1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BusEnvelope {
    pub v: u32,
    pub kind: String,
    pub from: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ticket_id: Option<String>,
    pub body: String,
}

struct BusInner {
    queue: VecDeque<BusEnvelope>,
    last_send: Option<Instant>,
    last_bus_ts: String,
    last_bus_error: String,
    last_bus_ok: bool,
    update_offset: i64,
}

impl BusInner {
    const fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            last_send: None,
            last_bus_ts: String::new(),
            last_bus_error: String::new(),
            last_bus_ok: false,
            update_offset: 0,
        }
    }
}

fn bus() -> MutexGuard<'static, BusInner> {
    static BUS: Mutex<BusInner> = Mutex::new(BusInner::new());
    BUS.lock().unwrap_or_else(|e| e.into_inner())
}

/// Drop the in-memory queue, rate-limit clock, and last-bus card fields.
pub fn bus_reset() {
    let mut g = bus();
    *g = BusInner::new();
}

/// Allow a second dry-run send in the same test process (rate-limit is 1/s).
pub fn bus_clear_rate_limit() {
    bus().last_send = None;
}

fn merge_last_bus(v: &mut Value) {
    let (ok, ts, err) = {
        let g = bus();
        (
            g.last_bus_ok,
            g.last_bus_ts.clone(),
            g.last_bus_error.clone(),
        )
    };
    if let Some(obj) = v.as_object_mut() {
        obj.insert("last_bus_ok".into(), json!(ok));
        obj.insert("last_bus_ts".into(), json!(ts));
        obj.insert("last_bus_error".into(), json!(err));
    }
}

fn record_last(ok: bool, error: &str) {
    let mut g = bus();
    g.last_bus_ok = ok;
    g.last_bus_ts = now_rfc3339();
    g.last_bus_error = error.to_string();
}

fn bus_fail(error: &str, token: &str) -> Value {
    json!({
        "ok": false,
        "error": redact(error, token),
    })
}

fn opt_arg(args: &Value, key: &str) -> Option<String> {
    args.get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn empty_to_none(v: Option<String>) -> Option<String> {
    v.and_then(|s| {
        let t = s.trim();
        if t.is_empty() {
            None
        } else {
            Some(t.to_string())
        }
    })
}

/// Serialize/validate a bus envelope. Invalid JSON or `kind != bus` → err.
pub fn parse_envelope(v: &Value) -> Result<BusEnvelope, String> {
    let mut env: BusEnvelope =
        serde_json::from_value(v.clone()).map_err(|_| "invalid envelope".to_string())?;
    if env.v != 1 {
        return Err("v must be 1".into());
    }
    if env.kind != "bus" {
        return Err("kind must be bus".into());
    }
    env.from = env.from.trim().to_string();
    if env.from.is_empty() {
        return Err("from required".into());
    }
    env.to = empty_to_none(env.to);
    env.ticket_id = empty_to_none(env.ticket_id);
    if env.body.len() > BODY_CAP {
        return Err("body exceeds 2 KiB".into());
    }
    Ok(env)
}

fn clamp_limit(limit: Option<usize>) -> usize {
    limit.unwrap_or(DEFAULT_POLL_LIMIT).clamp(1, MAX_POLL_LIMIT)
}

fn allowlisted(file: &SettingsFile, from: &str) -> bool {
    if file.godfather.allowed_user_ids.is_empty() {
        return true;
    }
    file.godfather
        .allowed_user_ids
        .iter()
        .any(|id| id.trim() == from)
}

/// `POST /api/telegram/bus` / MCP `gsv_telegram_bus_send`.
pub async fn bus_send(data_dir: &Path, explicit_dry: bool, args: &Value) -> Value {
    let dry = use_stub(explicit_dry);
    let file = match settings::load_result(data_dir) {
        Ok(f) => f,
        Err(e) => {
            record_last(false, &e);
            return bus_fail(&e, "");
        }
    };
    let token = resolved_token(&file).unwrap_or_default();
    if !settings::telegram_relay_enabled(&file) {
        let err = "telegram-relay workflow is off";
        record_last(false, err);
        return bus_fail(err, &token);
    }
    let from = opt_arg(args, "from").unwrap_or_default();
    if from.is_empty() {
        let err = "from required";
        record_last(false, err);
        return bus_fail(err, &token);
    }
    if !allowlisted(&file, &from) {
        let err = "from is not allowlisted";
        record_last(false, err);
        return bus_fail(err, &token);
    }
    let body = args.get("body").and_then(Value::as_str).unwrap_or("");
    let built = json!({
        "v": 1,
        "kind": "bus",
        "from": from,
        "to": opt_arg(args, "to"),
        "ticket_id": opt_arg(args, "ticket_id"),
        "body": body,
    });
    let envelope = match parse_envelope(&built) {
        Ok(e) => e,
        Err(e) => {
            record_last(false, &e);
            return bus_fail(&e, &token);
        }
    };
    {
        let g = bus();
        if let Some(prev) = g.last_send {
            if prev.elapsed() < RATE_LIMIT {
                let err = "rate limited";
                drop(g);
                record_last(false, err);
                return bus_fail(err, &token);
            }
        }
    }
    if dry {
        {
            let mut g = bus();
            g.queue.push_back(envelope.clone());
            g.last_send = Some(Instant::now());
            g.last_bus_ok = true;
            g.last_bus_ts = now_rfc3339();
            g.last_bus_error.clear();
        }
        return json!({
            "ok": true,
            "dry_run": true,
            "envelope": envelope,
        });
    }
    let channel = file.godfather.channel_id.trim().to_string();
    if channel.is_empty() {
        let err = "godfather channel_id is not set";
        record_last(false, err);
        return bus_fail(err, &token);
    }
    if token.is_empty() {
        let err = "godfather bot token is not set";
        record_last(false, err);
        return bus_fail(err, "");
    }
    let v = bus_send_live(&token, &channel, &envelope).await;
    if v.get("ok").and_then(Value::as_bool) == Some(true) {
        let mut g = bus();
        g.last_send = Some(Instant::now());
        g.last_bus_ok = true;
        g.last_bus_ts = now_rfc3339();
        g.last_bus_error.clear();
    } else {
        let err = v
            .get("error")
            .and_then(Value::as_str)
            .unwrap_or("send failed");
        record_last(false, err);
    }
    v
}

async fn bus_send_live(token: &str, channel: &str, envelope: &BusEnvelope) -> Value {
    let text = match serde_json::to_string(envelope) {
        Ok(t) => t,
        Err(_) => return bus_fail("envelope encode failed", token),
    };
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
    {
        Ok(c) => c,
        Err(e) => return bus_fail(&format!("telegram client: {e}"), token),
    };
    let url = format!("{API_ROOT}/bot{token}/sendMessage");
    let sent = match client
        .post(&url)
        .json(&json!({ "chat_id": channel, "text": text }))
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => return bus_fail(&format!("sendMessage: {e}"), token),
    };
    let body = match sent.json::<Value>().await {
        Ok(v) => v,
        Err(e) => return bus_fail(&format!("sendMessage body: {e}"), token),
    };
    if body.get("ok").and_then(Value::as_bool) != Some(true) {
        let desc = body
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or("sendMessage failed");
        return bus_fail(desc, token);
    }
    json!({
        "ok": true,
        "dry_run": false,
        "envelope": envelope,
    })
}

/// `GET /api/telegram/bus` / MCP `gsv_telegram_bus_poll`.
pub async fn bus_poll(data_dir: &Path, explicit_dry: bool, limit: Option<usize>) -> Value {
    let dry = use_stub(explicit_dry);
    let n = clamp_limit(limit);
    let file = match settings::load_result(data_dir) {
        Ok(f) => f,
        Err(e) => return bus_fail(&e, ""),
    };
    let token = resolved_token(&file).unwrap_or_default();
    if !settings::telegram_relay_enabled(&file) {
        return bus_fail("telegram-relay workflow is off", &token);
    }
    if dry {
        let mut messages = Vec::new();
        {
            let mut g = bus();
            for _ in 0..n {
                match g.queue.pop_front() {
                    Some(e) => messages.push(e),
                    None => break,
                }
            }
        }
        return json!({
            "ok": true,
            "dry_run": true,
            "messages": messages,
        });
    }
    let channel = file.godfather.channel_id.trim().to_string();
    if channel.is_empty() {
        return bus_fail("godfather channel_id is not set", &token);
    }
    if token.is_empty() {
        return bus_fail("godfather bot token is not set", "");
    }
    bus_poll_live(&token, &channel, n).await
}

async fn bus_poll_live(token: &str, channel: &str, limit: usize) -> Value {
    let offset = bus().update_offset;
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
    {
        Ok(c) => c,
        Err(e) => return bus_fail(&format!("telegram client: {e}"), token),
    };
    let url = format!("{API_ROOT}/bot{token}/getUpdates");
    let offset_s = offset.to_string();
    let got = match client
        .get(&url)
        .query(&[("offset", offset_s.as_str()), ("timeout", "0")])
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => return bus_fail(&format!("getUpdates: {e}"), token),
    };
    let body = match got.json::<Value>().await {
        Ok(v) => v,
        Err(e) => return bus_fail(&format!("getUpdates body: {e}"), token),
    };
    if body.get("ok").and_then(Value::as_bool) != Some(true) {
        let desc = body
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or("getUpdates failed");
        return bus_fail(desc, token);
    }
    let mut messages = Vec::new();
    let mut max_id = offset;
    if let Some(arr) = body.get("result").and_then(Value::as_array) {
        for item in arr {
            if let Some(id) = item.get("update_id").and_then(Value::as_i64) {
                if id >= max_id {
                    max_id = id + 1;
                }
            }
            let text = item
                .pointer("/message/text")
                .or_else(|| item.pointer("/channel_post/text"))
                .and_then(Value::as_str)
                .unwrap_or("");
            let chat = match item
                .pointer("/message/chat/id")
                .or_else(|| item.pointer("/channel_post/chat/id"))
            {
                Some(Value::Number(n)) => n.to_string(),
                Some(Value::String(s)) => s.clone(),
                _ => String::new(),
            };
            if !chat.is_empty() && chat != channel {
                continue;
            }
            let Ok(parsed) = serde_json::from_str::<Value>(text) else {
                continue;
            };
            if let Ok(env) = parse_envelope(&parsed) {
                messages.push(env);
                if messages.len() >= limit {
                    break;
                }
            }
        }
    }
    bus().update_offset = max_id;
    json!({
        "ok": true,
        "dry_run": false,
        "messages": messages,
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
