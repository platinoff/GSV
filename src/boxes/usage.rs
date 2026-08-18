//! Session token usage — OmniRouter + MCP bot + OmniRoute, durable across sync.
//!
//! Counts come from OpenAI-compatible `usage` on chat completions (and aliases
//! OmniRoute / Anthropic / Gemini use). Totals are keyed by MCP session,
//! `X-Gsv-Session`, or the process-wide `process` bucket. OmniRoute dashboard
//! stats are a fail-open pull (`GET {base}/api/usage/history`) merged on
//! vision-sync / `/api/usage` when not under `cargo test`.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use axum::http::HeaderMap;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::boxes::update;
use crate::vision;

/// Durable snapshot basename under `data/`.
pub const STORE_FILE: &str = "gsv_usage.json";
/// Default OmniRoute loopback (OpenAI-compatible proxy sits at `/v1`).
pub const DEFAULT_OMNIROUTE_URL: &str = "http://127.0.0.1:20128";
/// Cap retained completion events (oldest dropped).
pub const EVENT_CAP: usize = 256;
/// Cap distinct session rows (oldest `last_ts` dropped).
pub const SESSION_CAP: usize = 64;
/// Fail-open OmniRoute pull budget.
pub const OMNIROUTE_PULL_MS: u64 = 400;

/// Token counts from one completion (or an OmniRoute aggregate).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenCounts {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    #[serde(default)]
    pub cache_read_tokens: u64,
    #[serde(default)]
    pub reasoning_tokens: u64,
}

impl TokenCounts {
    pub fn total(self) -> u64 {
        self.prompt_tokens
            .saturating_add(self.completion_tokens)
            .saturating_add(self.cache_read_tokens)
            .saturating_add(self.reasoning_tokens)
    }

    pub fn is_zero(self) -> bool {
        self.total() == 0
    }
}

/// One recorded completion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageEvent {
    pub ts: String,
    pub session: String,
    pub source: String,
    pub provider: String,
    pub model: String,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
}

/// Aggregated row for one session key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionRow {
    pub session: String,
    pub source: String,
    pub provider: String,
    pub model: String,
    pub requests: u64,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
    pub last_ts: String,
}

/// Last OmniRoute dashboard pull (fail-open; never secrets).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OmniroutePull {
    pub ok: bool,
    #[serde(default)]
    pub base_url: String,
    #[serde(default)]
    pub requests: u64,
    #[serde(default)]
    pub prompt_tokens: u64,
    #[serde(default)]
    pub completion_tokens: u64,
    #[serde(default)]
    pub total_tokens: u64,
    #[serde(default)]
    pub pulled_at: String,
    #[serde(default)]
    pub error: String,
}

impl Default for OmniroutePull {
    fn default() -> Self {
        Self {
            ok: false,
            base_url: DEFAULT_OMNIROUTE_URL.to_string(),
            requests: 0,
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
            pulled_at: String::new(),
            error: String::new(),
        }
    }
}

/// Durable store at `data/gsv_usage.json`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageStore {
    #[serde(default)]
    pub events: Vec<UsageEvent>,
    #[serde(default)]
    pub sessions: BTreeMap<String, SessionRow>,
    #[serde(default)]
    pub omniroute: OmniroutePull,
}

pub fn store_path(data_dir: &Path) -> PathBuf {
    data_dir.join(STORE_FILE)
}

pub fn omniroute_base() -> String {
    std::env::var("GSV_OMNIROUTE_URL")
        .ok()
        .map(|s| s.trim().trim_end_matches('/').to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| DEFAULT_OMNIROUTE_URL.to_string())
}

pub fn omniroute_history_url(base: &str) -> String {
    format!("{}/api/usage/history", base.trim_end_matches('/'))
}

pub fn event_now(
    session: &str,
    source: &str,
    provider: &str,
    model: &str,
    counts: TokenCounts,
) -> UsageEvent {
    UsageEvent {
        ts: vision::rfc3339_now(),
        session: session.to_string(),
        source: source.to_string(),
        provider: provider.to_string(),
        model: model.to_string(),
        prompt_tokens: counts.prompt_tokens,
        completion_tokens: counts.completion_tokens,
        total_tokens: counts.total(),
    }
}

/// OpenAI `usage`, OmniRoute `input_tokens`/`output_tokens`, Gemini `usageMetadata`.
pub fn parse_usage(value: &Value) -> Option<TokenCounts> {
    if value.get("dry_run").and_then(Value::as_bool) == Some(true) {
        return None;
    }
    let usage = value
        .get("usage")
        .or_else(|| value.get("usageMetadata"))
        .or_else(|| value.get("usage_metadata"));
    let Some(u) = usage else {
        return gemini_top(value);
    };
    if u.as_object().is_some_and(|m| m.is_empty()) {
        return None;
    }
    let prompt = u64_field(u, &["prompt_tokens", "input_tokens", "promptTokenCount"]);
    let completion = u64_field(
        u,
        &["completion_tokens", "output_tokens", "candidatesTokenCount"],
    );
    let cache = u64_field(u, &["cache_read_tokens", "tokens_cache_read"]);
    let reasoning = u64_field(u, &["reasoning_tokens", "tokens_reasoning"]);
    let counts = TokenCounts {
        prompt_tokens: prompt,
        completion_tokens: completion,
        cache_read_tokens: cache,
        reasoning_tokens: reasoning,
    };
    if counts.is_zero() {
        None
    } else {
        Some(counts)
    }
}

fn gemini_top(value: &Value) -> Option<TokenCounts> {
    let prompt = u64_field(value, &["promptTokenCount"]);
    let completion = u64_field(value, &["candidatesTokenCount"]);
    let counts = TokenCounts {
        prompt_tokens: prompt,
        completion_tokens: completion,
        cache_read_tokens: 0,
        reasoning_tokens: 0,
    };
    if counts.is_zero() {
        None
    } else {
        Some(counts)
    }
}

fn u64_field(v: &Value, names: &[&str]) -> u64 {
    for name in names {
        if let Some(n) = v.get(*name).and_then(Value::as_u64) {
            return n;
        }
        if let Some(n) = v.get(*name).and_then(Value::as_i64) {
            return n.max(0) as u64;
        }
        if let Some(n) = v.get(*name).and_then(Value::as_f64) {
            return n.max(0.0) as u64;
        }
    }
    0
}

/// OmniRoute `getUsageStats()` JSON (camelCase totals).
pub fn parse_omniroute_stats(value: &Value) -> OmniroutePull {
    let requests = u64_field(value, &["totalRequests", "total_requests", "requests"]);
    let prompt = u64_field(value, &["totalPromptTokens", "total_prompt_tokens"]);
    let completion = u64_field(value, &["totalCompletionTokens", "total_completion_tokens"]);
    let err = value
        .get("error")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let ok = requests > 0 || prompt > 0 || completion > 0;
    OmniroutePull {
        ok,
        base_url: omniroute_base(),
        requests,
        prompt_tokens: prompt,
        completion_tokens: completion,
        total_tokens: prompt.saturating_add(completion),
        pulled_at: if ok {
            vision::rfc3339_now()
        } else {
            String::new()
        },
        error: if ok { String::new() } else { err },
    }
}

pub fn session_from_headers(headers: &HeaderMap) -> String {
    if let Some(id) = headers
        .get("mcp-session-id")
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| session_id_ok(s))
    {
        return format!("mcp:{id}");
    }
    headers
        .get("x-gsv-session")
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("process")
        .to_string()
}

pub fn source_from_headers(headers: &HeaderMap) -> String {
    if let Some(src) = headers
        .get("x-gsv-source")
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        return src.to_string();
    }
    if headers.get("mcp-session-id").is_some() {
        "mcp".into()
    } else {
        "omni".into()
    }
}

fn session_id_ok(id: &str) -> bool {
    let n = id.len();
    (8..=128).contains(&n) && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
}

pub fn record(store: &mut UsageStore, event: UsageEvent) {
    if event.total_tokens == 0 && event.prompt_tokens == 0 && event.completion_tokens == 0 {
        return;
    }
    let row = store
        .sessions
        .entry(event.session.clone())
        .or_insert_with(|| SessionRow {
            session: event.session.clone(),
            source: event.source.clone(),
            provider: event.provider.clone(),
            model: event.model.clone(),
            requests: 0,
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
            last_ts: event.ts.clone(),
        });
    row.requests = row.requests.saturating_add(1);
    row.prompt_tokens = row.prompt_tokens.saturating_add(event.prompt_tokens);
    row.completion_tokens = row
        .completion_tokens
        .saturating_add(event.completion_tokens);
    row.total_tokens = row.total_tokens.saturating_add(event.total_tokens);
    row.last_ts = event.ts.clone();
    row.provider = event.provider.clone();
    row.model = event.model.clone();
    row.source = event.source.clone();
    store.events.push(event);
    if store.events.len() > EVENT_CAP {
        let drop_n = store.events.len() - EVENT_CAP;
        store.events.drain(0..drop_n);
    }
    while store.sessions.len() > SESSION_CAP {
        let oldest = store
            .sessions
            .values()
            .min_by_key(|r| r.last_ts.as_str())
            .map(|r| r.session.clone());
        match oldest {
            Some(k) => {
                store.sessions.remove(&k);
            }
            None => break,
        }
    }
}

pub fn process_totals(store: &UsageStore) -> SessionRow {
    let mut row = SessionRow {
        session: "process".into(),
        source: "all".into(),
        provider: String::new(),
        model: String::new(),
        requests: 0,
        prompt_tokens: 0,
        completion_tokens: 0,
        total_tokens: 0,
        last_ts: String::new(),
    };
    for s in store.sessions.values() {
        row.requests = row.requests.saturating_add(s.requests);
        row.prompt_tokens = row.prompt_tokens.saturating_add(s.prompt_tokens);
        row.completion_tokens = row.completion_tokens.saturating_add(s.completion_tokens);
        row.total_tokens = row.total_tokens.saturating_add(s.total_tokens);
        if s.last_ts > row.last_ts {
            row.last_ts = s.last_ts.clone();
        }
    }
    row
}

pub fn load(data_dir: &Path) -> UsageStore {
    let path = store_path(data_dir);
    let Ok(text) = fs::read_to_string(&path) else {
        return UsageStore::default();
    };
    serde_json::from_str(&text).unwrap_or_default()
}

pub fn save(data_dir: &Path, store: &UsageStore) -> Result<(), String> {
    fs::create_dir_all(data_dir).map_err(|e| e.to_string())?;
    let text = serde_json::to_string_pretty(store).map_err(|e| e.to_string())?;
    fs::write(store_path(data_dir), text).map_err(|e| e.to_string())
}

/// Ensure the snapshot exists (vision-sync).
pub fn touch_snapshot(data_dir: &Path) -> String {
    let store = load(data_dir);
    let _ = save(data_dir, &store);
    store_path(data_dir).to_string_lossy().to_string()
}

pub fn apply_omniroute(store: &mut UsageStore, pull: OmniroutePull) {
    store.omniroute = pull;
}

pub fn wire(data_dir: &Path, store: &UsageStore) -> Value {
    let process = process_totals(store);
    let mut sessions: Vec<&SessionRow> = store.sessions.values().collect();
    sessions.sort_by(|a, b| b.last_ts.cmp(&a.last_ts));
    json!({
        "ok": true,
        "path": store_path(data_dir).display().to_string(),
        "process": {
            "requests": process.requests,
            "prompt_tokens": process.prompt_tokens,
            "completion_tokens": process.completion_tokens,
            "total_tokens": process.total_tokens,
            "last_ts": process.last_ts,
        },
        "sessions": sessions,
        "event_count": store.events.len() as u64,
        "omniroute": store.omniroute,
    })
}

/// Record one completion against process-local store + disk.
pub async fn record_into(state: &crate::AppState, event: UsageEvent) {
    let mut store = state.usage.write().await;
    record(&mut store, event);
    let _ = save(&state.data_dir, &store);
}

pub async fn wire_state(state: &crate::AppState) -> Value {
    let store = state.usage.read().await;
    wire(&state.data_dir, &store)
}

/// Fail-open OmniRoute pull (skipped in cargo-test harness).
pub async fn pull_omniroute(client: &reqwest::Client) -> OmniroutePull {
    if update::is_cargo_test_harness() {
        return OmniroutePull::default();
    }
    let base = omniroute_base();
    let url = omniroute_history_url(&base);
    let send = client.get(&url).send();
    match tokio::time::timeout(std::time::Duration::from_millis(OMNIROUTE_PULL_MS), send).await {
        Ok(Ok(resp)) => {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            let parsed: Value = serde_json::from_str(&body).unwrap_or(Value::Null);
            let mut pull = parse_omniroute_stats(&parsed);
            pull.base_url = base;
            if !status.is_success() && !pull.ok {
                pull.error = format!("http {}", status.as_u16());
            }
            pull
        }
        Ok(Err(e)) => OmniroutePull {
            error: e.to_string(),
            base_url: base,
            ..OmniroutePull::default()
        },
        Err(_) => OmniroutePull {
            error: "timeout".into(),
            base_url: base,
            ..OmniroutePull::default()
        },
    }
}

pub async fn merge_omniroute_pull(state: &crate::AppState) {
    if update::is_cargo_test_harness() {
        return;
    }
    let pull = pull_omniroute(&state.omni.client).await;
    if pull.ok || !pull.error.is_empty() {
        let mut store = state.usage.write().await;
        apply_omniroute(&mut store, pull);
        let _ = save(&state.data_dir, &store);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn total_adds_cache_and_reasoning() {
        let c = TokenCounts {
            prompt_tokens: 1,
            completion_tokens: 2,
            cache_read_tokens: 3,
            reasoning_tokens: 4,
        };
        assert_eq!(c.total(), 10);
    }
}
