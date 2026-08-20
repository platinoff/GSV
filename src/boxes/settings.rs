//! GSV settings + Godfather secret store (band 166).
//!
//! Public fields (channel id, co-workflows, `token_set`, jail/squad caps) vs
//! secrets (`bot_token`). Disk: `data/gsv_settings.json` (gitignored). Env
//! `GSV_TELEGRAM_BOT_TOKEN` wins over the file and is never written back unless
//! the owner POSTs a token. HTTP/MCP wires omit `bot_token`. Telegram probe is
//! `boxes/telegram` (band 167). Jail / squad cap: band 186.

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
    /// Owner override: `host` | `mate` | `guest` | `local` | empty = auto.
    #[serde(default)]
    pub role: String,
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
            .field("role", &self.role)
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

/// Default `in_progress` lease (seconds).
pub const DEFAULT_TICKET_LEASE_SECS: u64 = 300;
/// Floor for owner-configured lease.
pub const MIN_TICKET_LEASE_SECS: u64 = 60;
/// Cap for owner-configured lease.
pub const MAX_TICKET_LEASE_SECS: u64 = 3600;
/// Telegram group/supergroup bot ceiling (published limits, 2026).
pub const TG_GROUP_BOTS_MAX: u64 = 20;
/// Telegram channel admin ceiling including bots.
pub const TG_CHANNEL_ADMINS_MAX: u64 = 50;
/// Absolute MCP squad-worker clamp (Telegram supergroup membership ceiling).
pub const SQUAD_CAP_HARD_MAX: u64 = 200_000;

fn default_lease_secs() -> u64 {
    DEFAULT_TICKET_LEASE_SECS
}

fn default_chat_kind() -> String {
    "channel".to_string()
}

/// Ticket collaboration mode (`solo` | `squad`). Default solo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TicketsSettings {
    #[serde(default = "default_ticket_mode")]
    pub mode: String,
    /// `in_progress` lease length. 0 / missing → [`DEFAULT_TICKET_LEASE_SECS`].
    #[serde(default = "default_lease_secs")]
    pub lease_secs: u64,
    /// Owner override for MCP jail workers. `0` → derive from `member_count` / bot slots.
    #[serde(default)]
    pub squad_cap: u64,
    /// Godfather chat member/subscriber count (owner-set, or live
    /// `getChatMemberCount` in band 187). Not a secret.
    #[serde(default)]
    pub member_count: u64,
    /// `channel` | `group` | `supergroup`.
    #[serde(default = "default_chat_kind")]
    pub chat_kind: String,
    /// Last probed / stored role: `host` | `mate` | `guest` | `local`.
    #[serde(default)]
    pub chat_role: String,
}

impl Default for TicketsSettings {
    fn default() -> Self {
        Self {
            mode: default_ticket_mode(),
            lease_secs: default_lease_secs(),
            squad_cap: 0,
            member_count: 0,
            chat_kind: default_chat_kind(),
            chat_role: String::new(),
        }
    }
}

/// Public jail nickname. Empty on disk → wire `local`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct JailSettings {
    #[serde(default)]
    pub id: String,
}

/// Effective lease length, clamped.
pub fn ticket_lease_secs(file: &SettingsFile) -> u64 {
    let n = file.tickets.lease_secs;
    if n == 0 {
        DEFAULT_TICKET_LEASE_SECS
    } else {
        n.clamp(MIN_TICKET_LEASE_SECS, MAX_TICKET_LEASE_SECS)
    }
}

/// Effective mode: `squad` only when stored mode is squad **and** workflow is on.
/// Channel **guest** is always solo (they are not in the Godfather squad).
pub fn ticket_mode(file: &SettingsFile) -> &'static str {
    if chat_role(file) == "guest" {
        return "solo";
    }
    if file.tickets.mode.trim().eq_ignore_ascii_case("squad") && ticket_squad_enabled(file) {
        "squad"
    } else {
        "solo"
    }
}

/// Jail id for wires (`local` when unset).
pub fn jail_id(file: &SettingsFile) -> String {
    let id = file.jail.id.trim();
    if id.is_empty() {
        "local".to_string()
    } else {
        id.to_string()
    }
}

/// Normalized Godfather chat kind.
pub fn chat_kind(file: &SettingsFile) -> &str {
    match file.tickets.chat_kind.trim().to_ascii_lowercase().as_str() {
        "group" => "group",
        "supergroup" => "supergroup",
        _ => "channel",
    }
}

/// Map a stored / posted role string. `auto` / empty → `None` (derive).
pub fn normalize_chat_role(raw: &str) -> Option<&'static str> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "host" | "admin" | "administrator" | "creator" => Some("host"),
        "mate" | "member" => Some("mate"),
        "guest" => Some("guest"),
        "local" | "solo" => Some("local"),
        "auto" | "" => None,
        _ => None,
    }
}

/// Telegram `ChatMember.status` → GSV jail role.
pub fn map_member_status(status: &str) -> &'static str {
    match status.trim().to_ascii_lowercase().as_str() {
        "creator" | "administrator" => "host",
        "member" | "restricted" => "mate",
        "left" | "kicked" => "guest",
        _ => "guest",
    }
}

/// Effective jail role on the Godfather chat.
///
/// * **host** — bot is channel/group admin (or owner override).
/// * **mate** — human / bot is a member, not admin.
/// * **guest** — not a member yet; local work stays solo.
/// * **local** — no channel bound (same-machine Cursor+OpenCode squad still ok).
pub fn chat_role(file: &SettingsFile) -> &'static str {
    if let Some(r) = normalize_chat_role(&file.godfather.role) {
        return r;
    }
    if let Some(r) = normalize_chat_role(&file.tickets.chat_role) {
        return r;
    }
    let channel = !file.godfather.channel_id.trim().is_empty();
    let token = env_token().is_some() || !file.godfather.bot_token.trim().is_empty();
    if !channel {
        "local"
    } else if token {
        "host"
    } else {
        "mate"
    }
}

/// One-line join copy for Galaxy / MCP `env`.
pub fn role_hint(role: &str) -> &'static str {
    match role {
        "host" => {
            "You admin this Godfather chat. Bind the bot, poll, hook GitHub, run squad."
        }
        "mate" => {
            "Channel member. Heartbeat presence. Claim tickets. Do not share the host token. Shared bot uses from=jail_id."
        }
        "guest" => {
            "Not a channel member yet. Solo on this jail. Join as a human to become a mate. GitHub update still applies if origin is ahead."
        }
        _ => {
            "No Godfather channel bound. Local solo/squad MCP on this jail. Wire folder MCP to loopback."
        }
    }
}

/// Telegram BotFather bot/admin slots for this chat kind.
pub fn bot_slot_cap(file: &SettingsFile) -> u64 {
    match chat_kind(file) {
        "group" | "supergroup" => TG_GROUP_BOTS_MAX,
        _ => TG_CHANNEL_ADMINS_MAX,
    }
}

/// MCP jail-worker cap: owner policy = channel users (`member_count`).
pub fn squad_cap(file: &SettingsFile) -> u64 {
    let raw = if file.tickets.squad_cap > 0 {
        file.tickets.squad_cap
    } else if file.tickets.member_count > 0 {
        file.tickets.member_count
    } else {
        bot_slot_cap(file)
    };
    raw.clamp(1, SQUAD_CAP_HARD_MAX)
}

fn clamp_count(n: u64) -> u64 {
    n.min(SQUAD_CAP_HARD_MAX)
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
    #[serde(default)]
    pub jail: JailSettings,
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
        if let Some(role) = gf.get("role").and_then(Value::as_str) {
            let r = role.trim().to_ascii_lowercase();
            if r == "auto" {
                file.godfather.role.clear();
            } else if normalize_chat_role(&r).is_some() {
                file.godfather.role = r;
            }
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
        if let Some(secs) = tix.get("lease_secs").and_then(Value::as_u64) {
            file.tickets.lease_secs = if secs == 0 {
                DEFAULT_TICKET_LEASE_SECS
            } else {
                secs.clamp(MIN_TICKET_LEASE_SECS, MAX_TICKET_LEASE_SECS)
            };
        }
        if let Some(cap) = tix.get("squad_cap").and_then(Value::as_u64) {
            file.tickets.squad_cap = clamp_count(cap);
        }
        if let Some(n) = tix.get("member_count").and_then(Value::as_u64) {
            file.tickets.member_count = clamp_count(n);
        }
        if let Some(kind) = tix.get("chat_kind").and_then(Value::as_str) {
            let k = kind.trim().to_ascii_lowercase();
            if k == "channel" || k == "group" || k == "supergroup" {
                file.tickets.chat_kind = k;
            }
        }
        if let Some(role) = tix.get("chat_role").and_then(Value::as_str) {
            if let Some(r) = normalize_chat_role(role) {
                file.tickets.chat_role = r.to_string();
            }
        }
    }
    if let Some(jail) = patch.get("jail") {
        if let Some(id) = jail.get("id").and_then(Value::as_str) {
            file.jail.id = id.trim().to_string();
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
            "role": file.godfather.role,
        },
        "workflows": { "enabled": file.workflows.enabled },
        "security": { "redact": file.security.redact },
        "tickets": {
            "mode": file.tickets.mode,
            "lease_secs": ticket_lease_secs(file),
            "squad_cap": squad_cap(file),
            "squad_cap_override": file.tickets.squad_cap,
            "member_count": file.tickets.member_count,
            "chat_kind": chat_kind(file),
            "chat_role": chat_role(file),
            "bot_slot_cap": bot_slot_cap(file),
        },
        "jail": { "id": jail_id(file) },
    })
}

/// Persist Telegram `getChatMemberCount` / `getChat.type` / bot role.
/// A `0` count is ignored so a failed probe does not wipe an owner-set value.
/// Role is skipped when the owner set `godfather.role`.
pub fn apply_live_chat_meta(
    data_dir: &Path,
    member_count: u64,
    chat_kind: Option<&str>,
    chat_role: Option<&str>,
) -> Result<bool, String> {
    let mut file = load_result(data_dir)?;
    let mut changed = false;
    let n = clamp_count(member_count);
    if n > 0 && file.tickets.member_count != n {
        file.tickets.member_count = n;
        changed = true;
    }
    if let Some(raw) = chat_kind {
        let k = match raw.trim().to_ascii_lowercase().as_str() {
            "group" => "group",
            "supergroup" => "supergroup",
            "channel" => "channel",
            _ => "",
        };
        if !k.is_empty() && file.tickets.chat_kind != k {
            file.tickets.chat_kind = k.to_string();
            changed = true;
        }
    }
    if file.godfather.role.trim().is_empty() {
        if let Some(raw) = chat_role.and_then(normalize_chat_role) {
            if file.tickets.chat_role != raw {
                file.tickets.chat_role = raw.to_string();
                changed = true;
            }
        }
    }
    if changed {
        save(data_dir, &file)?;
    }
    Ok(changed)
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
            role: String::new(),
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
        apply_patch(&mut file, &json!({ "tickets": { "lease_secs": 12 } }));
        assert_eq!(file.tickets.lease_secs, MIN_TICKET_LEASE_SECS);
        apply_patch(&mut file, &json!({ "tickets": { "lease_secs": 900 } }));
        assert_eq!(ticket_lease_secs(&file), 900);
        apply_patch(
            &mut file,
            &json!({
                "jail": { "id": " alice " },
                "tickets": {
                    "squad_cap": 0,
                    "member_count": 12,
                    "chat_kind": "channel"
                }
            }),
        );
        assert_eq!(jail_id(&file), "alice");
        assert_eq!(squad_cap(&file), 12);
        assert_eq!(bot_slot_cap(&file), TG_CHANNEL_ADMINS_MAX);
        apply_patch(
            &mut file,
            &json!({ "tickets": { "chat_kind": "group", "member_count": 0, "squad_cap": 0 } }),
        );
        assert_eq!(bot_slot_cap(&file), TG_GROUP_BOTS_MAX);
        assert_eq!(squad_cap(&file), TG_GROUP_BOTS_MAX);
    }

    #[test]
    fn empty_jail_id_wires_local() {
        let file = SettingsFile::default();
        assert_eq!(jail_id(&file), "local");
        assert_eq!(chat_kind(&file), "channel");
        assert_eq!(chat_role(&file), "local");
        assert_eq!(squad_cap(&file), TG_CHANNEL_ADMINS_MAX);
        let w = redacted_wire(&file, None);
        assert_eq!(w["jail"]["id"], "local");
        assert_eq!(w["tickets"]["bot_slot_cap"], TG_CHANNEL_ADMINS_MAX);
        assert_eq!(w["tickets"]["squad_cap_override"], 0);
        assert_eq!(w["tickets"]["squad_cap"], TG_CHANNEL_ADMINS_MAX);
        assert!(!json_has_bot_token(&w), "{w}");
    }

    #[test]
    fn apply_live_chat_meta_sets_count_and_kind() {
        let dir = temp("live-meta");
        assert!(!apply_live_chat_meta(&dir, 0, None, None).expect("zero"));
        assert_eq!(load_result(&dir).expect("load").tickets.member_count, 0);
        assert!(apply_live_chat_meta(&dir, 7, Some("supergroup"), Some("host")).expect("set"));
        let mut file = load_result(&dir).expect("reload");
        assert_eq!(file.tickets.member_count, 7);
        assert_eq!(file.tickets.chat_kind, "supergroup");
        assert_eq!(file.tickets.chat_role, "host");
        assert_eq!(squad_cap(&file), 7);
        assert!(!apply_live_chat_meta(&dir, 7, Some("supergroup"), Some("host")).expect("same"));
        assert!(!apply_live_chat_meta(&dir, 0, Some("nope"), None).expect("ignore"));
        assert_eq!(load_result(&dir).expect("keep").tickets.member_count, 7);
        apply_patch(
            &mut file,
            &json!({ "godfather": { "role": "guest" }, "tickets": { "mode": "squad" } }),
        );
        file.workflows.enabled = vec!["ticket-squad".into()];
        assert_eq!(chat_role(&file), "guest");
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
