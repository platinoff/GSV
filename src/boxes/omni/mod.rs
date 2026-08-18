//! OmniRouter box — Rust AI proxy/router with a shared model catalog.
//!
//! Box 9: router for AI providers researched 2026-08-18 for **Rust + web**
//! across OmniRouter, Cursor, OpenCode, and Grok. Recommended: Grok 4.6,
//! GPT-5.2 Codex, Claude Sonnet 4.6, Gemini 3 Pro, Kimi K2.7 Code, GPT-5.3 Codex.
//! Each provider carries a quota window (`reset_secs`) so MCP can skip a host
//! until the free-tier timer elapses (`gsv_omni_route` / `GET /api/omni/route`).
//!
//! Endpoints:
//! - `GET /api/omni` — overview wire (providers, models, clients, quotas, routing)
//! - `GET /api/omni/route` — timer-aware next pick (`task=rust|web|any`)
//! - `GET /api/omni/config` · `POST /api/omni/config` — read (redacted) / tune
//! - `GET /api/omni/v1/models` — OpenAI-compatible model list
//! - `POST /api/omni/v1/chat/completions` — OpenAI-compatible proxy
//! - `POST /api/omni/test { provider }` — connectivity check

pub mod catalog;
pub mod config;
pub mod proxy;
pub mod quota;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use serde::Serialize;
use serde_json::{json, Value};
use tokio::sync::RwLock;

use crate::vision;

pub use catalog::{ClientSpec, ModelSpec, ProviderSpec, QuotaSpec};
pub use config::{OmniConfig, ProviderConfig, RoutingConfig};
pub use proxy::select_provider;
pub use quota::{pick_route, QuotaStore, RoutePick};

/// Canonical box name.
pub const OMNI_ROUTER_NAME: &str = "OmniRouter";

/// Shared OmniRouter runtime: durable config + HTTP client.
#[derive(Clone)]
pub struct OmniRouter {
    /// Durable data dir (`GSV/data/`).
    pub data_dir: Arc<PathBuf>,
    /// Outbound HTTP client for upstream requests.
    pub client: reqwest::Client,
    /// Tuned config (toml-backed, lock-guarded).
    pub config: Arc<RwLock<OmniConfig>>,
    /// Live cooldown windows (toml-adjacent JSON, no secrets).
    pub quota: Arc<RwLock<QuotaStore>>,
}

impl OmniRouter {
    /// Build a router from the durable config at `data_dir`.
    pub fn new(data_dir: &Path) -> Self {
        let config = OmniConfig::load(data_dir);
        let quota = QuotaStore::load(data_dir);
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            data_dir: Arc::new(data_dir.to_path_buf()),
            client,
            config: Arc::new(RwLock::new(config)),
            quota: Arc::new(RwLock::new(quota)),
        }
    }

    /// Persist the current config (best effort; logs on failure).
    pub fn persist(&self) {
        let cfg = self
            .config
            .try_read()
            .map(|c| c.clone())
            .unwrap_or_default();
        if let Err(e) = cfg.save(&self.data_dir) {
            tracing::warn!(error = %e, "omni config save failed");
        }
    }

    /// Persist live quota cooldowns (best effort).
    pub fn persist_quota(&self) {
        let q = self.quota.try_read().map(|c| c.clone()).unwrap_or_default();
        if let Err(e) = q.save(&self.data_dir) {
            tracing::warn!(error = %e, "omni quota save failed");
        }
    }
}

/// One provider row in the `/api/omni` wire.
#[derive(Debug, Clone, Serialize)]
pub struct ProviderWire {
    pub id: String,
    pub name: String,
    pub region: String,
    pub free: bool,
    pub base_url: String,
    pub enabled: bool,
    pub priority: i32,
    pub key_set: bool,
    pub notes: String,
    pub quota: serde_json::Value,
}

/// One model row in the `/api/omni` wire.
#[derive(Debug, Clone, Serialize)]
pub struct ModelWire {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub context_window: Option<u32>,
    pub max_output: Option<u32>,
    pub free: bool,
    pub recommended: bool,
    pub tier: String,
    pub rust: bool,
    pub web: bool,
    pub clients: Vec<String>,
}

/// `/api/omni` overview wire.
#[derive(Debug, Clone, Serialize)]
pub struct OmniWire {
    pub name: &'static str,
    pub providers: Vec<ProviderWire>,
    pub models: Vec<ModelWire>,
    pub recommended: Vec<ModelWire>,
    pub routing: Value,
    pub clients: Vec<Value>,
    pub researched_at: &'static str,
    pub config_path: String,
    pub generated_at: String,
    /// OmniRouter always uses GSV `data/omni.toml` keys (not the selected VDT product).
    pub account_product: &'static str,
    /// Currently selected VDT product id, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_product: Option<String>,
}

/// Build the overview wire from the current router config.
pub async fn wire(omni: &OmniRouter, selected: Option<&str>) -> OmniWire {
    let cfg = omni.config.read().await.clone();
    let store = omni.quota.read().await.clone();
    let now = quota::now_utc();
    let providers: Vec<ProviderWire> = catalog::providers()
        .iter()
        .map(|p| {
            let pc = cfg.provider_config(p.id);
            ProviderWire {
                id: p.id.to_string(),
                name: p.name.to_string(),
                region: p.region.to_string(),
                free: p.free,
                base_url: cfg.effective_base_url(p.id).unwrap_or_default(),
                enabled: pc.enabled,
                priority: pc.priority,
                key_set: cfg.effective_api_key(p.id).is_some(),
                notes: p.notes.to_string(),
                quota: quota::quota_wire(p.quota, &store, p.id, now),
            }
        })
        .collect();
    let models: Vec<ModelWire> = catalog::models().iter().map(model_wire).collect();
    let recommended = catalog::recommended_models()
        .iter()
        .map(|m| model_wire(m))
        .collect();
    let clients: Vec<Value> = catalog::clients()
        .iter()
        .map(|c| {
            json!({
                "id": c.id,
                "name": c.name,
                "kind": c.kind,
                "notes": c.notes,
                "rust_models": c.rust_models,
                "web_models": c.web_models,
                "free_models": c.free_models,
                "quota": {
                    "rpm": c.quota.rpm,
                    "rpd": c.quota.rpd,
                    "reset_secs": c.quota.reset_secs,
                    "daily_reset_secs": c.quota.daily_reset_secs,
                    "notes": c.quota.notes,
                },
            })
        })
        .collect();
    OmniWire {
        name: OMNI_ROUTER_NAME,
        providers,
        models,
        recommended,
        routing: json!({
            "default_provider": cfg.routing.default_provider,
            "auto": cfg.routing.auto,
            "fallback_order": cfg.routing.fallback_order,
            "free_fallback_order": cfg.routing.free_fallback_order,
        }),
        clients,
        researched_at: catalog::RESEARCHED_AT,
        config_path: omni.data_dir.join("omni.toml").display().to_string(),
        generated_at: vision::rfc3339_now(),
        account_product: "gsv",
        selected_product: selected.map(str::to_string),
    }
}

/// Timer-aware next pick (`task=rust|web|any`, `prefer_free`).
pub async fn route_wire(omni: &OmniRouter, task: &str, prefer_free: bool) -> Value {
    let cfg = omni.config.read().await.clone();
    let store = omni.quota.read().await.clone();
    match pick_route(&cfg, &store, task, prefer_free, quota::now_utc()) {
        Ok(p) => json!({
            "ok": true,
            "researched_at": catalog::RESEARCHED_AT,
            "task": task,
            "prefer_free": prefer_free,
            "provider": p.provider,
            "model": p.model,
            "free": p.free,
            "reason": p.reason,
            "reset_secs": p.reset_secs,
            "cooldown_secs": p.cooldown_secs,
            "rust": p.rust,
            "web": p.web,
        }),
        Err(e) => json!({ "ok": false, "error": e }),
    }
}

fn model_wire(m: &ModelSpec) -> ModelWire {
    ModelWire {
        id: m.id.to_string(),
        name: m.name.to_string(),
        provider: m.provider.to_string(),
        context_window: m.context_window,
        max_output: m.max_output,
        free: m.free,
        recommended: m.recommended,
        tier: m.tier.to_string(),
        rust: m.rust,
        web: m.web,
        clients: m.clients.iter().map(|s| (*s).to_string()).collect(),
    }
}
