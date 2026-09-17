//! WebGPU adapter probe (swarm browser path, ticket t-1789411940089445600).
//!
//! Phones already run Telegram, so the browser tensor path needs no Termux:
//! the Mini App probe page (`/probe`, `static/probe.js`) calls
//! `navigator.gpu.requestAdapter()` and POSTs the raw adapter info + limits
//! to `POST /api/edge/webgpu`. This module holds the *server-testable*
//! logic: strict parsing of the browser payload, a conservative memory cap,
//! and the hub-profile patch (`class: "webgpu"`) that Telenetis forwards to
//! GSV `POST /api/grid/profile` (keyed by the caller's bound poolAI peer, so
//! the planner follow-up can size `browser_peers` slices from real caps).
//!
//! What this deliberately does NOT do: poolAI-side admission of unsigned
//! browser peers (poolAI PH-S740 rejects unsigned `telegram_edge`
//! `register-remote`; the planner `tensor_class("webgpu")` follow-up is
//! ticket t-1789411993875408900). Until those land, probes accumulate as hub
//! profiles with real adapter caps and zero tensor routing.

use serde_json::Value;

/// Hard cap for free-form adapter strings (vendor/arch/device/description):
/// long enough for real `requestAdapterInfo()` values, short enough that a
/// hostile client cannot bloat the durable hub profile.
pub const MAX_INFO_LEN: usize = 256;

/// Adapter identity from `requestAdapterInfo()` (new) or `adapter.info`
/// (deprecated alias). All fields optional — mobile WebViews vary.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AdapterInfo {
    pub vendor: String,
    pub architecture: String,
    pub device: String,
    pub description: String,
}

/// The limits the planner actually sizes from. Only the allocation-class
/// limits are carried: everything else in `adapter.limits` is either a
/// count or irrelevant to slice sizing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProbeLimits {
    /// `maxStorageBufferBindingSize` (bytes): largest single SSBO.
    pub max_storage_bytes: u64,
    /// `maxBufferSize` (bytes): largest single buffer.
    pub max_buffer_bytes: u64,
    /// `maxTextureDimension2D`: capability signal, not a size.
    pub max_texture_dim_2d: u64,
}

/// One browser probe report.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct WebGpuProbe {
    pub supported: bool,
    pub adapter: AdapterInfo,
    pub limits: ProbeLimits,
    /// `navigator.deviceMemory` (GiB, Chrome-only, coarse buckets). `None`
    /// when the browser does not expose it — the cap then stays 0 (unknown)
    /// rather than inventing RAM the planner would slice from.
    pub device_memory_gb: Option<f64>,
    /// Telegram platform string (`android`/`ios`/…) for the profile note.
    pub platform: String,
    /// True when the report came from a `featureLevel: "compatibility"`
    /// adapter (T2.1: old Mali/Adreno WebViews that return null for core).
    /// Absent in old probe.js builds — defaults to false.
    pub compat_mode: bool,
    /// `adapter.features.has("core-features-and-limits")` when the browser
    /// reported it; `None` for old probe.js builds without the flag.
    pub core_capable: Option<bool>,
}

fn clean_str(v: Option<&Value>) -> String {
    v.and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .chars()
        .take(MAX_INFO_LEN)
        .collect()
}

fn clean_u64(v: Option<&Value>) -> Option<u64> {
    match v {
        Some(Value::Number(n)) => n.as_u64(),
        _ => None,
    }
}

/// Parse the browser payload. Accepted shapes:
/// `{supported: false, platform?}` (probe ran, no WebGPU — recorded as
/// unsupported, no profile write) or
/// `{supported: true, adapter: {vendor?, architecture?, device?, description?},
///   limits: {maxStorageBufferBindingSize?, maxBufferSize?, maxTextureDimension2D?},
///   deviceMemoryGb?, platform?, compat?, core?}` (`compat`/`core` are T2.1
/// mode flags; absent ⇒ core attempt, unknown core capability).
///
/// Missing `supported` defaults to `true` when `adapter`/`limits` are
/// present. Non-finite or negative numbers are rejected; absent numeric
/// limits default to 0 (unknown, never a capacity claim).
pub fn parse_probe(body: &Value) -> Result<WebGpuProbe, String> {
    let has_adapter = body.get("adapter").is_some();
    let has_limits = body.get("limits").is_some();
    // An absent `supported` flag means "supported" only when the report
    // carries evidence (adapter and/or limits); a bare `{platform}` is a
    // malformed report, not an unsupported device — otherwise the gates
    // could not tell "no WebGPU" from "never probed".
    let supported = match body.get("supported").and_then(Value::as_bool) {
        Some(s) => s,
        None => {
            if !(has_adapter || has_limits) {
                return Err("probe needs supported + adapter and/or limits".to_string());
            }
            true
        }
    };
    if !supported {
        return Ok(WebGpuProbe {
            supported: false,
            platform: clean_str(body.get("platform")),
            ..Default::default()
        });
    }
    let adapter_body = body.get("adapter");
    let adapter = AdapterInfo {
        vendor: clean_str(adapter_body.and_then(|a| a.get("vendor"))),
        architecture: clean_str(adapter_body.and_then(|a| a.get("architecture"))),
        device: clean_str(adapter_body.and_then(|a| a.get("device"))),
        description: clean_str(adapter_body.and_then(|a| a.get("description"))),
    };
    let limits_body = body.get("limits");
    let mut limits = ProbeLimits::default();
    if let Some(l) = limits_body {
        if !l.is_object() {
            return Err("limits must be an object".to_string());
        }
        limits.max_storage_bytes = clean_u64(l.get("maxStorageBufferBindingSize")).unwrap_or(0);
        limits.max_buffer_bytes = clean_u64(l.get("maxBufferSize")).unwrap_or(0);
        limits.max_texture_dim_2d = clean_u64(l.get("maxTextureDimension2D")).unwrap_or(0);
    }
    if adapter_body.is_some() && !adapter_body.map(Value::is_object).unwrap_or(false) {
        return Err("adapter must be an object".to_string());
    }
    if !has_adapter && !has_limits {
        return Err("supported probe needs adapter and/or limits".to_string());
    }
    let device_memory_gb = match body.get("deviceMemoryGb") {
        None => None,
        Some(v) => match v.as_f64() {
            Some(f) if f.is_finite() && f > 0.0 && f <= 64.0 => Some(f),
            _ => return Err("deviceMemoryGb must be a finite number in (0, 64]".to_string()),
        },
    };
    Ok(WebGpuProbe {
        supported: true,
        adapter,
        limits,
        device_memory_gb,
        platform: clean_str(body.get("platform")),
        compat_mode: body.get("compat").and_then(Value::as_bool).unwrap_or(false),
        core_capable: body.get("core").and_then(Value::as_bool),
    })
}

/// Conservative device-memory cap (MiB) for the hub profile. Only
/// `navigator.deviceMemory` feeds it today: WebGPU limits describe single
/// allocations, never total RAM, so without `deviceMemory` the cap is 0
/// (unknown — the planner must skip, not guess).
pub fn memory_cap_mb(probe: &WebGpuProbe) -> u64 {
    match probe.device_memory_gb {
        Some(gb) => (gb * 1024.0).floor() as u64,
        None => 0,
    }
}

/// Largest single WebGPU allocation the adapter allows (MiB) — the slice
/// ceiling for one browser shard, from `maxStorageBufferBindingSize`.
pub fn max_single_alloc_mb(probe: &WebGpuProbe) -> u64 {
    probe.limits.max_storage_bytes / (1024 * 1024)
}

/// Max length of the adapter slug inside a profile id (hub ids stay short
/// and sortable; the full adapter string rides the profile `note`).
pub const MAX_SLUG_LEN: usize = 32;

/// Profile id for a probe: `{peer}-{adapter-slug}`. Both phones can share
/// one Telegram account (one bound peer) while carrying different GPUs —
/// keying by peer alone would let the second probe overwrite the first.
/// Falls back to architecture, then to `"gpu"`, so an empty adapter still
/// keys deterministically per peer.
pub fn profile_id(peer_id: &str, probe: &WebGpuProbe) -> String {
    let raw = if !probe.adapter.device.is_empty() {
        probe.adapter.device.as_str()
    } else if !probe.adapter.architecture.is_empty() {
        probe.adapter.architecture.as_str()
    } else {
        "gpu"
    };
    let mut slug: String = raw
        .to_ascii_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    // Collapse runs of '-' and trim edges for stable, readable ids.
    let mut clean = String::with_capacity(slug.len());
    let mut prev_dash = true;
    for c in slug.chars() {
        if c == '-' {
            if !prev_dash {
                clean.push(c);
            }
            prev_dash = true;
        } else {
            clean.push(c);
            prev_dash = false;
        }
    }
    while clean.ends_with('-') {
        clean.pop();
    }
    slug = clean;
    if slug.is_empty() {
        slug = "gpu".to_string();
    }
    if slug.len() > MAX_SLUG_LEN {
        slug.truncate(MAX_SLUG_LEN);
        while slug.ends_with('-') {
            slug.pop();
        }
    }
    format!("{}-{slug}", peer_id.trim())
}

/// Hub-profile patch for GSV `POST /api/grid/profile` (includes the `id`;
/// see `upsert_profile` — unknown classes pass through, so no GSV change is
/// needed for `class: "webgpu"`). `vram_mb` stays 0: phone UMA has no
/// discrete VRAM; the unified-memory cap rides `ram_mb`. The `mode` marker
/// (`compat`/`core`/`default`) tells the planner which WebGPU flavor sized
/// the slice without a second lookup.
pub fn profile_patch(peer_id: &str, probe: &WebGpuProbe) -> Value {
    let device = if probe.adapter.device.is_empty() {
        "unknown-adapter"
    } else {
        &probe.adapter.device
    };
    let arch = if probe.adapter.architecture.is_empty() {
        "?"
    } else {
        &probe.adapter.architecture
    };
    let mode = if probe.compat_mode {
        "compat"
    } else if probe.core_capable == Some(true) {
        "core"
    } else {
        "default"
    };
    serde_json::json!({
        "id": profile_id(peer_id, probe),
        "class": "webgpu",
        "ram_mb": memory_cap_mb(probe),
        "vram_mb": 0,
        "note": format!(
            "webgpu {device}/{arch} maxbuf:{}MB plat:{} mode:{mode}",
            max_single_alloc_mb(probe),
            if probe.platform.is_empty() { "?" } else { &probe.platform },
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn adreno_probe() -> Value {
        json!({
            "supported": true,
            "adapter": {
                "vendor": "qualcomm",
                "architecture": "adreno-740",
                "device": "Adreno (TM) 740",
                "description": ""
            },
            "limits": {
                "maxStorageBufferBindingSize": 134217728u64,
                "maxBufferSize": 268435456u64,
                "maxTextureDimension2D": 8192u64
            },
            "deviceMemoryGb": 8.0,
            "platform": "android"
        })
    }

    #[test]
    fn parses_full_adreno_probe() {
        let p = parse_probe(&adreno_probe()).expect("parses");
        assert!(p.supported);
        assert_eq!(p.adapter.vendor, "qualcomm");
        assert_eq!(p.adapter.architecture, "adreno-740");
        assert_eq!(p.limits.max_storage_bytes, 134217728);
        assert_eq!(p.device_memory_gb, Some(8.0));
        assert_eq!(memory_cap_mb(&p), 8192);
        assert_eq!(max_single_alloc_mb(&p), 128);
        // Old probe.js builds carry no mode flags.
        assert!(!p.compat_mode);
        assert_eq!(p.core_capable, None);
    }

    #[test]
    fn parses_compat_mode_probe_and_marks_patch() {
        // T2.1: Redmi-class WebView — core returned null, compat served.
        let mut report = adreno_probe();
        report["compat"] = serde_json::json!(true);
        report["core"] = serde_json::json!(false);
        let p = parse_probe(&report).expect("parses");
        assert!(p.compat_mode);
        assert_eq!(p.core_capable, Some(false));
        let patch = profile_patch("redmi-01", &p);
        assert!(
            patch["note"].as_str().unwrap().contains("mode:compat"),
            "note: {}",
            patch["note"]
        );
    }

    #[test]
    fn core_probe_marks_core_mode() {
        let mut report = adreno_probe();
        report["core"] = serde_json::json!(true);
        let p = parse_probe(&report).expect("parses");
        assert!(!p.compat_mode);
        assert_eq!(p.core_capable, Some(true));
        let patch = profile_patch("a54-01", &p);
        assert!(
            patch["note"].as_str().unwrap().contains("mode:core"),
            "note: {}",
            patch["note"]
        );
    }

    #[test]
    fn compat_flag_rejects_non_bool() {
        // `compat: "yes"` is not a bool — defaults to false, never errors the
        // whole report (old/foreign clients keep flowing).
        let mut report = adreno_probe();
        report["compat"] = serde_json::json!("yes");
        let p = parse_probe(&report).expect("parses");
        assert!(!p.compat_mode);
    }

    #[test]
    fn patch_keys_peer_and_marks_webgpu_class() {
        let p = parse_probe(&adreno_probe()).expect("parses");
        let patch = profile_patch("a54-01", &p);
        assert_eq!(patch["id"], "a54-01-adreno-tm-740");
        assert_eq!(patch["class"], "webgpu");
        assert_eq!(patch["ram_mb"], 8192);
        assert_eq!(patch["vram_mb"], 0);
        let note = patch["note"].as_str().unwrap();
        assert!(note.contains("Adreno (TM) 740"));
        assert!(note.contains("maxbuf:128MB"));
    }

    #[test]
    fn profile_id_disambiguates_two_phones_on_one_account() {
        // Same Telegram account ⇒ same bound peer; adapters differ, so the
        // second probe must not overwrite the first profile.
        let adreno = parse_probe(&adreno_probe()).expect("parses");
        let mali = parse_probe(&json!({
            "supported": true,
            "adapter": {"vendor": "arm", "architecture": "mali-g72", "device": "Mali-G72"},
            "limits": {"maxStorageBufferBindingSize": 67108864u64}
        }))
        .expect("parses");
        let a = profile_id("a54-01", &adreno);
        let m = profile_id("a54-01", &mali);
        assert_eq!(a, "a54-01-adreno-tm-740");
        assert_eq!(m, "a54-01-mali-g72");
        assert_ne!(a, m);
    }

    #[test]
    fn profile_id_falls_back_without_adapter() {
        let p = parse_probe(&json!({
            "supported": true,
            "adapter": {},
            "limits": {"maxStorageBufferBindingSize": 1024u64}
        }))
        .expect("parses");
        assert_eq!(profile_id("a54-01", &p), "a54-01-gpu");
        let arch = parse_probe(&json!({
            "supported": true,
            "adapter": {"architecture": "adreno-610"},
            "limits": {}
        }))
        .expect("parses");
        assert_eq!(profile_id("peer", &arch), "peer-adreno-610");
    }

    #[test]
    fn unsupported_probe_parses_without_profile_data() {
        let p = parse_probe(&json!({"supported": false, "platform": "ios"})).expect("parses");
        assert!(!p.supported);
        assert_eq!(p.platform, "ios");
        assert_eq!(memory_cap_mb(&p), 0);
    }

    #[test]
    fn missing_everything_is_rejected() {
        assert!(parse_probe(&json!({"platform": "android"})).is_err());
    }

    #[test]
    fn non_object_limits_rejected() {
        assert!(parse_probe(&json!({"supported": true, "limits": 5})).is_err());
    }

    #[test]
    fn nan_and_negative_device_memory_rejected() {
        assert!(parse_probe(&json!({
            "supported": true, "adapter": {}, "limits": {},
            "deviceMemoryGb": -1.0
        }))
        .is_err());
        assert!(parse_probe(&json!({
            "supported": true, "adapter": {},
            "deviceMemoryGb": 128.0
        }))
        .is_err());
    }

    #[test]
    fn unknown_memory_stays_zero_never_guessed() {
        let p = parse_probe(&json!({
            "supported": true,
            "adapter": {"device": "Mali-G72"},
            "limits": {"maxStorageBufferBindingSize": 67108864u64}
        }))
        .expect("parses");
        assert_eq!(memory_cap_mb(&p), 0);
        let patch = profile_patch("redmi-01", &p);
        assert_eq!(patch["ram_mb"], 0);
        assert!(patch["note"].as_str().unwrap().contains("Mali-G72"));
    }

    #[test]
    fn long_strings_are_capped() {
        let big = "x".repeat(2000);
        let p = parse_probe(&json!({
            "supported": true,
            "adapter": {"device": big},
            "limits": {}
        }))
        .expect("parses");
        assert_eq!(p.adapter.device.len(), MAX_INFO_LEN);
    }
}
