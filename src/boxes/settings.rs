//! GSV settings + Godfather secret store (band 166).
//!
//! Public fields (channel id, co-workflows, `token_set`) vs secrets (`bot_token`).
//! Disk: `data/gsv_settings.json` (gitignored). Env `GSV_TELEGRAM_BOT_TOKEN` wins
//! over the file and is never written back unless the owner POSTs a token.
//! HTTP/MCP wires omit `bot_token`. Telegram probe is `boxes/telegram` (band 167).

use std::fmt;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Durable basename under `data/` (not served by `GET /data/{file}`).
pub const STORE_FILE: &str = "gsv_settings.json";
/// Process env that overrides a file token without being persisted.
pub const TOKEN_ENV: &str = "GSV_TELEGRAM_BOT_TOKEN";

/// Known co-workflow ids (unknown values are kept, ignored by later bands).
pub const WORKFLOW_IDS: &[&str] = &["drain", "ticket-claim", "telegram-relay", "ticket-squad"];

/// Owner control identity: channel + optional allowlist + bot token (secret).
#[derive(Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Godfather {
    #[serde(default)]
    pub channel_id: String,
    #[serde(default)]
    pub allowed_user_ids: Vec<String>,
    #[serde(default)]
    pub bot_token: String,
    /// Opt-in channel poll (band 167). Default off — always-on Galaxy does not probe.
    #[serde(default)]
    pub poll: bool,
}

impl fmt::Debug for Godfather {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Godfather")
            .field("channel_id", &self.channel_id)
            .field("allowed_user_ids", &self.allowed_user_ids)
            .field(
                "bot_token",
                &if self.bot_token.is_empty() {
                    ""
                } else {
                    "[redacted]"
                },
            )
            .field("poll", &self.poll)
            .finish()
    }
}

/// Named collaboration modes MCP may enter (v1: ids + enabled flags).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Workflows {
    #[serde(default)]
    pub enabled: Vec<String>,
}

/// Band 168: MCP/HTTP claim is allowed only when `ticket-claim` is enabled.
pub fn ticket_claim_enabled(file: &SettingsFile) -> bool {
    file.workflows.enabled.iter().any(|id| id == "ticket-claim")
}

/// Band 169: MCP/HTTP bus is allowed only when `telegram-relay` is enabled.
pub fn telegram_relay_enabled(file: &SettingsFile) -> bool {
    file.workflows
        .enabled
        .iter()
        .any(|id| id == "telegram-relay")
}

/// Band 170: squad random-assign is allowed only when `ticket-squad` is enabled.
pub fn ticket_squad_enabled(file: &SettingsFile) -> bool {
    file.workflows.enabled.iter().any(|id| id == "ticket-squad")
}

fn default_ticket_mode() -> String {
    "solo".to_string()
}

/// Ticket collaboration mode (`solo` | `squad`). Default solo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TicketsSettings {
    #[serde(default = "default_ticket_mode")]
    pub mode: String,
}

impl Default for TicketsSettings {
    fn default() -> Self {
        Self {
            mode: default_ticket_mode(),
        }
    }
}

/// Effective mode: `squad` only when stored mode is squad **and** workflow is on.
pub fn ticket_mode(file: &SettingsFile) -> &'static str {
    if file.tickets.mode.trim().eq_ignore_ascii_case("squad") && ticket_squad_enabled(file) {
        "squad"
    } else {
        "solo"
    }
}

fn default_redact() -> bool {
    true
}

/// Secret-policy flags. `redact` defaults on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Security {
    #[serde(default = "default_redact")]
    pub redact: bool,
}

impl Default for Security {
    fn default() -> Self {
        Self { redact: true }
    }
}

/// On-disk settings file. Unknown fields `#[serde(default)]` / ignored.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettingsFile {
    #[serde(default)]
    pub godfather: Godfather,
    #[serde(default)]
    pub workflows: Workflows,
    #[serde(default)]
    pub security: Security,
    #[serde(default)]
    pub tickets: TicketsSettings,
}

/// `{data_dir}/gsv_settings.json`.
pub fn store_path(data_dir: &Path) -> PathBuf {
    data_dir.join(STORE_FILE)
}

/// Non-empty `GSV_TELEGRAM_BOT_TOKEN`, if set.
pub fn env_token() -> Option<String> {
    std::env::var(TOKEN_ENV)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Load from disk. Missing file → empty defaults. Parse/IO fail → `Err`.
pub fn load_result(data_dir: &Path) -> Result<SettingsFile, String> {
    let path = store_path(data_dir);
    match fs::read_to_string(&path) {
        Ok(raw) => serde_json::from_str(&raw).map_err(|_| "settings parse failed".to_string()),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(SettingsFile::default()),
        Err(_) => Err("settings read failed".to_string()),
    }
}

/// Persist (creates `data_dir`). Does not write the env token unless it is
/// already on `file.godfather.bot_token` from an owner POST.
pub fn save(data_dir: &Path, file: &SettingsFile) -> Result<(), String> {
    fs::create_dir_all(data_dir).map_err(|_| "settings mkdir failed".to_string())?;
    let raw =
        serde_json::to_string_pretty(file).map_err(|_| "settings encode failed".to_string())?;
    fs::write(store_path(data_dir), raw).map_err(|_| "settings write failed".to_string())
}

/// Merge owner POST fields. Unknown keys ignored. Empty `bot_token` leaves
/// the stored secret unchanged.
pub fn apply_patch(file: &mut SettingsFile, patch: &Value) {
    if let Some(gf) = patch.get("godfather") {
        if let Some(ch) = gf.get("channel_id").and_then(Value::as_str) {
            file.godfather.channel_id = ch.to_string();
        }
        if let Some(ids) = gf.get("allowed_user_ids").and_then(Value::as_array) {
            file.godfather.allowed_user_ids = ids
                .iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }
        if let Some(tok) = gf.get("bot_token").and_then(Value::as_str) {
            let t = tok.trim();
            if !t.is_empty() {
                file.godfather.bot_token = t.to_string();
            }
        }
        if let Some(poll) = gf.get("poll").and_then(Value::as_bool) {
            file.godfather.poll = poll;
        }
    }
    if let Some(wf) = patch.get("workflows") {
        if let Some(enabled) = wf.get("enabled").and_then(Value::as_array) {
            file.workflows.enabled = enabled
                .iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }
    }
    if let Some(sec) = patch.get("security") {
        if let Some(redact) = sec.get("redact").and_then(Value::as_bool) {
            file.security.redact = redact;
        }
    }
    if let Some(tix) = patch.get("tickets") {
        if let Some(mode) = tix.get("mode").and_then(Value::as_str) {
            let m = mode.trim().to_ascii_lowercase();
            if m == "solo" || m == "squad" {
                file.tickets.mode = m;
            }
        }
    }
}

/// Redacted JSON: `token_set` + public Godfather fields, never `bot_token`.
pub fn redacted_wire(file: &SettingsFile, env: Option<&str>) -> Value {
    let env_set = env.map(str::trim).is_some_and(|s| !s.is_empty());
    let file_set = !file.godfather.bot_token.trim().is_empty();
    let source = if env_set {
        "env"
    } else if file_set {
        "file"
    } else {
        "none"
    };
    json!({
        "ok": true,
        "token_set": env_set || file_set,
        "source": source,
        "godfather": {
            "channel_id": file.godfather.channel_id,
            "allowed_user_ids": file.godfather.allowed_user_ids,
            "poll": file.godfather.poll,
        },
        "workflows": { "enabled": file.workflows.enabled },
        "security": { "redact": file.security.redact },
        "tickets": { "mode": file.tickets.mode },
    })
}

/// `GET /api/settings` / MCP `gsv_settings`.
pub fn wire(data_dir: &Path) -> Value {
    match load_result(data_dir) {
        Ok(file) => redacted_wire(&file, env_token().as_deref()),
        Err(e) => json!({ "ok": false, "error": e }),
    }
}

/// `POST /api/settings` — stores a posted token; response stays redacted.
pub fn wire_post(data_dir: &Path, patch: &Value) -> Value {
    let mut file = match load_result(data_dir) {
        Ok(f) => f,
        Err(e) => return json!({ "ok": false, "error": e }),
    };
    apply_patch(&mut file, patch);
    match save(data_dir, &file) {
        Ok(()) => redacted_wire(&file, env_token().as_deref()),
        Err(e) => json!({ "ok": false, "error": e }),
    }
}

/// True when a JSON value tree contains the key `bot_token`.
pub fn json_has_bot_token(v: &Value) -> bool {
    match v {
        Value::Object(m) => m.contains_key("bot_token") || m.values().any(json_has_bot_token),
        Value::Array(items) => items.iter().any(json_has_bot_token),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "gsv-settings-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = fs::create_dir_all(&dir);
        dir
    }

    #[test]
    fn missing_file_is_empty_ok() {
        let dir = temp("missing");
        let file = load_result(&dir).expect("load");
        assert_eq!(file, SettingsFile::default());
        let w = redacted_wire(&file, None);
        assert_eq!(w["ok"], true);
        assert_eq!(w["token_set"], false);
        assert_eq!(w["source"], "none");
        assert!(!json_has_bot_token(&w), "{w}");
    }

    #[test]
    fn round_trip_token_stays_on_disk_not_wire() {
        let dir = temp("round");
        let mut file = SettingsFile::default();
        file.godfather.channel_id = "-100123".into();
        file.godfather.bot_token = "123:secret-token".into();
        file.godfather.allowed_user_ids = vec!["42".into()];
        file.workflows.enabled = vec!["drain".into()];
        save(&dir, &file).expect("save");
        let loaded = load_result(&dir).expect("reload");
        assert_eq!(loaded.godfather.bot_token, "123:secret-token");
        let w = redacted_wire(&loaded, None);
        assert_eq!(w["token_set"], true);
        assert_eq!(w["source"], "file");
        assert_eq!(w["godfather"]["channel_id"], "-100123");
        assert!(!json_has_bot_token(&w), "{w}");
        let raw = serde_json::to_string(&w).expect("json");
        assert!(!raw.contains("bot_token"), "{raw}");
        assert!(!raw.contains("123:secret-token"), "{raw}");
    }

    #[test]
    fn env_overrides_file_without_writing_env() {
        let dir = temp("env");
        let mut file = SettingsFile::default();
        file.godfather.bot_token = "file-token".into();
        save(&dir, &file).expect("save");
        let w = redacted_wire(&load_result(&dir).unwrap(), Some("env-token-xyz"));
        assert_eq!(w["token_set"], true);
        assert_eq!(w["source"], "env");
        assert!(!serde_json::to_string(&w).unwrap().contains("env-token-xyz"));
        let disk = fs::read_to_string(store_path(&dir)).expect("disk");
        assert!(disk.contains("file-token"));
        assert!(!disk.contains("env-token-xyz"));
    }

    #[test]
    fn unknown_post_field_ignored() {
        let mut file = SettingsFile::default();
        apply_patch(
            &mut file,
            &json!({
                "nope": 1,
                "godfather": { "channel_id": "c1", "extra": "x" },
                "workflows": { "enabled": ["drain", "ticket-claim"] }
            }),
        );
        assert_eq!(file.godfather.channel_id, "c1");
        assert!(file.godfather.bot_token.is_empty());
        assert_eq!(
            file.workflows.enabled,
            vec!["drain".to_string(), "ticket-claim".to_string()]
        );
    }

    #[test]
    fn empty_posted_token_does_not_clear() {
        let mut file = SettingsFile::default();
        file.godfather.bot_token = "keep-me".into();
        apply_patch(&mut file, &json!({ "godfather": { "bot_token": "  " } }));
        assert_eq!(file.godfather.bot_token, "keep-me");
        apply_patch(
            &mut file,
            &json!({ "godfather": { "bot_token": "new-secret" } }),
        );
        assert_eq!(file.godfather.bot_token, "new-secret");
    }

    #[test]
    fn debug_redacts_bot_token() {
        let g = Godfather {
            channel_id: "ch".into(),
            allowed_user_ids: vec![],
            bot_token: "super-secret-bot".into(),
            poll: false,
        };
        let d = format!("{g:?}");
        assert!(d.contains("[redacted]"), "{d}");
        assert!(!d.contains("super-secret-bot"), "{d}");
    }

    #[test]
    fn default_security_redacts() {
        let raw: SettingsFile = serde_json::from_str("{}").expect("empty");
        assert!(raw.security.redact);
        assert!(WORKFLOW_IDS.contains(&"drain"));
        assert!(WORKFLOW_IDS.contains(&"ticket-squad"));
        assert_eq!(ticket_mode(&raw), "solo");
    }

    #[test]
    fn squad_mode_requires_workflow() {
        let mut file = SettingsFile::default();
        file.tickets.mode = "squad".into();
        assert_eq!(ticket_mode(&file), "solo");
        file.workflows.enabled = vec!["ticket-squad".into()];
        assert_eq!(ticket_mode(&file), "squad");
        apply_patch(&mut file, &json!({ "tickets": { "mode": "solo" } }));
        assert_eq!(file.tickets.mode, "solo");
        assert_eq!(ticket_mode(&file), "solo");
    }

    #[test]
    fn post_then_wire_omits_token() {
        let dir = temp("post");
        let w = wire_post(
            &dir,
            &json!({
                "godfather": {
                    "channel_id": "god-1",
                    "bot_token": "posted-secret"
                }
            }),
        );
        assert_eq!(w["ok"], true);
        assert_eq!(w["token_set"], true);
        assert_eq!(w["godfather"]["channel_id"], "god-1");
        assert!(!json_has_bot_token(&w));
        let get = wire(&dir);
        assert!(!json_has_bot_token(&get));
        assert_eq!(get["godfather"]["channel_id"], "god-1");
    }
}
