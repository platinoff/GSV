//! Ticket board + MCP claim / solo-squad dispatch (bands 168 + 170 + 171).
//!
//! Source of truth: `{kit}/docs/gsv/tickets.jsonl` (git-tracked, no secrets).
//! Claims/events append `{kit}/docs/gsv/ticket_claims.jsonl` — sibling of
//! fingerprints, never mixed into drain JSONL. Missing files → empty list
//! `{ok:true,tickets:[]}`.
//!
//! Band 170: scenario catalog, registered-product create, solo vs squad
//! assignment among online MCP presence, and claimed/done/error events.
//! Band 171: `lease_until` on `in_progress`; stale reclaim → `open` + `kind:reclaimed`.
//! Band 174: Telegram `/ticket` ingest → board row; solo MCP auto-claims.
//! Band 176: walk posts session lines (solo claimed / squad assigned / bench).

use std::collections::HashMap;
use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::fingerprint;
use super::settings;

/// Process-local MCP presence (heartbeat). Isolated per [`crate::AppState`].
pub type PresenceStore = Mutex<HashMap<String, Presence>>;

/// Heartbeats older than this are offline.
pub const PRESENCE_TTL_SECS: u64 = 120;

/// Default `in_progress` lease (seconds). Settings `tickets.lease_secs` may override.
pub const DEFAULT_LEASE_SECS: u64 = settings::DEFAULT_TICKET_LEASE_SECS;

/// Canonical tickets JSONL (kit repo, not `data/`).
pub fn tickets_path(repo_root: &Path) -> PathBuf {
    repo_root.join("docs/gsv/tickets.jsonl")
}

/// Canonical claim/event log (sibling of fingerprints).
pub fn claims_path(repo_root: &Path) -> PathBuf {
    repo_root.join("docs/gsv/ticket_claims.jsonl")
}

/// Named ticket templates (placement on the Galaxy board).
pub fn scenarios_path(repo_root: &Path) -> PathBuf {
    repo_root.join("docs/gsv/ticket_scenarios.json")
}

/// Who claimed a ticket (fingerprint-class fields).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaimedBy {
    pub actor: String,
    pub ide: String,
    pub model: String,
    pub agent: String,
}

/// Online MCP worker (same identity fields as a claim).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Presence {
    pub actor: String,
    pub ide: String,
    pub model: String,
    pub agent: String,
    pub seen_unix: u64,
}

impl Presence {
    fn key(&self) -> String {
        format!("{}|{}|{}", self.actor, self.ide, self.agent)
    }

    fn from_who(who: &ClaimedBy, seen_unix: u64) -> Self {
        Self {
            actor: who.actor.clone(),
            ide: who.ide.clone(),
            model: who.model.clone(),
            agent: who.agent.clone(),
            seen_unix,
        }
    }

    fn as_claimed(&self) -> ClaimedBy {
        ClaimedBy {
            actor: self.actor.clone(),
            ide: self.ide.clone(),
            model: self.model.clone(),
            agent: self.agent.clone(),
        }
    }
}

/// Solo = one MCP (stable pick). Squad = random among online.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TicketMode {
    Solo,
    Squad,
}

impl TicketMode {
    /// Wire string.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Solo => "solo",
            Self::Squad => "squad",
        }
    }
}

/// One board row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ticket {
    pub id: String,
    pub ts: String,
    pub title: String,
    #[serde(default)]
    pub body: String,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claimed_by: Option<ClaimedBy>,
    #[serde(default = "default_product")]
    pub product: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub workflow: String,
    /// Named scenario that placed this row (`ticket_scenarios.json` id).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub scenario: String,
    /// Unix seconds when an `in_progress` lease expires. Missing on legacy rows
    /// is treated as already stale.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lease_until: Option<u64>,
}

fn default_product() -> String {
    "gsv".to_string()
}

fn default_kind() -> String {
    "claimed".to_string()
}

/// Append-only claim / lifecycle row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TicketClaim {
    pub ticket_id: String,
    pub ts: String,
    pub actor: String,
    pub ide: String,
    pub model: String,
    pub agent: String,
    #[serde(default = "default_kind")]
    pub kind: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub note: String,
}

/// One step inside a scenario band (`tickets[]`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScenarioStep {
    pub title: String,
    #[serde(default)]
    pub body: String,
}

/// Named scenario from `ticket_scenarios.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TicketScenario {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub workflow: String,
    #[serde(default = "default_product")]
    pub product: String,
    /// When non-empty, create this many board rows (a band) instead of one.
    #[serde(default)]
    pub tickets: Vec<ScenarioStep>,
}

/// One claim→done step from [`solo_walk`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WalkStep {
    pub ticket_id: String,
    pub title: String,
    pub phase: String,
    /// `solo` or `squad` (empty → treat as solo).
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub actor: String,
}

/// Solo-bot walk of open tickets (optional scenario filter).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WalkReport {
    pub ok: bool,
    pub scenario: String,
    pub walked: Vec<WalkStep>,
    pub telegram: usize,
    /// Band 176: `gsv_dev` median session line (empty until Telegram sync).
    #[serde(default)]
    pub bench: String,
}

/// Load / claim failures (HTTP maps these to 404/403/400).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TicketError {
    NotFound,
    Forbidden,
    BadRequest(String),
    Io(String),
}

impl fmt::Display for TicketError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => write!(f, "unknown ticket"),
            Self::Forbidden => write!(f, "ticket-claim workflow is off"),
            Self::BadRequest(m) => write!(f, "{m}"),
            Self::Io(m) => write!(f, "{m}"),
        }
    }
}

fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| default.to_string())
}

/// Fingerprint-style claim identity (`GSV_ACTOR` / `GSV_IDE` / `GSV_MODEL` / `GSV_AGENT`).
pub fn resolve_claimed_by() -> ClaimedBy {
    ClaimedBy {
        actor: env_or("GSV_ACTOR", "agent"),
        ide: env_or("GSV_IDE", "cursor"),
        model: fingerprint::resolve_model(),
        agent: env_or("GSV_AGENT", "orchestrator"),
    }
}

/// Empty presence map (tests / [`crate::AppState::new`]).
pub fn new_presence_store() -> PresenceStore {
    Mutex::new(HashMap::new())
}

fn read_tickets(path: &Path) -> Result<Vec<Ticket>, TicketError> {
    let Ok(f) = fs::File::open(path) else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for line in BufReader::new(f).lines().map_while(Result::ok) {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        match serde_json::from_str::<Ticket>(line) {
            Ok(t) => out.push(t),
            Err(_) => continue,
        }
    }
    Ok(out)
}

fn write_tickets(path: &Path, tickets: &[Ticket]) -> Result<(), TicketError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|_| TicketError::Io("tickets mkdir failed".into()))?;
    }
    let mut raw = String::new();
    for t in tickets {
        let line = serde_json::to_string(t)
            .map_err(|_| TicketError::Io("tickets encode failed".into()))?;
        raw.push_str(&line);
        raw.push('\n');
    }
    fs::write(path, raw).map_err(|_| TicketError::Io("tickets write failed".into()))
}

fn read_claims(path: &Path) -> Vec<TicketClaim> {
    let Ok(f) = fs::File::open(path) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for line in BufReader::new(f).lines().map_while(Result::ok) {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Ok(row) = serde_json::from_str::<TicketClaim>(line) {
            out.push(row);
        }
    }
    out
}

fn append_claim(path: &Path, row: &TicketClaim) -> Result<(), TicketError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|_| TicketError::Io("claims mkdir failed".into()))?;
    }
    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|_| TicketError::Io("claims open failed".into()))?;
    serde_json::to_writer(&mut f, row)
        .map_err(|_| TicketError::Io("claims encode failed".into()))?;
    f.write_all(b"\n")
        .map_err(|_| TicketError::Io("claims write failed".into()))?;
    Ok(())
}

fn new_id() -> String {
    let n = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("t-{n}")
}

/// Registered product ids live in `docs/gsv/PRODUCTS.md`. Temp kits without
/// that file only allow `gsv` (band 168 tests).
pub fn product_allowed(repo_root: &Path, product: &str) -> bool {
    let id = product.trim();
    if id.is_empty() {
        return false;
    }
    let md = repo_root.join("docs/gsv/PRODUCTS.md");
    if md.is_file() {
        let text = fs::read_to_string(&md).unwrap_or_default();
        let needle = format!("| **{id}**");
        return text
            .to_ascii_lowercase()
            .contains(&needle.to_ascii_lowercase());
    }
    id == "gsv"
}

/// Load scenario catalog. Missing file → empty.
pub fn load_scenarios(repo_root: &Path) -> Vec<TicketScenario> {
    let Ok(raw) = fs::read_to_string(scenarios_path(repo_root)) else {
        return Vec::new();
    };
    let Ok(v) = serde_json::from_str::<Value>(&raw) else {
        return Vec::new();
    };
    serde_json::from_value(v.get("scenarios").cloned().unwrap_or(Value::Null)).unwrap_or_default()
}

/// Effective mode: squad only when settings say so **and** `ticket-squad` is on.
pub fn resolve_mode(file: &settings::SettingsFile) -> TicketMode {
    if settings::ticket_mode(file) == "squad" {
        TicketMode::Squad
    } else {
        TicketMode::Solo
    }
}

/// Prune stale heartbeats and return the online set (stable sort).
pub fn online_now(store: &PresenceStore) -> Vec<Presence> {
    let now = unix_now();
    let Ok(mut map) = store.lock() else {
        return Vec::new();
    };
    map.retain(|_, p| now.saturating_sub(p.seen_unix) <= PRESENCE_TTL_SECS);
    let mut out: Vec<Presence> = map.values().cloned().collect();
    out.sort_by(|a, b| {
        a.actor
            .cmp(&b.actor)
            .then(a.ide.cmp(&b.ide))
            .then(a.agent.cmp(&b.agent))
    });
    out
}

/// Heartbeat: record `who` as online and return the current set.
pub fn heartbeat(store: &PresenceStore, who: &ClaimedBy) -> Vec<Presence> {
    let now = unix_now();
    let row = Presence::from_who(who, now);
    if let Ok(mut map) = store.lock() {
        map.insert(row.key(), row);
    }
    online_now(store)
}

/// Pick who gets the ticket. Solo = lexicographic first. Squad = `seed % n`.
pub fn pick_assignee(mode: TicketMode, online: &[Presence], seed: u64) -> Option<&Presence> {
    if online.is_empty() {
        return None;
    }
    match mode {
        TicketMode::Solo => online.first(),
        TicketMode::Squad => online.get((seed as usize) % online.len()),
    }
}

fn same_worker(a: &ClaimedBy, b: &ClaimedBy) -> bool {
    a.actor == b.actor && a.ide == b.ide && a.agent == b.agent
}

fn lease_is_expired(t: &Ticket, now: u64) -> bool {
    if t.status != "in_progress" {
        return false;
    }
    match t.lease_until {
        Some(until) => until <= now,
        None => true,
    }
}

fn clear_claim(ticket: &mut Ticket) {
    ticket.status = "open".into();
    ticket.claimed_by = None;
    ticket.lease_until = None;
}

/// `in_progress` rows whose lease has passed (or never had one) → `open` + `reclaimed`.
pub fn reclaim_stale(repo_root: &Path, data_dir: &Path) -> Result<Vec<Ticket>, TicketError> {
    let file = settings::load_result(data_dir).map_err(TicketError::Io)?;
    if !settings::ticket_claim_enabled(&file) {
        return Ok(Vec::new());
    }
    let path = tickets_path(repo_root);
    let mut tickets = read_tickets(&path)?;
    let now = unix_now();
    let mut out = Vec::new();
    for ticket in &mut tickets {
        if !lease_is_expired(ticket, now) {
            continue;
        }
        let who = ticket.claimed_by.clone().unwrap_or_else(resolve_claimed_by);
        clear_claim(ticket);
        append_event(repo_root, &ticket.id, &who, "reclaimed", "lease expired")?;
        out.push(ticket.clone());
    }
    if !out.is_empty() {
        write_tickets(&path, &tickets)?;
    }
    Ok(out)
}

/// Extend leases for `in_progress` tickets held by `who` (heartbeat grace).
pub fn renew_leases(
    repo_root: &Path,
    data_dir: &Path,
    who: &ClaimedBy,
) -> Result<usize, TicketError> {
    let file = settings::load_result(data_dir).map_err(TicketError::Io)?;
    let secs = settings::ticket_lease_secs(&file);
    let path = tickets_path(repo_root);
    let mut tickets = read_tickets(&path)?;
    let until = unix_now().saturating_add(secs);
    let mut n = 0usize;
    for ticket in &mut tickets {
        if ticket.status != "in_progress" {
            continue;
        }
        let Some(by) = &ticket.claimed_by else {
            continue;
        };
        if !same_worker(by, who) {
            continue;
        }
        ticket.lease_until = Some(until);
        n += 1;
    }
    if n > 0 {
        write_tickets(&path, &tickets)?;
    }
    Ok(n)
}

fn assign_seed() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(1)
}

/// `GET /api/tickets` tickets array only.
pub fn list(repo_root: &Path) -> Value {
    match read_tickets(&tickets_path(repo_root)) {
        Ok(tickets) => json!({ "ok": true, "tickets": tickets }),
        Err(e) => json!({ "ok": false, "error": e.to_string() }),
    }
}

/// Board wire: tickets + mode + online + scenarios + recent events.
pub fn wire_list(repo_root: &Path, data_dir: &Path, presence: &PresenceStore) -> Value {
    let _ = reclaim_stale(repo_root, data_dir);
    let mut v = list(repo_root);
    if v.get("ok").and_then(Value::as_bool) != Some(true) {
        return v;
    }
    let file = settings::load_result(data_dir).unwrap_or_default();
    let mode = resolve_mode(&file);
    let online = online_now(presence);
    let mut events = read_claims(&claims_path(repo_root));
    if events.len() > 8 {
        events = events.split_off(events.len() - 8);
    }
    v["mode"] = json!(mode.as_str());
    v["lease_secs"] = json!(settings::ticket_lease_secs(&file));
    v["online"] = json!(online);
    v["scenarios"] = json!(load_scenarios(repo_root));
    v["events"] = json!(events);
    v
}

/// Create an `open` ticket (no workflow gate). Product must be registered.
pub fn create(
    repo_root: &Path,
    title: &str,
    body: &str,
    product: &str,
) -> Result<Ticket, TicketError> {
    create_with_workflow(repo_root, title, body, product, "", "")
}

/// Board row from a Godfather Telegram message (workflow `telegram`).
pub fn create_from_telegram(
    repo_root: &Path,
    title: &str,
    body: &str,
    product: &str,
) -> Result<Ticket, TicketError> {
    create_with_workflow(repo_root, title, body, product, "telegram", "")
}

/// Fingerprint-class event that the row came from Telegram (`kind: telegram`).
pub fn append_telegram_event(
    repo_root: &Path,
    ticket_id: &str,
    from: &str,
) -> Result<(), TicketError> {
    append_event(
        repo_root,
        ticket_id,
        &ClaimedBy {
            actor: "telegram".into(),
            ide: "telegram".into(),
            model: "godfather".into(),
            agent: from.trim().to_string(),
        },
        "telegram",
        from,
    )
}

fn create_with_workflow(
    repo_root: &Path,
    title: &str,
    body: &str,
    product: &str,
    workflow: &str,
    scenario: &str,
) -> Result<Ticket, TicketError> {
    let title = title.trim();
    if title.is_empty() {
        return Err(TicketError::BadRequest("title required".into()));
    }
    let product = {
        let p = product.trim();
        if p.is_empty() {
            "gsv"
        } else {
            p
        }
    };
    if !product_allowed(repo_root, product) {
        return Err(TicketError::BadRequest("unregistered product".into()));
    }
    let path = tickets_path(repo_root);
    let mut tickets = read_tickets(&path)?;
    let ticket = Ticket {
        id: new_id(),
        ts: now_rfc3339(),
        title: title.to_string(),
        body: body.trim().to_string(),
        status: "open".into(),
        claimed_by: None,
        product: product.to_string(),
        workflow: workflow.trim().to_string(),
        scenario: scenario.trim().to_string(),
        lease_until: None,
    };
    tickets.push(ticket.clone());
    write_tickets(&path, &tickets)?;
    Ok(ticket)
}

fn scenario_steps(sc: &TicketScenario) -> Vec<(String, String)> {
    if sc.tickets.is_empty() {
        vec![(sc.title.clone(), sc.body.clone())]
    } else {
        sc.tickets
            .iter()
            .map(|s| (s.title.clone(), s.body.clone()))
            .collect()
    }
}

fn load_scenario_gated(
    repo_root: &Path,
    data_dir: &Path,
    scenario_id: &str,
) -> Result<TicketScenario, TicketError> {
    let id = scenario_id.trim();
    if id.is_empty() {
        return Err(TicketError::BadRequest("scenario_id required".into()));
    }
    let sc = load_scenarios(repo_root)
        .into_iter()
        .find(|s| s.id == id)
        .ok_or(TicketError::NotFound)?;
    let file = settings::load_result(data_dir).map_err(TicketError::Io)?;
    if !sc.workflow.is_empty() && !file.workflows.enabled.iter().any(|w| w == &sc.workflow) {
        return Err(TicketError::BadRequest(format!(
            "{} workflow is off",
            sc.workflow
        )));
    }
    Ok(sc)
}

/// Create every row in a named scenario (one ticket, or a `tickets[]` band).
pub fn create_band_from_scenario(
    repo_root: &Path,
    data_dir: &Path,
    scenario_id: &str,
    product_override: &str,
) -> Result<Vec<Ticket>, TicketError> {
    let sc = load_scenario_gated(repo_root, data_dir, scenario_id)?;
    let product = if product_override.trim().is_empty() {
        sc.product.as_str()
    } else {
        product_override.trim()
    };
    let mut out = Vec::new();
    for (title, body) in scenario_steps(&sc) {
        out.push(create_with_workflow(
            repo_root,
            &title,
            &body,
            product,
            &sc.workflow,
            &sc.id,
        )?);
    }
    if out.is_empty() {
        return Err(TicketError::BadRequest("scenario has no tickets".into()));
    }
    Ok(out)
}

/// Create from a named scenario (workflow must be enabled). Band → first row.
pub fn create_from_scenario(
    repo_root: &Path,
    data_dir: &Path,
    scenario_id: &str,
    product_override: &str,
) -> Result<Ticket, TicketError> {
    let mut tickets =
        create_band_from_scenario(repo_root, data_dir, scenario_id, product_override)?;
    Ok(tickets.remove(0))
}

fn append_event(
    repo_root: &Path,
    ticket_id: &str,
    who: &ClaimedBy,
    kind: &str,
    note: &str,
) -> Result<(), TicketError> {
    append_claim(
        &claims_path(repo_root),
        &TicketClaim {
            ticket_id: ticket_id.to_string(),
            ts: now_rfc3339(),
            actor: who.actor.clone(),
            ide: who.ide.clone(),
            model: who.model.clone(),
            agent: who.agent.clone(),
            kind: kind.to_string(),
            note: note.trim().to_string(),
        },
    )
}

struct Transition<'a> {
    from: &'a [&'a str],
    to: &'a str,
    kind: &'a str,
    note: &'a str,
}

fn set_status(
    repo_root: &Path,
    data_dir: &Path,
    id: &str,
    who: ClaimedBy,
    step: Transition<'_>,
    presence: Option<&PresenceStore>,
) -> Result<Ticket, TicketError> {
    let id = id.trim();
    if id.is_empty() {
        return Err(TicketError::BadRequest("id required".into()));
    }
    let file = settings::load_result(data_dir).map_err(TicketError::Io)?;
    if !settings::ticket_claim_enabled(&file) {
        return Err(TicketError::Forbidden);
    }
    let path = tickets_path(repo_root);
    let mut tickets = read_tickets(&path)?;
    let pos = tickets
        .iter()
        .position(|t| t.id == id)
        .ok_or(TicketError::NotFound)?;
    if !step.from.contains(&tickets[pos].status.as_str()) {
        return Err(TicketError::BadRequest(format!(
            "ticket is {}",
            tickets[pos].status
        )));
    }
    tickets[pos].status = step.to.into();
    tickets[pos].claimed_by = Some(who.clone());
    if step.to == "in_progress" {
        let secs = settings::ticket_lease_secs(&file);
        tickets[pos].lease_until = Some(unix_now().saturating_add(secs));
    } else {
        tickets[pos].lease_until = None;
    }
    let updated = tickets[pos].clone();
    write_tickets(&path, &tickets)?;
    append_event(repo_root, &updated.id, &who, step.kind, step.note)?;
    if let Some(store) = presence {
        let _ = heartbeat(store, &who);
    }
    Ok(updated)
}

/// Claim an open ticket: `open` → `in_progress`, append claim JSONL.
pub fn claim(
    repo_root: &Path,
    data_dir: &Path,
    id: &str,
    who: ClaimedBy,
) -> Result<Ticket, TicketError> {
    claim_with(repo_root, data_dir, id, who, None)
}

/// Claim and optionally heartbeat the claimant as online.
pub fn claim_with(
    repo_root: &Path,
    data_dir: &Path,
    id: &str,
    who: ClaimedBy,
    presence: Option<&PresenceStore>,
) -> Result<Ticket, TicketError> {
    let _ = reclaim_stale(repo_root, data_dir);
    set_status(
        repo_root,
        data_dir,
        id,
        who,
        Transition {
            from: &["open"],
            to: "in_progress",
            kind: "claimed",
            note: "",
        },
        presence,
    )
}

/// `in_progress` → `done`.
pub fn done(
    repo_root: &Path,
    data_dir: &Path,
    id: &str,
    who: ClaimedBy,
    note: &str,
    presence: Option<&PresenceStore>,
) -> Result<Ticket, TicketError> {
    set_status(
        repo_root,
        data_dir,
        id,
        who,
        Transition {
            from: &["in_progress"],
            to: "done",
            kind: "done",
            note,
        },
        presence,
    )
}

/// `in_progress` → `blocked` (error).
pub fn error_ticket(
    repo_root: &Path,
    data_dir: &Path,
    id: &str,
    who: ClaimedBy,
    note: &str,
    presence: Option<&PresenceStore>,
) -> Result<Ticket, TicketError> {
    set_status(
        repo_root,
        data_dir,
        id,
        who,
        Transition {
            from: &["in_progress"],
            to: "blocked",
            kind: "error",
            note,
        },
        presence,
    )
}

/// If someone is online, claim as solo/squad pick. Otherwise leave `open`.
pub fn try_dispatch(
    repo_root: &Path,
    data_dir: &Path,
    id: &str,
    presence: &PresenceStore,
    seed: u64,
) -> Result<Option<Ticket>, TicketError> {
    let file = settings::load_result(data_dir).map_err(TicketError::Io)?;
    if !settings::ticket_claim_enabled(&file) {
        return Ok(None);
    }
    let _ = reclaim_stale(repo_root, data_dir);
    let online = online_now(presence);
    let mode = resolve_mode(&file);
    let Some(who) = pick_assignee(mode, &online, seed).map(Presence::as_claimed) else {
        return Ok(None);
    };
    let kind = if mode == TicketMode::Squad {
        "assigned"
    } else {
        "claimed"
    };
    let t = set_status(
        repo_root,
        data_dir,
        id,
        who,
        Transition {
            from: &["open"],
            to: "in_progress",
            kind,
            note: mode.as_str(),
        },
        Some(presence),
    )?;
    Ok(Some(t))
}

fn maybe_dispatch(
    repo_root: &Path,
    data_dir: &Path,
    ticket: Ticket,
    presence: Option<&PresenceStore>,
    seed: u64,
) -> Result<Ticket, TicketError> {
    let Some(store) = presence else {
        return Ok(ticket);
    };
    match try_dispatch(repo_root, data_dir, &ticket.id, store, seed)? {
        Some(claimed) => Ok(claimed),
        None => Ok(ticket),
    }
}

/// HTTP POST create wire (`scenario_id` or title/body).
pub fn wire_create(
    repo_root: &Path,
    data_dir: &Path,
    body: &Value,
    presence: Option<&PresenceStore>,
) -> Result<Value, TicketError> {
    let scenario_id = body
        .get("scenario_id")
        .and_then(Value::as_str)
        .unwrap_or("");
    let product = body.get("product").and_then(Value::as_str).unwrap_or("");
    let tickets = if !scenario_id.trim().is_empty() {
        create_band_from_scenario(repo_root, data_dir, scenario_id, product)?
    } else {
        let title = body.get("title").and_then(Value::as_str).unwrap_or("");
        let tbody = body.get("body").and_then(Value::as_str).unwrap_or("");
        let product = if product.is_empty() { "gsv" } else { product };
        vec![create(repo_root, title, tbody, product)?]
    };
    let mut tickets = tickets;
    if tickets.len() == 1 {
        tickets[0] = maybe_dispatch(
            repo_root,
            data_dir,
            tickets[0].clone(),
            presence,
            assign_seed(),
        )?;
    }
    let ticket = tickets[0].clone();
    Ok(json!({ "ok": true, "ticket": ticket, "tickets": tickets }))
}

/// HTTP POST claim wire.
pub fn wire_claim(
    repo_root: &Path,
    data_dir: &Path,
    body: &Value,
    presence: Option<&PresenceStore>,
) -> Result<Value, TicketError> {
    let id = body.get("id").and_then(Value::as_str).unwrap_or("");
    let ticket = claim_with(repo_root, data_dir, id, resolve_claimed_by(), presence)?;
    Ok(json!({ "ok": true, "ticket": ticket }))
}

/// HTTP POST done wire.
pub fn wire_done(
    repo_root: &Path,
    data_dir: &Path,
    body: &Value,
    presence: Option<&PresenceStore>,
) -> Result<Value, TicketError> {
    let id = body.get("id").and_then(Value::as_str).unwrap_or("");
    let note = body.get("note").and_then(Value::as_str).unwrap_or("");
    let ticket = done(
        repo_root,
        data_dir,
        id,
        resolve_claimed_by(),
        note,
        presence,
    )?;
    Ok(json!({ "ok": true, "ticket": ticket }))
}

/// HTTP POST error wire.
pub fn wire_error(
    repo_root: &Path,
    data_dir: &Path,
    body: &Value,
    presence: Option<&PresenceStore>,
) -> Result<Value, TicketError> {
    let id = body.get("id").and_then(Value::as_str).unwrap_or("");
    let note = body.get("note").and_then(Value::as_str).unwrap_or("");
    let ticket = error_ticket(
        repo_root,
        data_dir,
        id,
        resolve_claimed_by(),
        note,
        presence,
    )?;
    Ok(json!({ "ok": true, "ticket": ticket }))
}

/// HTTP POST presence wire (heartbeat + renew leases for this worker).
pub fn wire_presence(
    repo_root: &Path,
    data_dir: &Path,
    presence: &PresenceStore,
    body: &Value,
) -> Value {
    let mut who = resolve_claimed_by();
    if let Some(a) = body.get("actor").and_then(Value::as_str) {
        if !a.trim().is_empty() {
            who.actor = a.trim().to_string();
        }
    }
    if let Some(a) = body.get("ide").and_then(Value::as_str) {
        if !a.trim().is_empty() {
            who.ide = a.trim().to_string();
        }
    }
    if let Some(a) = body.get("model").and_then(Value::as_str) {
        if !a.trim().is_empty() {
            who.model = a.trim().to_string();
        }
    }
    if let Some(a) = body.get("agent").and_then(Value::as_str) {
        if !a.trim().is_empty() {
            who.agent = a.trim().to_string();
        }
    }
    let online = heartbeat(presence, &who);
    let _ = renew_leases(repo_root, data_dir, &who);
    json!({ "ok": true, "online": online })
}

/// HTTP POST reclaim wire. Empty id → all stale. Else that `in_progress` row.
pub fn wire_reclaim(repo_root: &Path, data_dir: &Path, body: &Value) -> Result<Value, TicketError> {
    let file = settings::load_result(data_dir).map_err(TicketError::Io)?;
    if !settings::ticket_claim_enabled(&file) {
        return Err(TicketError::Forbidden);
    }
    let id = body.get("id").and_then(Value::as_str).unwrap_or("").trim();
    if id.is_empty() {
        let tickets = reclaim_stale(repo_root, data_dir)?;
        return Ok(json!({ "ok": true, "tickets": tickets }));
    }
    let path = tickets_path(repo_root);
    let mut tickets = read_tickets(&path)?;
    let pos = tickets
        .iter()
        .position(|t| t.id == id)
        .ok_or(TicketError::NotFound)?;
    if tickets[pos].status != "in_progress" {
        return Err(TicketError::BadRequest(format!(
            "ticket is {}",
            tickets[pos].status
        )));
    }
    let who = tickets[pos]
        .claimed_by
        .clone()
        .unwrap_or_else(resolve_claimed_by);
    clear_claim(&mut tickets[pos]);
    let updated = tickets[pos].clone();
    write_tickets(&path, &tickets)?;
    append_event(repo_root, &updated.id, &who, "reclaimed", "lease")?;
    Ok(json!({ "ok": true, "ticket": updated }))
}

fn ticket_sort_key(t: &Ticket) -> (String, String) {
    (t.ts.clone(), t.id.clone())
}

/// Claim then done every `open` row (optional `scenario` filter). Telegram notify is separate.
///
/// Squad mode (`tickets.mode=squad` + `ticket-squad`) assigns via [`try_dispatch`]
/// (`seed % n` among online MCP). One online worker is a valid live demo.
pub fn solo_walk(
    repo_root: &Path,
    data_dir: &Path,
    presence: Option<&PresenceStore>,
    who: ClaimedBy,
    scenario: &str,
) -> Result<WalkReport, TicketError> {
    let file = settings::load_result(data_dir).map_err(TicketError::Io)?;
    if !settings::ticket_claim_enabled(&file) {
        return Err(TicketError::Forbidden);
    }
    if let Some(store) = presence {
        let _ = heartbeat(store, &who);
        let _ = renew_leases(repo_root, data_dir, &who);
    }
    let _ = reclaim_stale(repo_root, data_dir);
    let mode = resolve_mode(&file);
    let filter = scenario.trim();
    let mut open: Vec<Ticket> = read_tickets(&tickets_path(repo_root))?
        .into_iter()
        .filter(|t| t.status == "open")
        .filter(|t| filter.is_empty() || t.scenario == filter)
        .collect();
    open.sort_by_key(ticket_sort_key);
    let mut walked = Vec::new();
    for (i, t) in open.into_iter().enumerate() {
        let (claimed, phase) = match mode {
            TicketMode::Squad => match presence {
                Some(store) => match try_dispatch(repo_root, data_dir, &t.id, store, i as u64)? {
                    Some(c) => (c, "assigned"),
                    None => (
                        claim_with(repo_root, data_dir, &t.id, who.clone(), presence)?,
                        "claimed",
                    ),
                },
                None => (
                    claim_with(repo_root, data_dir, &t.id, who.clone(), presence)?,
                    "claimed",
                ),
            },
            TicketMode::Solo => (
                claim_with(repo_root, data_dir, &t.id, who.clone(), presence)?,
                "claimed",
            ),
        };
        let kind = if phase == "assigned" { "squad" } else { "solo" };
        let actor = claimed
            .claimed_by
            .as_ref()
            .map(|c| c.actor.clone())
            .unwrap_or_default();
        walked.push(WalkStep {
            ticket_id: claimed.id.clone(),
            title: claimed.title.clone(),
            phase: phase.into(),
            kind: kind.into(),
            actor: actor.clone(),
        });
        let finisher = claimed.claimed_by.clone().unwrap_or_else(|| who.clone());
        let note = if kind == "squad" {
            "squad walk"
        } else {
            "solo walk"
        };
        let finished = done(
            repo_root,
            data_dir,
            &claimed.id,
            finisher.clone(),
            note,
            presence,
        )?;
        walked.push(WalkStep {
            ticket_id: finished.id,
            title: finished.title,
            phase: "done".into(),
            kind: kind.into(),
            actor: finisher.actor,
        });
    }
    Ok(WalkReport {
        ok: true,
        scenario: filter.to_string(),
        walked,
        telegram: 0,
        bench: String::new(),
    })
}

/// HTTP POST walk wire. Optional `scenario_id` creates the band first (all stay open).
pub fn wire_walk(
    repo_root: &Path,
    data_dir: &Path,
    body: &Value,
    presence: Option<&PresenceStore>,
) -> Result<WalkReport, TicketError> {
    let scenario_id = body
        .get("scenario_id")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();
    let create = body
        .get("create")
        .and_then(Value::as_bool)
        .unwrap_or(!scenario_id.is_empty());
    if create && !scenario_id.is_empty() {
        let _ = create_band_from_scenario(repo_root, data_dir, scenario_id, "")?;
    }
    let mut who = resolve_claimed_by();
    if let Some(a) = body.get("actor").and_then(Value::as_str) {
        if !a.trim().is_empty() {
            who.actor = a.trim().to_string();
        }
    }
    solo_walk(repo_root, data_dir, presence, who, scenario_id)
}

#[cfg(test)]
mod unit {
    use super::*;

    fn p(actor: &str) -> Presence {
        Presence {
            actor: actor.into(),
            ide: "cursor".into(),
            model: "grok-4.6".into(),
            agent: "worker".into(),
            seen_unix: 1,
        }
    }

    #[test]
    fn pick_solo_is_first_stable() {
        let online = vec![p("b"), p("a"), p("c")];
        // callers pass a sorted slice from online_now; here we assert first-of-slice
        let got = pick_assignee(TicketMode::Solo, &online, 99).expect("who");
        assert_eq!(got.actor, "b");
    }

    #[test]
    fn pick_squad_is_seed_mod_len() {
        let online = vec![p("a"), p("b"), p("c")];
        assert_eq!(
            pick_assignee(TicketMode::Squad, &online, 0)
                .expect("0")
                .actor,
            "a"
        );
        assert_eq!(
            pick_assignee(TicketMode::Squad, &online, 1)
                .expect("1")
                .actor,
            "b"
        );
        assert_eq!(
            pick_assignee(TicketMode::Squad, &online, 5)
                .expect("5")
                .actor,
            "c"
        );
        assert!(pick_assignee(TicketMode::Squad, &[], 0).is_none());
    }

    #[test]
    fn expired_when_missing_or_past() {
        let mut t = Ticket {
            id: "t".into(),
            ts: "now".into(),
            title: "x".into(),
            body: String::new(),
            status: "in_progress".into(),
            claimed_by: None,
            product: "gsv".into(),
            workflow: String::new(),
            scenario: String::new(),
            lease_until: None,
        };
        assert!(lease_is_expired(&t, 100));
        t.lease_until = Some(50);
        assert!(lease_is_expired(&t, 100));
        t.lease_until = Some(200);
        assert!(!lease_is_expired(&t, 100));
        t.status = "open".into();
        assert!(!lease_is_expired(&t, 100));
    }
}
