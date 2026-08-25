//! Durable per-provider cooldown so MCP/OmniRouter can skip a host until
//! its rate-limit window resets.
//!
//! Source of the window sizes is [`super::catalog`] (researched 2026-08-18).
//! A 429 (or an RPM cap hit) sets `cooldown_until`; `pick_route` walks the
//! free fallback chain and then the paid chain, skipping cooling hosts.

use std::collections::BTreeMap;
use std::path::Path;

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::boxes::omni::catalog::{self, QuotaSpec};
use crate::boxes::omni::config::OmniConfig;

/// Durable snapshot basename under `data/`.
pub const STORE_FILE: &str = "omni_quota.json";

/// One provider's live window.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderQuota {
    #[serde(default)]
    pub cooldown_until: String,
    #[serde(default)]
    pub last_status: u16,
    #[serde(default)]
    pub requests: u64,
    #[serde(default)]
    pub window_start: String,
}

/// Durable store at `data/omni_quota.json`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuotaStore {
    #[serde(default)]
    pub providers: BTreeMap<String, ProviderQuota>,
}

/// A timer-aware model/provider pick for MCP auto-switch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RoutePick {
    pub provider: String,
    pub model: String,
    pub free: bool,
    pub reason: String,
    pub reset_secs: u32,
    pub cooldown_secs: u64,
    pub rust: bool,
    pub web: bool,
}

impl QuotaStore {
    pub fn load(data_dir: &Path) -> Self {
        let raw = std::fs::read_to_string(data_dir.join(STORE_FILE));
        raw.ok()
            .and_then(|s| serde_json::from_str::<QuotaStore>(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, data_dir: &Path) -> Result<(), String> {
        std::fs::create_dir_all(data_dir).map_err(|e| format!("create data dir: {e}"))?;
        let raw = serde_json::to_string_pretty(self).map_err(|e| format!("serialize: {e}"))?;
        std::fs::write(data_dir.join(STORE_FILE), raw).map_err(|e| format!("write: {e}"))
    }

    pub fn remaining_secs(&self, id: &str, now: DateTime<Utc>) -> u64 {
        let Some(row) = self.providers.get(id) else {
            return 0;
        };
        let Ok(until) = DateTime::parse_from_rfc3339(&row.cooldown_until) else {
            return 0;
        };
        let delta = until.with_timezone(&Utc) - now;
        if delta <= Duration::zero() {
            0
        } else {
            delta.num_seconds().max(0) as u64
        }
    }

    pub fn is_cooling(&self, id: &str, now: DateTime<Utc>) -> bool {
        self.remaining_secs(id, now) > 0
    }

    /// Record an upstream status. 429 starts a cooldown of `retry_after` (or
    /// the catalog `reset_secs`). A successful call increments the minute window
    /// and cools the host if RPM is exhausted.
    pub fn record(&mut self, id: &str, status: u16, retry_after: Option<u32>, now: DateTime<Utc>) {
        let spec = catalog::provider(id).map(|p| p.quota);
        let reset = spec.map(|q| q.reset_secs).unwrap_or(60).max(1);
        let rpm = spec.and_then(|q| q.rpm);
        let entry = self.providers.entry(id.to_string()).or_default();
        entry.last_status = status;
        if status == 429 {
            let secs = retry_after.unwrap_or(reset).max(1);
            entry.cooldown_until = rfc_plus(now, secs);
            return;
        }
        if !(200..300).contains(&status) {
            return;
        }
        let window_stale = entry.window_start.is_empty()
            || DateTime::parse_from_rfc3339(&entry.window_start)
                .ok()
                .map(|t| now - t.with_timezone(&Utc) >= Duration::seconds(i64::from(reset)))
                .unwrap_or(true);
        if window_stale {
            entry.window_start = now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
            entry.requests = 0;
        }
        entry.requests = entry.requests.saturating_add(1);
        if let Some(cap) = rpm {
            if entry.requests >= u64::from(cap) {
                entry.cooldown_until = rfc_plus(now, reset);
            }
        }
    }
}

fn rfc_plus(now: DateTime<Utc>, secs: u32) -> String {
    (now + Duration::seconds(i64::from(secs))).to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

/// Live quota row for the `/api/omni` wire (no secrets).
pub fn quota_wire(spec: QuotaSpec, store: &QuotaStore, id: &str, now: DateTime<Utc>) -> Value {
    json!({
        "rpm": spec.rpm,
        "rpd": spec.rpd,
        "tpm": spec.tpm,
        "reset_secs": spec.reset_secs,
        "daily_reset_secs": spec.daily_reset_secs,
        "notes": spec.notes,
        "cooling": store.is_cooling(id, now),
        "cooldown_secs": store.remaining_secs(id, now),
    })
}

/// Lane filter for a routing task: exact lanes for rust/web, and any
/// non-empty/unknown task routes the union (`rust || web`) like "any" —
/// never the accidental rust∩web slice.
fn task_matches(task: &str, m: &catalog::ModelSpec) -> bool {
    match task {
        "rust" => m.rust,
        "web" => m.web,
        _ => m.rust || m.web,
    }
}

/// Pick the next model/provider for a rust|web|any task, skipping cooldowns.
pub fn pick_route(
    cfg: &OmniConfig,
    store: &QuotaStore,
    task: &str,
    prefer_free: bool,
    now: DateTime<Utc>,
) -> Result<RoutePick, String> {
    let task = task.trim().to_ascii_lowercase();
    let mut candidates: Vec<&catalog::ModelSpec> = catalog::models()
        .iter()
        .filter(|m| task_matches(&task, m))
        .collect();
    candidates.sort_by_key(|m| {
        (
            if prefer_free { !m.free } else { m.free },
            !m.recommended,
            m.id,
        )
    });

    let mut last_err = "no enabled provider with a base_url".to_string();
    for model in candidates {
        match resolve_host(model, cfg, store, prefer_free, now) {
            Ok(provider) => {
                let q = catalog::provider(&provider)
                    .map(|p| p.quota)
                    .unwrap_or(catalog::QUOTA_PAID);
                return Ok(RoutePick {
                    provider: provider.clone(),
                    model: model.id.to_string(),
                    free: model.free || catalog::provider(&provider).is_some_and(|p| p.free),
                    reason: format!(
                        "task={task} prefer_free={prefer_free} recommended={} rust={} web={}",
                        model.recommended, model.rust, model.web
                    ),
                    reset_secs: q.reset_secs,
                    cooldown_secs: store.remaining_secs(&provider, now),
                    rust: model.rust,
                    web: model.web,
                });
            }
            Err(e) => last_err = e,
        }
    }
    Err(last_err)
}

fn resolve_host(
    model: &catalog::ModelSpec,
    cfg: &OmniConfig,
    store: &QuotaStore,
    prefer_free: bool,
    now: DateTime<Utc>,
) -> Result<String, String> {
    let available = |id: &str| {
        cfg.enabled(id) && cfg.effective_base_url(id).is_some() && !store.is_cooling(id, now)
    };
    let mut owners: Vec<&str> = catalog::find_models(model.id)
        .iter()
        .map(|m| m.provider)
        .collect();
    owners.dedup();
    owners.sort_by_key(|id| {
        let free_pen = if prefer_free {
            i32::from(!catalog::provider(id).is_some_and(|p| p.free))
        } else {
            0
        };
        (free_pen, -cfg.priority(id))
    });
    for id in &owners {
        if available(id) {
            return Ok((*id).to_string());
        }
    }
    let chains = if prefer_free {
        [
            cfg.routing.free_fallback_order.as_slice(),
            cfg.routing.fallback_order.as_slice(),
        ]
    } else {
        [
            cfg.routing.fallback_order.as_slice(),
            cfg.routing.free_fallback_order.as_slice(),
        ]
    };
    for chain in chains {
        for id in chain {
            if available(id) {
                return Ok(id.clone());
            }
        }
    }
    Err(format!(
        "no live host for {} (all cooling or missing base_url)",
        model.id
    ))
}

/// Parse `Retry-After` seconds from an upstream header map.
pub fn retry_after_secs(headers: &reqwest::header::HeaderMap, fallback: u32) -> u32 {
    headers
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.trim().parse::<u32>().ok())
        .unwrap_or(fallback)
        .max(1)
}

pub fn now_utc() -> DateTime<Utc> {
    Utc::now()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boxes::omni::config::OmniConfig;
    use serde_json::json;

    fn tmp() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("gsv-omni-quota-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        dir
    }

    #[test]
    fn cooldown_skips_provider_until_reset() {
        let mut store = QuotaStore::default();
        let t0 = DateTime::parse_from_rfc3339("2026-08-18T12:00:00Z")
            .expect("t0")
            .with_timezone(&Utc);
        store.record("groq", 429, Some(60), t0);
        assert!(store.is_cooling("groq", t0));
        assert_eq!(store.remaining_secs("groq", t0), 60);
        let t1 = t0 + Duration::seconds(61);
        assert!(!store.is_cooling("groq", t1));
    }

    #[test]
    fn rpm_cap_cools_after_burst() {
        let mut store = QuotaStore::default();
        let t0 = DateTime::parse_from_rfc3339("2026-08-18T12:00:00Z")
            .expect("t0")
            .with_timezone(&Utc);
        let rpm = catalog::provider("groq").unwrap().quota.rpm.unwrap_or(30);
        for _ in 0..rpm {
            store.record("groq", 200, None, t0);
        }
        assert!(store.is_cooling("groq", t0), "rpm {rpm} should cool");
    }

    #[test]
    fn pick_route_skips_cooling_free_host() {
        let mut cfg = OmniConfig::default();
        cfg.apply(&json!({
            "provider": {
                "groq": { "base_url": "http://127.0.0.1:9/v1", "priority": 100 },
                "openrouter": { "base_url": "http://127.0.0.1:8/v1", "priority": 50 },
            }
        }))
        .expect("apply");
        let mut store = QuotaStore::default();
        let now = DateTime::parse_from_rfc3339("2026-08-18T12:00:00Z")
            .expect("now")
            .with_timezone(&Utc);
        store.record("groq", 429, Some(60), now);
        let pick = pick_route(&cfg, &store, "rust", true, now).expect("pick");
        assert_ne!(pick.provider, "groq");
        assert!(pick.rust);
    }

    #[test]
    fn task_matches_unknown_task_is_union_not_intersection() {
        // Minimal spec doubles for the flag check (same crate, pub fields).
        let spec = |rust: bool, web: bool| catalog::ModelSpec {
            id: "x",
            name: "x",
            provider: "p",
            context_window: None,
            max_output: None,
            free: true,
            recommended: false,
            tier: "t",
            rust,
            web,
            clients: &[],
        };
        // Unknown / empty tasks route the union like "any".
        for task in ["code", "", "any", "CODE"] {
            assert!(task_matches(task, &spec(true, false)), "{task} rust-only");
            assert!(task_matches(task, &spec(false, true)), "{task} web-only");
            assert!(!task_matches(task, &spec(false, false)), "{task} neither");
        }
        // Exact lanes stay exact.
        assert!(!task_matches("rust", &spec(false, true)));
        assert!(!task_matches("web", &spec(true, false)));
    }

    #[test]
    fn pick_route_unknown_task_equals_any() {
        let cfg = OmniConfig::default();
        let store = QuotaStore::default();
        let now = DateTime::parse_from_rfc3339("2026-08-18T12:00:00Z")
            .expect("now")
            .with_timezone(&Utc);
        let any = pick_route(&cfg, &store, "any", true, now).expect("any pick");
        let unknown = pick_route(&cfg, &store, "code", true, now).expect("unknown-task pick");
        // Same lane semantics (reason intentionally echoes the raw task).
        assert_eq!(unknown.provider, any.provider);
        assert_eq!(unknown.model, any.model);
        assert_eq!(unknown.rust, any.rust);
        assert_eq!(unknown.web, any.web);
    }

    #[test]
    fn save_roundtrip() {
        let dir = tmp();
        let mut store = QuotaStore::default();
        store.record("nvidia", 429, Some(30), Utc::now());
        store.save(&dir).expect("save");
        let loaded = QuotaStore::load(&dir);
        assert!(loaded.providers.contains_key("nvidia"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
