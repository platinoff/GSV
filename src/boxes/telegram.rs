//! Godfather Telegram channel bind (band 167) + MCP bus (band 169) +
//! ticket ingest (band 174).
//!
//! On-demand `getMe` + `getChat`. Tests and `X-Telegram-Dry-Run: 1` use an
//! in-process stub (no sockets). Live Bot API is enabled only from `gsv-server`
//! / `gsv-mcp` via [`enable_live_api`]. Poller default off — no boot probe.
//!
//! Band 169 bus: JSON envelopes on the Godfather channel. No public webhook,
//! no Cloudflare. Dry-run uses a process-local queue.
//! Band 175: `kind:sync` envelopes on claim/done during a solo scenario walk.
//! Band 176: session lines (`solo claimed …` / `squad assigned …` / `bench gsv_dev … ns`);
//! live `sendMessage` 1/s when the token is set; cargo tests stay dry-run.
//! Band 177: `run mcp bot hook up scenario` (catalog / roadmap band / plan).

use std::collections::VecDeque;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard};
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

/// Channel-as-bus envelope (`kind` is `bus` or `sync`).
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
    last_ticket_id: String,
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
            last_ticket_id: String::new(),
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
    let (ok, ts, err, ticket) = {
        let g = bus();
        (
            g.last_bus_ok,
            g.last_bus_ts.clone(),
            g.last_bus_error.clone(),
            g.last_ticket_id.clone(),
        )
    };
    if let Some(obj) = v.as_object_mut() {
        obj.insert("last_bus_ok".into(), json!(ok));
        obj.insert("last_bus_ts".into(), json!(ts));
        obj.insert("last_bus_error".into(), json!(err));
        obj.insert("last_ticket_id".into(), json!(ticket));
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
    if env.kind != "bus" && env.kind != "sync" {
        return Err("kind must be bus or sync".into());
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

/// `gsv_dev` medians as a session line. Missing speed-index → zeros (dry-run stub).
pub fn bench_session_line(repo_root: &Path) -> String {
    let (create, walk, mds, enqueue) = gsv_dev_medians(repo_root);
    format!("bench gsv_dev create={create} walk={walk} mds={mds} enqueue={enqueue} ns")
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
    };
    {
        let mut g = bus();
        g.queue.push_back(envelope.clone());
        g.last_send = Some(Instant::now());
        g.last_bus_ok = true;
        g.last_bus_ts = now_rfc3339();
        g.last_bus_error.clear();
        g.last_ticket_id = ticket_id.to_string();
    }
    Ok(envelope)
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
    let mut lines: Vec<String> = Vec::new();
    for step in &report.walked {
        let kind = if step.kind.is_empty() {
            "solo"
        } else {
            step.kind.as_str()
        };
        let line = session_line(kind, &step.phase, &step.title, &step.actor);
        if enqueue_session(from, &step.ticket_id, &line).is_ok() {
            report.telegram += 1;
            lines.push(line);
        }
    }
    let bench = bench_session_line(repo_root);
    if enqueue_session(from, "gsv_dev", &bench).is_ok() {
        report.telegram += 1;
        report.bench = bench.clone();
        lines.push(bench);
    }
    if !use_stub(explicit_dry) {
        live_send_session_lines(&file, &lines).await;
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
    let mut lines: Vec<String> = Vec::new();
    if settings::telegram_relay_enabled(&file) {
        let tid = report
            .tickets
            .first()
            .map(|t| t.id.as_str())
            .unwrap_or("hook");
        if enqueue_session(from, tid, &session_line("hook", "placed", &hook_line, "")).is_ok() {
            telegram += 1;
            lines.push(hook_line.clone());
        }
        if !use_stub(explicit_dry) {
            live_send_session_lines(&file, &lines).await;
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
            Some("bus") => Err("bus envelope is not a ticket".into()),
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
    };
    {
        let mut g = bus();
        g.queue.push_back(envelope.clone());
        g.last_send = Some(Instant::now());
        g.last_bus_ok = true;
        g.last_bus_ts = now_rfc3339();
        g.last_bus_error.clear();
        g.last_ticket_id = ticket.id.clone();
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

/// Telegram `chat.id` as a decimal string (`-100…`).
fn json_id(v: Option<&Value>) -> String {
    match v {
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::String(s)) => s.clone(),
        _ => String::new(),
    }
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
        assert_eq!(stub, "bench gsv_dev create=0 walk=0 mds=0 enqueue=0 ns");
    }
}
