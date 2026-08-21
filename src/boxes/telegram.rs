//! Godfather Telegram channel bind (band 167) + MCP bus (band 169) +
//! ticket ingest (band 174) + inbound poll loop (band 179).
//!
//! On-demand `getMe` + `getChat` + `getChatMemberCount` + `getChatMember` (band
//! **187** fills `tickets.member_count`; band **191** maps bot status to
//! `host` / `mate` / `guest`). Tests and `X-Telegram-Dry-Run: 1`
//! use an in-process stub (no sockets). Live Bot API is enabled only from
//! `gsv-server` / `gsv-mcp` via [`enable_live_api`]. Poller default off — no boot probe.
//! Band **179**: `gsv-server` runs [`spawn_poll_loop`] when live API is on;
//! `getUpdates` classifies `/ticket` / hook / bus JSON. Offset persists in
//! `data/telegram_offset.json` (gitignored). Cargo tests stay dry-run.
//!
//! Band 169 bus: JSON envelopes on the Godfather channel. No public webhook,
//! no Cloudflare. Dry-run uses a process-local queue.
//! Band 175: `kind:sync` envelopes on claim/done during a solo scenario walk.
//! Band 176: session lines (`solo claimed …` / `squad assigned …` / `bench gsv_dev … ns`);
//! live `sendMessage` 1/s when the token is set; cargo tests stay dry-run.
//! Band 182: Godfather posts a human line plus JSON `{v:1,kind:sync,data}` so MCP
//! clients can parse `hint` / `next` / disk / crate and steer the next drain.
//! Band 177: `run mcp bot hook up scenario` (catalog / roadmap band / plan).
//! Band 193: `kind:presence` federated jail heartbeats (host/mate; guest refused).
//! Band 194: `kind:claim` federated ticket claim on the host board (guest mute; echo skip).
//! Band 195: `kind:done` federated ticket close — remote jail finishes a claimed row
//! and the host board transitions it `in_progress` → `done` (guest mute; echo skip).
//! Band 196: `kind:reclaim` federated ticket release — lease expiry or explicit
//! reclaim posts the reopen and boards transition the row `in_progress` → `open`
//! (guest mute; echo skip).

use std::collections::VecDeque;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use axum::http::HeaderMap;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::settings::{self, SettingsFile};
use super::tickets::{self, PresenceStore};

/// Header that forces the dry-run stub even on a live server.
pub const DRY_RUN_HEADER: &str = "x-telegram-dry-run";
/// Process env that forces the stub (`1` / `true`).
pub const DRY_RUN_ENV: &str = "GSV_TELEGRAM_DRY_RUN";

const STUB_BOT: &str = "gsv_godfather_bot";
const STUB_CHAT: &str = "GSV Godfather (dry-run)";
/// Dry-run member count — distinct from bot-slot fallbacks (50 / 20).
const STUB_MEMBERS: u64 = 3;
const STUB_CHAT_KIND: &str = "channel";
const API_ROOT: &str = "https://api.telegram.org";
const MEMBER_REFRESH: Duration = Duration::from_secs(60);

static LIVE_API: AtomicBool = AtomicBool::new(false);
static POLL_LOOP: AtomicBool = AtomicBool::new(false);
static LAST_MEMBER_REFRESH: Mutex<Option<Instant>> = Mutex::new(None);
static LAST_FEDERATE: Mutex<Option<Instant>> = Mutex::new(None);
const FEDERATE_EVERY: Duration = Duration::from_secs(60);

/// Durable getUpdates offset under `data/` (gitignored via `/data/*`).
pub const OFFSET_FILE: &str = "telegram_offset.json";

/// Allow outbound Bot API (call from `gsv-server` / `gsv-mcp` mains only).
pub fn enable_live_api() {
    LIVE_API.store(true, Ordering::SeqCst);
}

/// True while [`spawn_poll_loop`] is running in this process (`gsv-server`).
pub fn poll_loop_alive() -> bool {
    POLL_LOOP.load(Ordering::SeqCst)
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
        "member_count": STUB_MEMBERS,
        "chat_kind": STUB_CHAT_KIND,
        "chat_role": "host",
        "last_probe": now_rfc3339(),
        "polling": polling,
    })
}

fn map_chat_kind(tg_type: &str) -> Option<&'static str> {
    match tg_type {
        "group" => Some("group"),
        "supergroup" => Some("supergroup"),
        "channel" => Some("channel"),
        _ => None,
    }
}

fn json_count(v: &Value) -> u64 {
    v.as_u64()
        .or_else(|| v.as_i64().and_then(|n| u64::try_from(n).ok()))
        .unwrap_or(0)
}

fn persist_probe_meta(data_dir: &Path, v: &Value) {
    if v.get("ok").and_then(Value::as_bool) != Some(true) {
        return;
    }
    if v.get("dry_run").and_then(Value::as_bool) == Some(true) {
        return;
    }
    let n = v.get("member_count").and_then(Value::as_u64).unwrap_or(0);
    let kind = v.get("chat_kind").and_then(Value::as_str);
    let role = v.get("chat_role").and_then(Value::as_str);
    let _ = settings::apply_live_chat_meta(data_dir, n, kind, role);
}

async fn fetch_member_count(client: &reqwest::Client, token: &str, channel: &str) -> u64 {
    let url = format!("{API_ROOT}/bot{token}/getChatMemberCount");
    let resp = match client.get(&url).query(&[("chat_id", channel)]).send().await {
        Ok(r) => r,
        Err(_) => return 0,
    };
    let body = match resp.json::<Value>().await {
        Ok(v) => v,
        Err(_) => return 0,
    };
    if body.get("ok").and_then(Value::as_bool) != Some(true) {
        return 0;
    }
    json_count(&body["result"])
}

async fn fetch_chat_role(
    client: &reqwest::Client,
    token: &str,
    channel: &str,
    user_id: i64,
) -> &'static str {
    if user_id == 0 {
        return "guest";
    }
    let url = format!("{API_ROOT}/bot{token}/getChatMember");
    let uid = user_id.to_string();
    let resp = match client
        .get(&url)
        .query(&[("chat_id", channel), ("user_id", uid.as_str())])
        .send()
        .await
    {
        Ok(r) => r,
        Err(_) => return "guest",
    };
    let body = match resp.json::<Value>().await {
        Ok(v) => v,
        Err(_) => return "guest",
    };
    if body.get("ok").and_then(Value::as_bool) != Some(true) {
        return "guest";
    }
    settings::map_member_status(body["result"]["status"].as_str().unwrap_or(""))
}

fn should_refresh_members() -> bool {
    let Ok(mut g) = LAST_MEMBER_REFRESH.lock() else {
        return false;
    };
    if let Some(t) = *g {
        if t.elapsed() < MEMBER_REFRESH {
            return false;
        }
    }
    *g = Some(Instant::now());
    true
}

async fn refresh_members_throttled(data_dir: &Path, token: &str, channel: &str) {
    if token.is_empty() || channel.is_empty() || !should_refresh_members() {
        return;
    }
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
    {
        Ok(c) => c,
        Err(_) => return,
    };
    let n = fetch_member_count(&client, token, channel).await;
    let _ = settings::apply_live_chat_meta(data_dir, n, None, None);
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
    persist_probe_meta(data_dir, &v);
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
    let bot_id = me_json["result"]["id"].as_i64().unwrap_or(0);
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
    let chat_kind = map_chat_kind(chat_json["result"]["type"].as_str().unwrap_or(""));
    let member_count = fetch_member_count(&client, token, channel).await;
    let chat_role = fetch_chat_role(&client, token, channel, bot_id).await;
    json!({
        "ok": true,
        "dry_run": false,
        "channel_id": channel,
        "token_set": true,
        "bot_username": bot_username,
        "chat_title": chat_title,
        "member_count": member_count,
        "chat_kind": chat_kind.unwrap_or(""),
        "chat_role": chat_role,
        "last_probe": now_rfc3339(),
        "polling": polling,
    })
}

/// Max bus envelope `body` (2 KiB).
pub const BODY_CAP: usize = 2048;

const RATE_LIMIT: Duration = Duration::from_secs(1);
const DEFAULT_POLL_LIMIT: usize = 8;
const MAX_POLL_LIMIT: usize = 32;

/// Channel-as-bus envelope (`kind` is `bus`, `sync`, `presence`, `claim`, `done`,
/// or `reclaim`).
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
    /// Machine fields for MCP-to-MCP correction (band 182). Absent on legacy bus.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<SyncData>,
}

/// Structured payload inside a `kind:sync` envelope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SyncData {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub product: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scenario: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phase: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor: Option<String>,
    #[serde(rename = "crate", default, skip_serializing_if = "Option::is_none")]
    pub crate_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disk_ok: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disk_free_gb: Option<u64>,
    /// What the next MCP should do: `work-ticket` · `claim-next` · `work-assigned` · `record-bench` · `hook-placed` · `heartbeat`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub jail_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ide: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rank_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rank_title: Option<String>,
}

/// Dry-run / test fake of one `getUpdates` item.
#[derive(Clone, Debug)]
pub struct InboundUpdate {
    pub update_id: i64,
    pub text: String,
    pub chat_id: String,
    pub chat_username: String,
    pub from: String,
    pub from_username: String,
}

struct BusInner {
    queue: VecDeque<BusEnvelope>,
    inbound: VecDeque<InboundUpdate>,
    last_send: Option<Instant>,
    last_bus_ts: String,
    last_bus_error: String,
    last_bus_ok: bool,
    last_ticket_id: String,
    last_poll_ts: String,
    last_poll_n: usize,
    last_ingest_kind: String,
    last_ingest_id: String,
    last_sync_hint: String,
    last_sync_next: String,
    last_sync_body: String,
    update_offset: i64,
}

impl BusInner {
    const fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            inbound: VecDeque::new(),
            last_send: None,
            last_bus_ts: String::new(),
            last_bus_error: String::new(),
            last_bus_ok: false,
            last_ticket_id: String::new(),
            last_poll_ts: String::new(),
            last_poll_n: 0,
            last_ingest_kind: String::new(),
            last_ingest_id: String::new(),
            last_sync_hint: String::new(),
            last_sync_next: String::new(),
            last_sync_body: String::new(),
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

/// Last Godfather MCP signal (`hint`, `next`) for squad/solo next-action.
pub fn last_signal() -> (String, String) {
    let g = bus();
    (g.last_sync_hint.clone(), g.last_sync_next.clone())
}

fn record_signal(g: &mut BusInner, env: &BusEnvelope) {
    if let Some(tid) = env.ticket_id.as_ref() {
        if !tid.is_empty() {
            g.last_ticket_id = tid.clone();
        }
    }
    g.last_sync_body = env.body.clone();
    if let Some(d) = env.data.as_ref() {
        g.last_sync_hint = d.hint.clone().unwrap_or_default();
        g.last_sync_next = d.next.clone().unwrap_or_default();
    }
}

fn merge_last_bus(v: &mut Value) {
    let (ok, ts, err, ticket, poll_ts, poll_n, ingest_kind, ingest_id, offset, hint, next, body) = {
        let g = bus();
        (
            g.last_bus_ok,
            g.last_bus_ts.clone(),
            g.last_bus_error.clone(),
            g.last_ticket_id.clone(),
            g.last_poll_ts.clone(),
            g.last_poll_n,
            g.last_ingest_kind.clone(),
            g.last_ingest_id.clone(),
            g.update_offset,
            g.last_sync_hint.clone(),
            g.last_sync_next.clone(),
            g.last_sync_body.clone(),
        )
    };
    if let Some(obj) = v.as_object_mut() {
        obj.insert("last_bus_ok".into(), json!(ok));
        obj.insert("last_bus_ts".into(), json!(ts));
        obj.insert("last_bus_error".into(), json!(err));
        obj.insert("last_ticket_id".into(), json!(ticket));
        obj.insert("poll_alive".into(), json!(poll_loop_alive()));
        obj.insert("last_poll_ts".into(), json!(poll_ts));
        obj.insert("last_poll_n".into(), json!(poll_n));
        obj.insert("last_ingest_kind".into(), json!(ingest_kind));
        obj.insert("last_ingest_id".into(), json!(ingest_id));
        obj.insert("update_offset".into(), json!(offset));
        obj.insert("last_hint".into(), json!(hint));
        obj.insert("last_next".into(), json!(next));
        obj.insert("last_body".into(), json!(body));
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

/// Serialize/validate a bus envelope. Invalid JSON or unknown `kind` → err.
pub fn parse_envelope(v: &Value) -> Result<BusEnvelope, String> {
    let mut env: BusEnvelope =
        serde_json::from_value(v.clone()).map_err(|_| "invalid envelope".to_string())?;
    if env.v != 1 {
        return Err("v must be 1".into());
    }
    if env.kind != "bus"
        && env.kind != "sync"
        && env.kind != "presence"
        && env.kind != "claim"
        && env.kind != "done"
        && env.kind != "reclaim"
    {
        return Err("kind must be bus, sync, presence, claim, done, or reclaim".into());
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
    if let Some(data) = env.data.as_mut() {
        data.product = empty_to_none(data.product.take());
        data.scenario = empty_to_none(data.scenario.take());
        data.phase = empty_to_none(data.phase.take());
        data.mode = empty_to_none(data.mode.take());
        data.actor = empty_to_none(data.actor.take());
        data.crate_version = empty_to_none(data.crate_version.take());
        data.next = empty_to_none(data.next.take());
        data.hint = empty_to_none(data.hint.take());
        data.jail_id = empty_to_none(data.jail_id.take());
        data.ide = empty_to_none(data.ide.take());
        data.agent = empty_to_none(data.agent.take());
        data.rank_id = empty_to_none(data.rank_id.take());
        data.rank_title = empty_to_none(data.rank_title.take());
    }
    if env.kind == "claim" && env.ticket_id.is_none() {
        return Err("claim requires ticket_id".into());
    }
    if env.kind == "done" && env.ticket_id.is_none() {
        return Err("done requires ticket_id".into());
    }
    if env.kind == "reclaim" && env.ticket_id.is_none() {
        return Err("reclaim requires ticket_id".into());
    }
    Ok(env)
}

/// Stable hint another MCP uses to pick the next action.
pub fn sync_hint(mode: &str, phase: &str) -> &'static str {
    match (mode.trim(), phase.trim()) {
        ("squad", "assigned") | ("squad", "claimed") => "work-assigned",
        ("bench", _) => "record-bench",
        ("hook", _) => "hook-placed",
        (_, "done") => "claim-next",
        _ => "work-ticket",
    }
}

/// Disk / crate / next-sprint snapshot for a session envelope.
pub fn collect_sync_data(
    repo_root: &Path,
    scenario: &str,
    phase: &str,
    mode: &str,
    actor: &str,
) -> SyncData {
    let disk = crate::boxes::xtask::health_disk(repo_root);
    let next = crate::boxes::vision::read_manifest(repo_root)
        .ok()
        .map(|m| m.next_sprint)
        .filter(|s| !s.is_empty());
    SyncData {
        product: Some("gsv".into()),
        scenario: empty_to_none(Some(scenario.to_string())),
        phase: empty_to_none(Some(phase.to_string())),
        mode: empty_to_none(Some(mode.to_string())),
        actor: empty_to_none(Some(actor.to_string())),
        crate_version: Some(env!("CARGO_PKG_VERSION").into()),
        next,
        disk_ok: disk.get("disk_ok").and_then(Value::as_bool),
        disk_free_gb: disk.get("disk_free_gb").and_then(Value::as_u64),
        hint: Some(sync_hint(mode, phase).into()),
        ..Default::default()
    }
}

/// Pull a bus/sync envelope out of a Godfather body (JSON, `GSV1 ` prefix, or dual line+JSON).
pub fn extract_envelope(text: &str) -> Result<BusEnvelope, String> {
    let t = text.trim();
    if t.is_empty() {
        return Err("empty".into());
    }
    let json_slice = if let Some(rest) = t.strip_prefix("GSV1 ") {
        rest.trim()
    } else if t.starts_with('{') {
        t
    } else if let Some(i) = t.find('{') {
        t[i..].trim()
    } else {
        return Err("no envelope".into());
    };
    let v: Value = serde_json::from_str(json_slice).map_err(|_| "invalid envelope".to_string())?;
    parse_envelope(&v)
}

/// Human session line plus compact JSON so MCP can parse `data`.
pub fn format_channel_message(env: &BusEnvelope) -> String {
    let json = serde_json::to_string(env).unwrap_or_else(|_| "{}".into());
    let body = env.body.trim();
    if body.is_empty() {
        return json;
    }
    let dual = format!("{body}\n{json}");
    if dual.len() > 4096 {
        json
    } else {
        dual
    }
}

/// MCP / HTTP decode: structured envelope, never `bot_token`.
pub fn decode_wire(text: &str) -> Value {
    match extract_envelope(text) {
        Ok(env) => {
            let hint = env
                .data
                .as_ref()
                .and_then(|d| d.hint.clone())
                .unwrap_or_default();
            let next = env
                .data
                .as_ref()
                .and_then(|d| d.next.clone())
                .unwrap_or_default();
            json!({
                "ok": true,
                "kind": env.kind.clone(),
                "body": env.body.clone(),
                "hint": hint,
                "next": next,
                "data": env.data.clone(),
                "envelope": env,
            })
        }
        Err(e) => json!({ "ok": false, "error": e }),
    }
}

/// Plain Godfather session line (band 176). Not `{phase} {id}`.
pub fn session_line(kind: &str, phase: &str, title: &str, worker: &str) -> String {
    let title = title.trim();
    let worker = worker.trim();
    match (kind, phase) {
        ("squad", "assigned") | ("squad", "claimed") => {
            if worker.is_empty() {
                format!("squad assigned {title}")
            } else {
                format!("squad assigned {title} to {worker}")
            }
        }
        ("squad", "done") => format!("squad done {title}"),
        ("bench", _) => {
            if title.starts_with("bench ") {
                title.to_string()
            } else {
                format!("bench gsv_dev {title}")
            }
        }
        ("hook", _) => {
            if title.starts_with("hook ") {
                title.to_string()
            } else {
                format!("hook {title}")
            }
        }
        (_, "done") => format!("solo done {title}"),
        _ => format!("solo claimed {title}"),
    }
}

/// `gsv_dev` medians as a session line. Prefers `scenario_bench.json`;
/// missing file falls back to speed-index (zeros in cargo tests).
pub fn bench_session_line(repo_root: &Path) -> String {
    let b = tickets::load_scenario_bench(repo_root);
    if b.ok || b.session_walk_ns > 0 || b.create_ns > 0 {
        return tickets::scenario_bench_line(&b);
    }
    let (create, walk, mds, enqueue) = gsv_dev_medians(repo_root);
    format!("bench gsv_dev create={create} walk={walk} mds={mds} enqueue={enqueue} session=0 ns")
}

fn gsv_dev_medians(repo_root: &Path) -> (u64, u64, u64, u64) {
    let Ok(idx) = crate::boxes::vision::read_speed_index(repo_root) else {
        return (0, 0, 0, 0);
    };
    let find = |needles: &[&str]| {
        idx.bench_history
            .iter()
            .rev()
            .find(|b| {
                let hay = format!("{} {} {}", b.bench, b.group, b.kind);
                needles.iter().any(|n| hay.contains(n))
            })
            .map(|b| b.median_ns)
            .unwrap_or(0)
    };
    (
        find(&["scenario_band_create"]),
        find(&["solo_walk_mds"]),
        find(&["mds_report"]),
        find(&["telegram_enqueue_sync"]),
    )
}

/// Queue a fingerprint-class sync envelope (claim/done). Dry-run has no 1/s
/// cap — a walk posts two envelopes per ticket plus a bench line. Live Bot API
/// is 1 message/s ([`sync_walk`]).
pub fn enqueue_sync(from: &str, ticket_id: &str, phase: &str) -> Result<BusEnvelope, String> {
    let body = session_line("solo", phase, ticket_id, "");
    enqueue_session(from, ticket_id, &body)
}

/// Queue a `kind:sync` envelope with an explicit session-line body.
pub fn enqueue_session(from: &str, ticket_id: &str, body: &str) -> Result<BusEnvelope, String> {
    enqueue_session_data(from, ticket_id, body, None)
}

/// Queue a `kind:sync` envelope with optional MCP `data` fields.
pub fn enqueue_session_data(
    from: &str,
    ticket_id: &str,
    body: &str,
    data: Option<SyncData>,
) -> Result<BusEnvelope, String> {
    let from = from.trim();
    if from.is_empty() {
        return Err("from required".into());
    }
    let ticket_id = ticket_id.trim();
    if ticket_id.is_empty() {
        return Err("ticket_id required".into());
    }
    let body = body.trim();
    if body.is_empty() {
        return Err("body required".into());
    }
    if body.len() > BODY_CAP {
        return Err("body exceeds 2 KiB".into());
    }
    let envelope = BusEnvelope {
        v: 1,
        kind: "sync".into(),
        from: from.to_string(),
        to: None,
        ticket_id: Some(ticket_id.to_string()),
        body: body.to_string(),
        data,
    };
    {
        let mut g = bus();
        g.queue.push_back(envelope.clone());
        g.last_send = Some(Instant::now());
        g.last_bus_ok = true;
        g.last_bus_ts = now_rfc3339();
        g.last_bus_error.clear();
        record_signal(&mut g, &envelope);
    }
    Ok(envelope)
}

/// Queue a `kind:presence` envelope (federated jail heartbeat).
pub fn enqueue_presence(
    from: &str,
    body: &str,
    data: Option<SyncData>,
) -> Result<BusEnvelope, String> {
    let from = from.trim();
    if from.is_empty() {
        return Err("from required".into());
    }
    let body = body.trim();
    if body.is_empty() {
        return Err("body required".into());
    }
    if body.len() > BODY_CAP {
        return Err("body exceeds 2 KiB".into());
    }
    let envelope = BusEnvelope {
        v: 1,
        kind: "presence".into(),
        from: from.to_string(),
        to: None,
        ticket_id: None,
        body: body.to_string(),
        data,
    };
    {
        let mut g = bus();
        g.queue.push_back(envelope.clone());
        g.last_send = Some(Instant::now());
        g.last_bus_ok = true;
        g.last_bus_ts = now_rfc3339();
        g.last_bus_error.clear();
        record_signal(&mut g, &envelope);
    }
    Ok(envelope)
}

/// Queue a `kind:claim` envelope (federated ticket claim on the host board).
pub fn enqueue_claim(
    from: &str,
    ticket_id: &str,
    body: &str,
    data: Option<SyncData>,
) -> Result<BusEnvelope, String> {
    let from = from.trim();
    if from.is_empty() {
        return Err("from required".into());
    }
    let ticket_id = ticket_id.trim();
    if ticket_id.is_empty() {
        return Err("claim requires ticket_id".into());
    }
    let body = body.trim();
    if body.is_empty() {
        return Err("body required".into());
    }
    if body.len() > BODY_CAP {
        return Err("body exceeds 2 KiB".into());
    }
    let envelope = BusEnvelope {
        v: 1,
        kind: "claim".into(),
        from: from.to_string(),
        to: None,
        ticket_id: Some(ticket_id.to_string()),
        body: body.to_string(),
        data,
    };
    {
        let mut g = bus();
        g.queue.push_back(envelope.clone());
        g.last_send = Some(Instant::now());
        g.last_bus_ok = true;
        g.last_bus_ts = now_rfc3339();
        g.last_bus_error.clear();
        record_signal(&mut g, &envelope);
    }
    Ok(envelope)
}

/// Host/mate heartbeat → Godfather `kind:presence`. Guest never posts. 60s throttle
/// (skipped in the cargo-test harness).
pub fn maybe_federate_presence(
    file: &SettingsFile,
    who: &tickets::ClaimedBy,
    rank_id: &str,
    rank_title: &str,
) -> bool {
    if !settings::telegram_relay_enabled(file) {
        return false;
    }
    if settings::chat_role(file) == "guest" {
        return false;
    }
    if super::update::is_cargo_test_harness() {
        return false;
    }
    if let Ok(mut g) = LAST_FEDERATE.lock() {
        if let Some(prev) = *g {
            if prev.elapsed() < FEDERATE_EVERY {
                return false;
            }
        }
        *g = Some(Instant::now());
    }
    let jail = settings::jail_id(file);
    let from = if jail.is_empty() {
        "local"
    } else {
        jail.as_str()
    };
    let data = SyncData {
        product: Some("gsv".into()),
        actor: Some(who.actor.clone()),
        ide: Some(who.ide.clone()),
        agent: Some(who.agent.clone()),
        jail_id: Some(from.to_string()),
        rank_id: empty_to_none(Some(rank_id.to_string())),
        rank_title: empty_to_none(Some(rank_title.to_string())),
        crate_version: Some(env!("CARGO_PKG_VERSION").into()),
        hint: Some("heartbeat".into()),
        ..Default::default()
    };
    let body = format!("{from} heartbeat");
    enqueue_presence(from, &body, Some(data)).is_ok()
}

/// Apply an inbound `kind:presence` envelope onto the host board.
pub fn apply_presence_envelope(
    data_dir: &Path,
    store: &tickets::PresenceStore,
    env: &BusEnvelope,
) -> bool {
    if env.kind != "presence" {
        return false;
    }
    let file = settings::load_result(data_dir).unwrap_or_default();
    let skip = settings::jail_id(&file);
    let data = env.data.clone().unwrap_or_default();
    let who = tickets::ClaimedBy {
        actor: data.actor.clone().unwrap_or_else(|| env.from.clone()),
        ide: data.ide.clone().unwrap_or_else(|| "cursor".into()),
        model: String::new(),
        agent: data.agent.clone().unwrap_or_else(|| "orchestrator".into()),
    };
    tickets::apply_remote_presence(
        store,
        data.jail_id.as_deref().unwrap_or(env.from.as_str()),
        &who,
        data.rank_id.as_deref().unwrap_or(""),
        data.rank_title.as_deref().unwrap_or(""),
        &skip,
    )
}

/// Host/mate local claim → Godfather `kind:claim`. Guest never posts. Skipped in
/// the cargo-test harness (tests call [`enqueue_claim`] / [`apply_claim_envelope`]).
pub fn maybe_federate_claim(
    file: &SettingsFile,
    who: &tickets::ClaimedBy,
    ticket_id: &str,
) -> bool {
    if !settings::telegram_relay_enabled(file) {
        return false;
    }
    if settings::chat_role(file) == "guest" {
        return false;
    }
    let ticket_id = ticket_id.trim();
    if ticket_id.is_empty() {
        return false;
    }
    if super::update::is_cargo_test_harness() {
        return false;
    }
    let jail = settings::jail_id(file);
    let from = if jail.is_empty() {
        "local"
    } else {
        jail.as_str()
    };
    let data = SyncData {
        product: Some("gsv".into()),
        actor: Some(who.actor.clone()),
        ide: Some(who.ide.clone()),
        agent: Some(who.agent.clone()),
        jail_id: Some(from.to_string()),
        crate_version: Some(env!("CARGO_PKG_VERSION").into()),
        hint: Some("federated-claim".into()),
        ..Default::default()
    };
    let body = format!("{from} claims {ticket_id}");
    enqueue_claim(from, ticket_id, &body, Some(data)).is_ok()
}

/// Queue a `kind:done` envelope (federated ticket close on the host board).
pub fn enqueue_done(
    from: &str,
    ticket_id: &str,
    body: &str,
    data: Option<SyncData>,
) -> Result<BusEnvelope, String> {
    let from = from.trim();
    if from.is_empty() {
        return Err("from required".into());
    }
    let ticket_id = ticket_id.trim();
    if ticket_id.is_empty() {
        return Err("done requires ticket_id".into());
    }
    let body = body.trim();
    if body.is_empty() {
        return Err("body required".into());
    }
    if body.len() > BODY_CAP {
        return Err("body exceeds 2 KiB".into());
    }
    let envelope = BusEnvelope {
        v: 1,
        kind: "done".into(),
        from: from.to_string(),
        to: None,
        ticket_id: Some(ticket_id.to_string()),
        body: body.to_string(),
        data,
    };
    {
        let mut g = bus();
        g.queue.push_back(envelope.clone());
        g.last_send = Some(Instant::now());
        g.last_bus_ok = true;
        g.last_bus_ts = now_rfc3339();
        g.last_bus_error.clear();
        record_signal(&mut g, &envelope);
    }
    Ok(envelope)
}

/// Queue a `kind:reclaim` envelope (federated ticket release on the host board).
pub fn enqueue_reclaim(
    from: &str,
    ticket_id: &str,
    body: &str,
    data: Option<SyncData>,
) -> Result<BusEnvelope, String> {
    let from = from.trim();
    if from.is_empty() {
        return Err("from required".into());
    }
    let ticket_id = ticket_id.trim();
    if ticket_id.is_empty() {
        return Err("reclaim requires ticket_id".into());
    }
    let body = body.trim();
    if body.is_empty() {
        return Err("body required".into());
    }
    if body.len() > BODY_CAP {
        return Err("body exceeds 2 KiB".into());
    }
    let envelope = BusEnvelope {
        v: 1,
        kind: "reclaim".into(),
        from: from.to_string(),
        to: None,
        ticket_id: Some(ticket_id.to_string()),
        body: body.to_string(),
        data,
    };
    {
        let mut g = bus();
        g.queue.push_back(envelope.clone());
        g.last_send = Some(Instant::now());
        g.last_bus_ok = true;
        g.last_bus_ts = now_rfc3339();
        g.last_bus_error.clear();
        record_signal(&mut g, &envelope);
    }
    Ok(envelope)
}

/// Host/mate local done → Godfather `kind:done`. Guest never posts. Skipped in
/// the cargo-test harness (tests call [`enqueue_done`] / [`apply_done_envelope`]).
pub fn maybe_federate_done(
    file: &SettingsFile,
    who: &tickets::ClaimedBy,
    ticket_id: &str,
    note: &str,
) -> bool {
    if !settings::telegram_relay_enabled(file) {
        return false;
    }
    if settings::chat_role(file) == "guest" {
        return false;
    }
    let ticket_id = ticket_id.trim();
    if ticket_id.is_empty() {
        return false;
    }
    if super::update::is_cargo_test_harness() {
        return false;
    }
    let jail = settings::jail_id(file);
    let from = if jail.is_empty() {
        "local"
    } else {
        jail.as_str()
    };
    let data = SyncData {
        product: Some("gsv".into()),
        actor: Some(who.actor.clone()),
        ide: Some(who.ide.clone()),
        agent: Some(who.agent.clone()),
        jail_id: Some(from.to_string()),
        crate_version: Some(env!("CARGO_PKG_VERSION").into()),
        hint: empty_to_none(Some(note.trim().to_string())),
        ..Default::default()
    };
    let body = format!("{from} done {ticket_id}");
    enqueue_done(from, ticket_id, &body, Some(data)).is_ok()
}

/// Host/mate reclaim (lease expiry or explicit) → Godfather `kind:reclaim`.
/// Guest never posts. Skipped in the cargo-test harness (tests call
/// [`enqueue_reclaim`] / [`apply_reclaim_envelope`]).
pub fn maybe_federate_reclaim(
    file: &SettingsFile,
    who: &tickets::ClaimedBy,
    ticket_id: &str,
) -> bool {
    if !settings::telegram_relay_enabled(file) {
        return false;
    }
    if settings::chat_role(file) == "guest" {
        return false;
    }
    let ticket_id = ticket_id.trim();
    if ticket_id.is_empty() {
        return false;
    }
    if super::update::is_cargo_test_harness() {
        return false;
    }
    let jail = settings::jail_id(file);
    let from = if jail.is_empty() {
        "local"
    } else {
        jail.as_str()
    };
    let data = SyncData {
        product: Some("gsv".into()),
        actor: Some(who.actor.clone()),
        ide: Some(who.ide.clone()),
        agent: Some(who.agent.clone()),
        jail_id: Some(from.to_string()),
        crate_version: Some(env!("CARGO_PKG_VERSION").into()),
        hint: Some("federated-reclaim".into()),
        ..Default::default()
    };
    let body = format!("{from} reclaims {ticket_id}");
    enqueue_reclaim(from, ticket_id, &body, Some(data)).is_ok()
}

/// Apply an inbound `kind:done` onto this jail's board. Echo of *this* jail is
/// ignored. Guest boards stay solo. Missing / non-`in_progress` rows are a no-op.
/// Ranks stay process-local: remote dones never move this jail's merit ladder.
pub fn apply_done_envelope(repo_root: &Path, data_dir: &Path, env: &BusEnvelope) -> bool {
    if env.kind != "done" {
        return false;
    }
    let Some(tid) = env
        .ticket_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    else {
        return false;
    };
    let file = settings::load_result(data_dir).unwrap_or_default();
    if settings::chat_role(&file) == "guest" {
        return false;
    }
    let skip = settings::jail_id(&file);
    if env.from.eq_ignore_ascii_case(&skip) {
        return false;
    }
    let data = env.data.clone().unwrap_or_default();
    let who = tickets::ClaimedBy {
        actor: data.actor.clone().unwrap_or_else(|| env.from.clone()),
        ide: data.ide.clone().unwrap_or_else(|| "cursor".into()),
        model: String::new(),
        agent: data.agent.clone().unwrap_or_else(|| "orchestrator".into()),
    };
    let note = data.hint.clone().unwrap_or_default();
    tickets::done_remote(repo_root, data_dir, tid, who, &note).is_ok()
}

/// Apply an inbound `kind:reclaim` onto this jail's board (`in_progress` →
/// `open`). Echo of *this* jail is ignored. Guest boards stay solo. Missing /
/// non-`in_progress` rows are a no-op. Ranks are untouched: reclaims never move
/// a merit ladder.
pub fn apply_reclaim_envelope(repo_root: &Path, data_dir: &Path, env: &BusEnvelope) -> bool {
    if env.kind != "reclaim" {
        return false;
    }
    let Some(tid) = env
        .ticket_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    else {
        return false;
    };
    let file = settings::load_result(data_dir).unwrap_or_default();
    if settings::chat_role(&file) == "guest" {
        return false;
    }
    let skip = settings::jail_id(&file);
    if env.from.eq_ignore_ascii_case(&skip) {
        return false;
    }
    let data = env.data.clone().unwrap_or_default();
    let who = tickets::ClaimedBy {
        actor: data.actor.clone().unwrap_or_else(|| env.from.clone()),
        ide: data.ide.clone().unwrap_or_else(|| "cursor".into()),
        model: String::new(),
        agent: data.agent.clone().unwrap_or_else(|| "orchestrator".into()),
    };
    tickets::reclaim_remote(repo_root, data_dir, tid, who, "federated reclaim").is_ok()
}

/// Apply an inbound `kind:claim` onto this jail's board. Echo of *this* jail
/// is ignored. Guest boards stay solo. Missing / non-open tickets are a no-op.
pub fn apply_claim_envelope(repo_root: &Path, data_dir: &Path, env: &BusEnvelope) -> bool {
    if env.kind != "claim" {
        return false;
    }
    let Some(tid) = env
        .ticket_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    else {
        return false;
    };
    let file = settings::load_result(data_dir).unwrap_or_default();
    if settings::chat_role(&file) == "guest" {
        return false;
    }
    let skip = settings::jail_id(&file);
    if env.from.eq_ignore_ascii_case(&skip) {
        return false;
    }
    let data = env.data.clone().unwrap_or_default();
    let who = tickets::ClaimedBy {
        actor: data.actor.clone().unwrap_or_else(|| env.from.clone()),
        ide: data.ide.clone().unwrap_or_else(|| "cursor".into()),
        model: String::new(),
        agent: data.agent.clone().unwrap_or_else(|| "orchestrator".into()),
    };
    match tickets::claim(repo_root, data_dir, tid, who) {
        Ok(t) => {
            let jail = data.jail_id.as_deref().unwrap_or(env.from.as_str());
            let _ = tickets::stamp_claimed_jail(repo_root, &t.id, jail);
            true
        }
        Err(_) => false,
    }
}

/// Walk open tickets, enqueue session lines, optionally live-send 1/s.
///
/// Cargo tests stay dry-run (queue only, no sockets). Live `sendMessage` runs
/// when the Bot API is enabled, a token is set, and `explicit_dry` is false.
pub async fn sync_walk(
    repo_root: &Path,
    data_dir: &Path,
    body: &Value,
    presence: Option<&tickets::PresenceStore>,
    explicit_dry: bool,
) -> Result<Value, tickets::TicketError> {
    let file = settings::load_result(data_dir).map_err(tickets::TicketError::Io)?;
    if !settings::telegram_relay_enabled(&file) {
        return Err(tickets::TicketError::BadRequest(
            "telegram-relay workflow is off".into(),
        ));
    }
    let mut report = tickets::wire_walk(repo_root, data_dir, body, presence)?;
    let from = body
        .get("from")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("solo");
    report.telegram = 0;
    let mut envelopes: Vec<BusEnvelope> = Vec::new();
    for step in &report.walked {
        let kind = if step.kind.is_empty() {
            "solo"
        } else {
            step.kind.as_str()
        };
        let line = session_line(kind, &step.phase, &step.title, &step.actor);
        let data = collect_sync_data(repo_root, &report.scenario, &step.phase, kind, &step.actor);
        if let Ok(env) = enqueue_session_data(from, &step.ticket_id, &line, Some(data)) {
            report.telegram += 1;
            envelopes.push(env);
        }
    }
    let bench = bench_session_line(repo_root);
    let bench_data = collect_sync_data(repo_root, &report.scenario, "bench", "bench", "");
    if let Ok(env) = enqueue_session_data(from, "gsv_dev", &bench, Some(bench_data)) {
        report.telegram += 1;
        report.bench = bench;
        envelopes.push(env);
    }
    if !use_stub(explicit_dry) {
        live_send_envelopes(&file, &envelopes).await;
    }
    serde_json::to_value(&report)
        .map_err(|_| tickets::TicketError::Io("walk encode failed".into()))
        .map(|mut v| {
            if let Some(obj) = v.as_object_mut() {
                obj.insert("ok".into(), json!(true));
            }
            v
        })
}

/// Place tickets from a hook phrase/source, enqueue a Godfather line, optional walk.
pub async fn sync_hook(
    repo_root: &Path,
    data_dir: &Path,
    body: &Value,
    presence: Option<&tickets::PresenceStore>,
    explicit_dry: bool,
) -> Result<Value, tickets::TicketError> {
    let (report, walk) = tickets::wire_hook(repo_root, data_dir, body)?;
    let from = body
        .get("from")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("solo");
    let file = settings::load_result(data_dir).map_err(tickets::TicketError::Io)?;
    let hook_line = format!(
        "hook {} {} n={}",
        report.source,
        report.id,
        report.tickets.len()
    );
    let mut telegram = 0usize;
    let mut envelopes: Vec<BusEnvelope> = Vec::new();
    if settings::telegram_relay_enabled(&file) {
        let tid = report
            .tickets
            .first()
            .map(|t| t.id.as_str())
            .unwrap_or("hook");
        let line = session_line("hook", "placed", &hook_line, "");
        let data = collect_sync_data(repo_root, &report.scenario, "placed", "hook", from);
        if let Ok(env) = enqueue_session_data(from, tid, &line, Some(data)) {
            telegram += 1;
            envelopes.push(env);
        }
        if !use_stub(explicit_dry) {
            live_send_envelopes(&file, &envelopes).await;
        }
    }
    let mut walked = json!([]);
    let mut bench = String::new();
    if walk {
        if !settings::telegram_relay_enabled(&file) {
            return Err(tickets::TicketError::BadRequest(
                "telegram-relay workflow is off".into(),
            ));
        }
        let walk_body = json!({
            "scenario_id": report.scenario,
            "create": false,
            "from": from,
        });
        let v = sync_walk(repo_root, data_dir, &walk_body, presence, explicit_dry).await?;
        walked = v.get("walked").cloned().unwrap_or(json!([]));
        telegram += v.get("telegram").and_then(Value::as_u64).unwrap_or(0) as usize;
        bench = v
            .get("bench")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
    }
    Ok(json!({
        "ok": true,
        "source": report.source,
        "id": report.id,
        "scenario": report.scenario,
        "tickets": report.tickets,
        "skipped": report.skipped,
        "walk": walk,
        "walked": walked,
        "telegram": telegram,
        "bench": bench,
        "dry_run": use_stub(explicit_dry),
    }))
}

/// Telegram `/ticket` or `run mcp bot hook up scenario …`.
pub async fn ingest_channel_body(
    repo_root: &Path,
    data_dir: &Path,
    explicit_dry: bool,
    args: &Value,
    presence: Option<&tickets::PresenceStore>,
) -> Value {
    let raw = args.get("body").and_then(Value::as_str).unwrap_or("");
    if tickets::parse_hook_phrase(raw).is_some() {
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
        return match sync_hook(repo_root, data_dir, args, presence, explicit_dry).await {
            Ok(v) => v,
            Err(e) => {
                record_last(false, &e.to_string());
                bus_fail(&e.to_string(), &token)
            }
        };
    }
    ticket_from_message(repo_root, data_dir, explicit_dry, args, presence)
}

async fn live_send_envelopes(file: &SettingsFile, envelopes: &[BusEnvelope]) {
    let lines: Vec<String> = envelopes.iter().map(format_channel_message).collect();
    live_send_session_lines(file, &lines).await;
}

async fn live_send_session_lines(file: &SettingsFile, lines: &[String]) {
    let token = resolved_token(file).unwrap_or_default();
    let channel = file.godfather.channel_id.trim().to_string();
    if token.is_empty() || channel.is_empty() || lines.is_empty() {
        return;
    }
    for (i, line) in lines.iter().enumerate() {
        if i > 0 {
            let wait = {
                let g = bus();
                g.last_send
                    .map(|prev| RATE_LIMIT.saturating_sub(prev.elapsed()))
                    .unwrap_or(Duration::ZERO)
            };
            if !wait.is_zero() {
                tokio::time::sleep(wait).await;
            }
        }
        let _ = send_plain_live(&token, &channel, line).await;
        let mut g = bus();
        g.last_send = Some(Instant::now());
    }
}

async fn send_plain_live(token: &str, channel: &str, text: &str) -> Value {
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
    json!({ "ok": true, "dry_run": false })
}

fn title_body(s: &str) -> (String, String) {
    let first = s.lines().next().unwrap_or(s).trim();
    let title: String = first.chars().take(80).collect();
    (title, s.to_string())
}

/// Channel-post ingest: `/ticket …` or JSON `{kind:ticket,body}`.
pub fn parse_channel_ticket(text: &str) -> Option<(String, String)> {
    let t = text.trim();
    if t.is_empty() {
        return None;
    }
    if let Ok(v) = serde_json::from_str::<Value>(t) {
        if v.get("kind").and_then(Value::as_str) != Some("ticket") {
            return None;
        }
        let body = v.get("body").and_then(Value::as_str).unwrap_or("").trim();
        if body.is_empty() {
            return None;
        }
        return Some(title_body(body));
    }
    t.strip_prefix("/ticket")
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(title_body)
}

/// Classify a Godfather channel body for the inbound poller (band 179).
///
/// MCP JSON envelopes (plain, `GSV1 `, or dual line+JSON) are `bus`. Hook
/// phrases win over tickets. Legacy plain session lines stay `skip` (echo).
pub fn classify_inbound(text: &str) -> &'static str {
    let t = text.trim();
    if t.is_empty() {
        return "skip";
    }
    if let Ok(env) = extract_envelope(t) {
        return if env.kind == "presence" {
            "presence"
        } else if env.kind == "claim" {
            "claim"
        } else if env.kind == "done" {
            "done"
        } else if env.kind == "reclaim" {
            "reclaim"
        } else {
            "bus"
        };
    }
    if is_own_session_line(t) {
        return "skip";
    }
    if tickets::parse_hook_phrase(t).is_some() {
        return "hook";
    }
    if parse_channel_ticket(t).is_some() {
        return "ticket";
    }
    "skip"
}

/// Outbound Godfather session lines must not be re-ingested (echo loop).
pub fn is_own_session_line(text: &str) -> bool {
    let t = text.trim();
    t.starts_with("solo claimed ")
        || t.starts_with("solo done ")
        || t.starts_with("squad assigned ")
        || t.starts_with("bench gsv_dev ")
        || (t.starts_with("hook ") && t.contains(" n="))
}

/// Queue a dry-run `getUpdates` item for [`poll_once`].
pub fn push_inbound_stub(
    update_id: i64,
    text: &str,
    chat_id: &str,
    chat_username: &str,
    from: &str,
) {
    let mut g = bus();
    g.inbound.push_back(InboundUpdate {
        update_id,
        text: text.to_string(),
        chat_id: chat_id.to_string(),
        chat_username: chat_username.to_string(),
        from: from.to_string(),
        from_username: from.to_string(),
    });
}

fn offset_path(data_dir: &Path) -> PathBuf {
    data_dir.join(OFFSET_FILE)
}

fn load_offset(data_dir: &Path) -> i64 {
    let mem = bus().update_offset;
    if mem > 0 {
        return mem;
    }
    let Ok(raw) = fs::read_to_string(offset_path(data_dir)) else {
        return 0;
    };
    serde_json::from_str::<Value>(&raw)
        .ok()
        .and_then(|v| v.get("offset").and_then(Value::as_i64))
        .unwrap_or(0)
}

fn save_offset(data_dir: &Path, offset: i64) {
    bus().update_offset = offset;
    let body = json!({ "offset": offset });
    if let Ok(s) = serde_json::to_string(&body) {
        let _ = fs::write(offset_path(data_dir), s);
    }
}

fn drain_inbound_stubs() -> Vec<InboundUpdate> {
    let mut out = Vec::new();
    let mut g = bus();
    while let Some(item) = g.inbound.pop_front() {
        out.push(item);
    }
    out
}

fn enqueue_inbound_bus(env: BusEnvelope) {
    let mut g = bus();
    g.queue.push_back(env.clone());
    g.last_bus_ok = true;
    g.last_bus_ts = now_rfc3339();
    g.last_bus_error.clear();
    record_signal(&mut g, &env);
}

/// Background `getUpdates` loop. No-op in cargo tests (`live_api` off).
///
/// Only `gsv-server` should call this so stdio `gsv-mcp` does not steal the
/// offset. Each tick no-ops unless [`poller_wanted`].
pub fn spawn_poll_loop(
    repo_root: PathBuf,
    data_dir: PathBuf,
    presence: Arc<tickets::PresenceStore>,
) {
    if !live_api_enabled() {
        return;
    }
    POLL_LOOP.store(true, Ordering::SeqCst);
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            let file = settings::load_result(&data_dir).unwrap_or_default();
            if !poller_wanted(&file) {
                continue;
            }
            if resolved_token(&file).is_none() {
                continue;
            }
            let _ = poll_once(&repo_root, &data_dir, false, Some(presence.as_ref())).await;
        }
    });
}

/// One inbound pass: dry-run stub queue or live `getUpdates`.
///
/// Classifies Godfather posts into bus / ticket / hook. Never returns `bot_token`.
pub async fn poll_once(
    repo_root: &Path,
    data_dir: &Path,
    explicit_dry: bool,
    presence: Option<&tickets::PresenceStore>,
) -> Value {
    let dry = use_stub(explicit_dry);
    let file = match settings::load_result(data_dir) {
        Ok(f) => f,
        Err(e) => {
            record_last(false, &e);
            return bus_fail(&e, "");
        }
    };
    let token = resolved_token(&file).unwrap_or_default();
    if !poller_wanted(&file) {
        let err = "poller is off (godfather.poll or telegram-relay)";
        record_last(false, err);
        return bus_fail(err, &token);
    }
    let channel = file.godfather.channel_id.trim().to_string();
    if channel.is_empty() {
        let err = "godfather channel_id is not set";
        record_last(false, err);
        return bus_fail(err, &token);
    }
    let items = if dry {
        drain_inbound_stubs()
    } else {
        if token.is_empty() {
            let err = "godfather bot token is not set";
            record_last(false, err);
            return bus_fail(err, "");
        }
        match fetch_inbound_live(&token, data_dir).await {
            Ok(v) => v,
            Err(v) => return v,
        }
    };
    let mut n_bus = 0usize;
    let mut n_ticket = 0usize;
    let mut n_hook = 0usize;
    let mut n_presence = 0usize;
    let mut n_claim = 0usize;
    let mut n_done = 0usize;
    let mut n_reclaim = 0usize;
    let mut n_skip = 0usize;
    let mut ingested = Vec::new();
    let mut max_id = load_offset(data_dir);
    for item in items {
        if item.update_id >= max_id {
            max_id = item.update_id + 1;
        }
        if item.from_username.eq_ignore_ascii_case(STUB_BOT) {
            n_skip += 1;
            continue;
        }
        if !chat_is_godfather(&channel, &item.chat_id, &item.chat_username) {
            n_skip += 1;
            continue;
        }
        let kind = classify_inbound(&item.text);
        match kind {
            "presence" => match extract_envelope(&item.text) {
                Ok(env) => {
                    enqueue_inbound_bus(env.clone());
                    if let Some(store) = presence {
                        let _ = apply_presence_envelope(data_dir, store, &env);
                    }
                    n_presence += 1;
                    ingested.push(json!({ "kind": "presence", "from": env.from }));
                }
                Err(_) => n_skip += 1,
            },
            "claim" => match extract_envelope(&item.text) {
                Ok(env) => {
                    enqueue_inbound_bus(env.clone());
                    if apply_claim_envelope(repo_root, data_dir, &env) {
                        n_claim += 1;
                        ingested.push(json!({
                            "kind": "claim",
                            "from": env.from,
                            "ticket_id": env.ticket_id,
                            "ok": true
                        }));
                    } else {
                        n_skip += 1;
                        ingested.push(json!({
                            "kind": "claim",
                            "from": env.from,
                            "ok": false
                        }));
                    }
                }
                Err(_) => n_skip += 1,
            },
            "done" => match extract_envelope(&item.text) {
                Ok(env) => {
                    enqueue_inbound_bus(env.clone());
                    if apply_done_envelope(repo_root, data_dir, &env) {
                        n_done += 1;
                        ingested.push(json!({
                            "kind": "done",
                            "from": env.from,
                            "ticket_id": env.ticket_id,
                            "ok": true
                        }));
                    } else {
                        n_skip += 1;
                        ingested.push(json!({
                            "kind": "done",
                            "from": env.from,
                            "ok": false
                        }));
                    }
                }
                Err(_) => n_skip += 1,
            },
            "reclaim" => match extract_envelope(&item.text) {
                Ok(env) => {
                    enqueue_inbound_bus(env.clone());
                    if apply_reclaim_envelope(repo_root, data_dir, &env) {
                        n_reclaim += 1;
                        ingested.push(json!({
                            "kind": "reclaim",
                            "from": env.from,
                            "ticket_id": env.ticket_id,
                            "ok": true
                        }));
                    } else {
                        n_skip += 1;
                        ingested.push(json!({
                            "kind": "reclaim",
                            "from": env.from,
                            "ok": false
                        }));
                    }
                }
                Err(_) => n_skip += 1,
            },
            "bus" => match extract_envelope(&item.text) {
                Ok(env) => {
                    enqueue_inbound_bus(env);
                    n_bus += 1;
                    ingested.push(json!({ "kind": "bus" }));
                }
                Err(_) => n_skip += 1,
            },
            "ticket" | "hook" => {
                let args = json!({
                    "from": item.from,
                    "body": item.text,
                });
                let v = ingest_channel_body(repo_root, data_dir, dry, &args, presence).await;
                let ok = v.get("ok").and_then(Value::as_bool) == Some(true);
                let id = v
                    .pointer("/ticket/id")
                    .and_then(Value::as_str)
                    .or_else(|| v.get("scenario").and_then(Value::as_str))
                    .unwrap_or("")
                    .to_string();
                if ok {
                    if kind == "ticket" {
                        n_ticket += 1;
                    } else {
                        n_hook += 1;
                    }
                    ingested.push(json!({ "kind": kind, "id": id, "ok": true }));
                    {
                        let mut g = bus();
                        g.last_ingest_kind = kind.to_string();
                        g.last_ingest_id = id;
                    }
                } else {
                    n_skip += 1;
                    ingested.push(json!({
                        "kind": kind,
                        "ok": false,
                        "error": v.get("error").cloned().unwrap_or(json!("ingest failed")),
                    }));
                }
            }
            _ => n_skip += 1,
        }
    }
    save_offset(data_dir, max_id);
    if !dry && !token.is_empty() {
        refresh_members_throttled(data_dir, &token, &channel).await;
    }
    let n = n_bus + n_ticket + n_hook + n_presence + n_claim + n_done + n_reclaim + n_skip;
    {
        let mut g = bus();
        g.last_poll_ts = now_rfc3339();
        g.last_poll_n = n;
    }
    record_last(true, "");
    json!({
        "ok": true,
        "dry_run": dry,
        "n": n,
        "bus": n_bus,
        "presence": n_presence,
        "claim": n_claim,
        "done": n_done,
        "reclaim": n_reclaim,
        "ticket": n_ticket,
        "hook": n_hook,
        "skip": n_skip,
        "ingested": ingested,
        "update_offset": max_id,
        "poll_alive": poll_loop_alive(),
    })
}

/// Explicit ticket body: channel forms plus any non-empty plain text.
pub fn parse_ticket_body(text: &str) -> Result<(String, String), String> {
    let t = text.trim();
    if t.is_empty() {
        return Err("body required".into());
    }
    if let Ok(v) = serde_json::from_str::<Value>(t) {
        return match v.get("kind").and_then(Value::as_str) {
            Some("ticket") => {
                let body = v.get("body").and_then(Value::as_str).unwrap_or("").trim();
                if body.is_empty() {
                    Err("body required".into())
                } else {
                    Ok(title_body(body))
                }
            }
            Some("bus") | Some("sync") | Some("presence") | Some("claim") | Some("done")
            | Some("reclaim") => Err("bus envelope is not a ticket".into()),
            _ => Err("kind must be ticket".into()),
        };
    }
    if let Some(pair) = parse_channel_ticket(t) {
        return Ok(pair);
    }
    Ok(title_body(t))
}

/// Ingest a Godfather message as a ticket. Solo MCP auto-claims when online.
///
/// Requires `telegram-relay` and `ticket-claim`. Never returns `bot_token`.
/// Cargo tests / dry-run write JSONL but open no sockets.
pub fn ticket_from_message(
    repo_root: &Path,
    data_dir: &Path,
    explicit_dry: bool,
    args: &Value,
    presence: Option<&PresenceStore>,
) -> Value {
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
    if !settings::ticket_claim_enabled(&file) {
        let err = "ticket-claim workflow is off";
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
    let raw = args.get("body").and_then(Value::as_str).unwrap_or("");
    let (title, body) = match parse_ticket_body(raw) {
        Ok(p) => p,
        Err(e) => {
            record_last(false, &e);
            return bus_fail(&e, &token);
        }
    };
    if title.len() + body.len() > BODY_CAP {
        let err = "body exceeds 2 KiB";
        record_last(false, err);
        return bus_fail(err, &token);
    }
    let product = opt_arg(args, "product").unwrap_or_else(|| "gsv".into());
    let ticket = match tickets::create_from_telegram(repo_root, &title, &body, &product) {
        Ok(t) => t,
        Err(e) => {
            let err = e.to_string();
            record_last(false, &err);
            return bus_fail(&err, &token);
        }
    };
    let _ = tickets::append_telegram_event(repo_root, &ticket.id, &from);
    let ticket = match presence {
        Some(store) => match tickets::try_dispatch(repo_root, data_dir, &ticket.id, store, 1) {
            Ok(Some(claimed)) => claimed,
            Ok(None) => ticket,
            Err(e) => {
                let err = e.to_string();
                record_last(false, &err);
                return bus_fail(&err, &token);
            }
        },
        None => ticket,
    };
    let envelope = BusEnvelope {
        v: 1,
        kind: "ticket".into(),
        from: from.clone(),
        to: None,
        ticket_id: Some(ticket.id.clone()),
        body,
        data: None,
    };
    {
        let mut g = bus();
        g.queue.push_back(envelope.clone());
        g.last_send = Some(Instant::now());
        g.last_bus_ok = true;
        g.last_bus_ts = now_rfc3339();
        g.last_bus_error.clear();
        record_signal(&mut g, &envelope);
    }
    json!({
        "ok": true,
        "dry_run": dry,
        "ticket": ticket,
        "envelope": envelope,
    })
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
    if !dry && settings::chat_role(&file) == "guest" {
        let err = "chat_role=guest: not a channel admin — join as a human or bind a bot admin";
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
    let kind = opt_arg(args, "kind").unwrap_or_else(|| "bus".into());
    let mut body = args
        .get("body")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    if kind == "presence" && body.trim().is_empty() {
        body = format!("{from} heartbeat");
    }
    let tid = opt_arg(args, "ticket_id").unwrap_or_default();
    if kind == "claim" {
        if tid.trim().is_empty() {
            let err = "claim requires ticket_id";
            record_last(false, err);
            return bus_fail(err, &token);
        }
        if body.trim().is_empty() {
            body = format!("{from} claims {tid}");
        }
    }
    if kind == "done" {
        if tid.trim().is_empty() {
            let err = "done requires ticket_id";
            record_last(false, err);
            return bus_fail(err, &token);
        }
        if body.trim().is_empty() {
            body = format!("{from} done {tid}");
        }
    }
    if kind == "reclaim" {
        if tid.trim().is_empty() {
            let err = "reclaim requires ticket_id";
            record_last(false, err);
            return bus_fail(err, &token);
        }
        if body.trim().is_empty() {
            body = format!("{from} reclaims {tid}");
        }
    }
    let mut built = json!({
        "v": 1,
        "kind": kind,
        "from": from,
        "to": opt_arg(args, "to"),
        "ticket_id": opt_arg(args, "ticket_id"),
        "body": body,
    });
    if kind == "presence" {
        built["data"] = json!({
            "actor": opt_arg(args, "actor"),
            "ide": opt_arg(args, "ide"),
            "agent": opt_arg(args, "agent"),
            "jail_id": opt_arg(args, "jail_id").or(Some(from.clone())),
            "rank_id": opt_arg(args, "rank_id"),
            "rank_title": opt_arg(args, "rank_title"),
            "hint": "heartbeat",
            "product": "gsv",
            "crate": env!("CARGO_PKG_VERSION"),
        });
    }
    if kind == "claim" {
        built["data"] = json!({
            "actor": opt_arg(args, "actor"),
            "ide": opt_arg(args, "ide"),
            "agent": opt_arg(args, "agent"),
            "jail_id": opt_arg(args, "jail_id").or(Some(from.clone())),
            "hint": "federated-claim",
            "product": "gsv",
            "crate": env!("CARGO_PKG_VERSION"),
        });
    }
    if kind == "done" {
        built["data"] = json!({
            "actor": opt_arg(args, "actor"),
            "ide": opt_arg(args, "ide"),
            "agent": opt_arg(args, "agent"),
            "jail_id": opt_arg(args, "jail_id").or(Some(from.clone())),
            "hint": "federated-done",
            "product": "gsv",
            "crate": env!("CARGO_PKG_VERSION"),
        });
    }
    if kind == "reclaim" {
        built["data"] = json!({
            "actor": opt_arg(args, "actor"),
            "ide": opt_arg(args, "ide"),
            "agent": opt_arg(args, "agent"),
            "jail_id": opt_arg(args, "jail_id").or(Some(from.clone())),
            "hint": "federated-reclaim",
            "product": "gsv",
            "crate": env!("CARGO_PKG_VERSION"),
        });
    }
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
    if dry || poll_loop_alive() {
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
            "dry_run": dry,
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
    bus_poll_live(&token, &channel, n, data_dir).await
}

/// Telegram `chat.id` as a decimal string (`-100…`).
fn json_id(v: Option<&Value>) -> String {
    match v {
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::String(s)) => s.clone(),
        _ => String::new(),
    }
}

fn inbound_from_tg_update(item: &Value) -> InboundUpdate {
    let update_id = item.get("update_id").and_then(Value::as_i64).unwrap_or(0);
    let text = item
        .pointer("/message/text")
        .or_else(|| item.pointer("/channel_post/text"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let chat_id = json_id(
        item.pointer("/message/chat/id")
            .or_else(|| item.pointer("/channel_post/chat/id")),
    );
    let chat_username = item
        .pointer("/message/chat/username")
        .or_else(|| item.pointer("/channel_post/chat/username"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let from_id = json_id(item.pointer("/message/from/id"));
    let from_username = item
        .pointer("/message/from/username")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let from = if !from_id.is_empty() {
        from_id
    } else if !from_username.is_empty() {
        from_username.clone()
    } else {
        "godfather".into()
    };
    InboundUpdate {
        update_id,
        text,
        chat_id,
        chat_username,
        from,
        from_username,
    }
}

async fn fetch_inbound_live(token: &str, data_dir: &Path) -> Result<Vec<InboundUpdate>, Value> {
    let offset = load_offset(data_dir);
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            let v = bus_fail(&format!("telegram client: {e}"), token);
            return Err(v);
        }
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
        Err(e) => return Err(bus_fail(&format!("getUpdates: {e}"), token)),
    };
    let body = match got.json::<Value>().await {
        Ok(v) => v,
        Err(e) => return Err(bus_fail(&format!("getUpdates body: {e}"), token)),
    };
    if body.get("ok").and_then(Value::as_bool) != Some(true) {
        let desc = body
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or("getUpdates failed");
        return Err(bus_fail(desc, token));
    }
    let mut items = Vec::new();
    if let Some(arr) = body.get("result").and_then(Value::as_array) {
        for item in arr {
            items.push(inbound_from_tg_update(item));
        }
    }
    Ok(items)
}

/// Godfather channel may be stored as `@GSV_OFFICIAL`; getUpdates uses numeric id.
fn chat_is_godfather(channel: &str, chat_id: &str, chat_username: &str) -> bool {
    let ch = channel.trim();
    if ch.is_empty() {
        return false;
    }
    if !chat_id.is_empty() && (chat_id == ch || format!("@{chat_id}") == ch) {
        return true;
    }
    let want = ch.trim_start_matches('@');
    let got = chat_username.trim().trim_start_matches('@');
    !want.is_empty() && !got.is_empty() && want.eq_ignore_ascii_case(got)
}

async fn bus_poll_live(token: &str, channel: &str, limit: usize, data_dir: &Path) -> Value {
    let offset = load_offset(data_dir);
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
            let chat_id = json_id(
                item.pointer("/message/chat/id")
                    .or_else(|| item.pointer("/channel_post/chat/id")),
            );
            let chat_username = item
                .pointer("/message/chat/username")
                .or_else(|| item.pointer("/channel_post/chat/username"))
                .and_then(Value::as_str)
                .unwrap_or("");
            if !chat_is_godfather(channel, &chat_id, chat_username) {
                continue;
            }
            if let Ok(env) = extract_envelope(text) {
                messages.push(env);
                if messages.len() >= limit {
                    break;
                }
            }
        }
    }
    save_offset(data_dir, max_id);
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
        assert!(!poll_loop_alive());
        assert_eq!(map_chat_kind("channel"), Some("channel"));
        assert_eq!(map_chat_kind("group"), Some("group"));
        assert_eq!(map_chat_kind("supergroup"), Some("supergroup"));
        assert_eq!(map_chat_kind("private"), None);
        assert_eq!(json_count(&json!(3)), 3);
        assert_eq!(json_count(&json!(-1)), 0);
    }

    #[test]
    fn classify_inbound_hook_bus_ticket_skip() {
        assert_eq!(
            classify_inbound("run mcp bot hook up scenario band 177"),
            "hook"
        );
        assert_eq!(
            classify_inbound(r#"{"v":1,"kind":"bus","from":"a","body":"x"}"#),
            "bus"
        );
        assert_eq!(classify_inbound("/ticket Fix poller"), "ticket");
        assert_eq!(
            classify_inbound(r#"{"v":1,"kind":"ticket","body":"Join"}"#),
            "ticket"
        );
        assert_eq!(classify_inbound("hello channel"), "skip");
        assert_eq!(classify_inbound("solo claimed Session: S0 disk"), "skip");
        assert_eq!(
            classify_inbound("bench gsv_dev create=1 walk=2 mds=3 enqueue=4 session=5 ns"),
            "skip"
        );
        assert!(is_own_session_line("hook band 177 n=10"));
        let dual = format!(
            "solo claimed Session: S0 disk\n{}",
            r#"{"v":1,"kind":"sync","from":"solo","ticket_id":"t-1","body":"solo claimed Session: S0 disk","data":{"hint":"work-ticket","next":"PH-S2459"}}"#
        );
        assert_eq!(classify_inbound(&dual), "bus");
        assert_eq!(
            classify_inbound(
                r#"GSV1 {"v":1,"kind":"sync","from":"solo","ticket_id":"t-1","body":"x"}"#
            ),
            "bus"
        );
    }

    #[test]
    fn extract_envelope_json_dual_and_prefix() {
        let compact = r#"{"v":1,"kind":"sync","from":"solo","ticket_id":"t-1","body":"solo claimed Session: S0 disk","data":{"hint":"work-ticket","next":"PH-S2459","product":"gsv"}}"#;
        let env = extract_envelope(compact).expect("compact");
        assert_eq!(env.kind, "sync");
        assert_eq!(
            env.data.as_ref().and_then(|d| d.hint.as_deref()),
            Some("work-ticket")
        );
        assert_eq!(
            env.data.as_ref().and_then(|d| d.next.as_deref()),
            Some("PH-S2459")
        );
        let dual = format!("solo claimed Session: S0 disk\n{compact}");
        let from_dual = extract_envelope(&dual).expect("dual");
        assert_eq!(from_dual.ticket_id.as_deref(), Some("t-1"));
        let prefixed = format!("GSV1 {compact}");
        assert!(extract_envelope(&prefixed).is_ok());
        assert!(extract_envelope("solo claimed Session: S0 disk").is_err());
        let decoded = decode_wire(&dual);
        assert_eq!(decoded["ok"], true);
        assert_eq!(decoded["hint"], "work-ticket");
        assert_eq!(decoded["next"], "PH-S2459");
        assert!(!decoded.to_string().contains("bot_token"));
    }

    #[test]
    fn enqueue_session_data_records_mcp_signal() {
        bus_reset();
        let data = SyncData {
            hint: Some("claim-next".into()),
            next: Some("PH-S2459".into()),
            ..Default::default()
        };
        enqueue_session_data("solo", "t-1", "solo done Session: close", Some(data)).unwrap();
        let g = bus();
        assert_eq!(g.last_sync_hint, "claim-next");
        assert_eq!(g.last_sync_next, "PH-S2459");
        assert_eq!(g.last_sync_body, "solo done Session: close");
        assert_eq!(g.last_ticket_id, "t-1");
    }

    #[test]
    fn format_channel_message_is_human_plus_json() {
        let env = BusEnvelope {
            v: 1,
            kind: "sync".into(),
            from: "solo".into(),
            to: None,
            ticket_id: Some("t-1".into()),
            body: "solo claimed Session: S0 disk".into(),
            data: Some(SyncData {
                hint: Some("work-ticket".into()),
                next: Some("PH-S2459".into()),
                ..Default::default()
            }),
        };
        let msg = format_channel_message(&env);
        assert!(msg.starts_with("solo claimed Session: S0 disk\n{"), "{msg}");
        let parsed = extract_envelope(&msg).expect("roundtrip");
        assert_eq!(
            parsed.data.as_ref().and_then(|d| d.hint.as_deref()),
            Some("work-ticket")
        );
    }

    #[test]
    fn sync_hint_and_collect_data() {
        assert_eq!(sync_hint("solo", "claimed"), "work-ticket");
        assert_eq!(sync_hint("solo", "done"), "claim-next");
        assert_eq!(sync_hint("squad", "assigned"), "work-assigned");
        assert_eq!(sync_hint("bench", "bench"), "record-bench");
        assert_eq!(sync_hint("hook", "placed"), "hook-placed");
        let data = collect_sync_data(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")),
            "abrakadabra-session",
            "claimed",
            "solo",
            "agent",
        );
        assert_eq!(data.product.as_deref(), Some("gsv"));
        assert_eq!(data.hint.as_deref(), Some("work-ticket"));
        assert_eq!(
            data.crate_version.as_deref(),
            Some(env!("CARGO_PKG_VERSION"))
        );
        assert_eq!(data.scenario.as_deref(), Some("abrakadabra-session"));
    }

    #[test]
    fn redact_replaces_token_only() {
        assert_eq!(redact("ok", ""), "ok");
        assert_eq!(redact("hit 1:abc/getMe", "1:abc"), "hit [redacted]/getMe");
    }

    #[test]
    fn chat_is_godfather_matches_username_or_numeric() {
        assert!(chat_is_godfather("@GSV_OFFICIAL", "-1001", "GSV_OFFICIAL"));
        assert!(chat_is_godfather("@GSV_OFFICIAL", "-1001", "gsv_official"));
        assert!(chat_is_godfather("-1001", "-1001", ""));
        assert!(!chat_is_godfather("@GSV_OFFICIAL", "-1001", "other"));
        assert!(!chat_is_godfather("@GSV_OFFICIAL", "", ""));
    }

    #[test]
    fn parse_ticket_body_slash_json_and_plain() {
        let slash = parse_ticket_body("/ticket Fix live copy").expect("slash");
        assert_eq!(slash.0, "Fix live copy");
        assert_eq!(slash.1, "Fix live copy");
        let json =
            parse_ticket_body(r#"{"v":1,"kind":"ticket","from":"cursor","body":"Join board"}"#)
                .expect("json");
        assert_eq!(json.0, "Join board");
        let plain = parse_ticket_body("plain title").expect("plain");
        assert_eq!(plain.0, "plain title");
        assert!(parse_ticket_body("").is_err());
        assert!(parse_ticket_body(r#"{"v":1,"kind":"bus","from":"a","body":"x"}"#).is_err());
        assert!(parse_channel_ticket("hello").is_none());
        assert!(parse_channel_ticket("/ticket hi").is_some());
    }

    #[test]
    fn session_line_is_plain_copy() {
        assert_eq!(
            session_line("solo", "claimed", "Session: S0 disk", ""),
            "solo claimed Session: S0 disk"
        );
        assert_eq!(
            session_line("solo", "done", "Session: close", "agent"),
            "solo done Session: close"
        );
        assert_eq!(
            session_line("squad", "assigned", "Session: squad assign", "opencode"),
            "squad assigned Session: squad assign to opencode"
        );
        assert_eq!(
            session_line("bench", "bench", "create=1 walk=2 mds=3 enqueue=4 ns", ""),
            "bench gsv_dev create=1 walk=2 mds=3 enqueue=4 ns"
        );
        assert_eq!(
            session_line("hook", "placed", "hook band 177 n=10", ""),
            "hook band 177 n=10"
        );
        let stub = bench_session_line(std::path::Path::new("/no/such/gsv-speed-index"));
        assert_eq!(
            stub,
            "bench gsv_dev create=0 walk=0 mds=0 enqueue=0 session=0 ns"
        );
    }
}
