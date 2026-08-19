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
//! Band 177: `run mcp bot hook up scenario` parses catalog / roadmap band /
//! superpowers plan into tickets (cap 10). Optional walk stays Telegram-sync.
//! Band 178: scenario benchmark — time `abrakadabra-session` create+walk and
//! persist `docs/gsv/scenario_bench.json` (Godfather line + Galaxy + MCP).

use std::collections::HashMap;
use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

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

/// Max tickets a roadmap/plan hook may place (one VDT band).
pub const MAX_HOOK_TICKETS: usize = 10;

/// Catalog id timed by the scenario bench (`gsv_dev` / `GET /api/tickets/bench`).
pub const SCENARIO_BENCH_ID: &str = "abrakadabra-session";

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

/// GSV PH-S* band tables (source for `hook band N`).
pub fn roadmap_path(repo_root: &Path) -> PathBuf {
    repo_root.join("docs/gsv/GSV_TECH_ROADMAP.md")
}

/// Superpowers plan markdown (source for `hook plan <stem>`).
pub fn plans_dir(repo_root: &Path) -> PathBuf {
    repo_root.join("docs/superpowers/plans")
}

/// Last `abrakadabra-session` Instant timings (git-tracked, no secrets).
pub fn scenario_bench_path(repo_root: &Path) -> PathBuf {
    repo_root.join("docs/gsv/scenario_bench.json")
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

/// Parsed `run mcp bot hook up scenario …` (catalog id, `band N`, or `plan stem`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HookPhrase {
    pub source: String,
    pub id: String,
    pub walk: bool,
}

/// One `## Спринти (band N)` table from `GSV_TECH_ROADMAP.md`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoadmapBand {
    pub band: u32,
    pub title: String,
    pub open: Vec<ScenarioStep>,
    pub all: Vec<ScenarioStep>,
}

/// Instant timings for one `abrakadabra-session` create+walk (band 178).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScenarioBench {
    pub ok: bool,
    #[serde(default)]
    pub scenario: String,
    #[serde(default)]
    pub create_ns: u64,
    #[serde(default)]
    pub walk_ns: u64,
    #[serde(default)]
    pub session_walk_ns: u64,
    #[serde(default)]
    pub mds_ns: u64,
    #[serde(default)]
    pub enqueue_ns: u64,
    #[serde(default)]
    pub walked: usize,
    #[serde(default)]
    pub recorded_at: String,
}

impl Default for ScenarioBench {
    fn default() -> Self {
        Self {
            ok: false,
            scenario: SCENARIO_BENCH_ID.into(),
            create_ns: 0,
            walk_ns: 0,
            session_walk_ns: 0,
            mds_ns: 0,
            enqueue_ns: 0,
            walked: 0,
            recorded_at: String::new(),
        }
    }
}

/// Result of placing tickets from a catalog / roadmap / plan hook.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HookReport {
    pub ok: bool,
    pub source: String,
    pub id: String,
    pub scenario: String,
    pub tickets: Vec<Ticket>,
    #[serde(default)]
    pub skipped: usize,
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
    v["bench"] = wire_bench(repo_root);
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

fn title_chars(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}

fn parse_band_heading(line: &str) -> Option<(u32, String)> {
    let t = line.trim();
    if !t.starts_with('#') {
        return None;
    }
    let lower = t.to_ascii_lowercase();
    let i = lower.find("band ")?;
    let rest = t.get(i + 5..)?.trim();
    let num: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    let band = num.parse::<u32>().ok()?;
    if band < 102 {
        return None;
    }
    Some((band, t.trim_start_matches('#').trim().to_string()))
}

fn parse_phs_row(line: &str) -> Option<(String, String, String, bool)> {
    if !line.contains("PH-S") {
        return None;
    }
    let cells: Vec<&str> = line
        .split('|')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    if cells.len() < 2 {
        return None;
    }
    let id = cells[0].replace('*', "");
    let id = id.trim();
    if !id.starts_with("PH-S") {
        return None;
    }
    let title = cells[1].replace('*', "");
    let title = title.trim();
    let body = cells.get(2).copied().unwrap_or("").trim();
    let closed = line.contains("✅");
    let open = !closed;
    Some((id.to_string(), title.to_string(), body.to_string(), open))
}

/// Parse `GSV_TECH_ROADMAP.md` band tables (`## … band N` + `PH-S*` rows).
pub fn parse_roadmap_bands(md: &str) -> Vec<RoadmapBand> {
    let mut bands = Vec::new();
    let mut cur: Option<RoadmapBand> = None;
    for line in md.lines() {
        if let Some((n, title)) = parse_band_heading(line) {
            if let Some(b) = cur.take() {
                bands.push(b);
            }
            cur = Some(RoadmapBand {
                band: n,
                title,
                open: Vec::new(),
                all: Vec::new(),
            });
            continue;
        }
        if line.starts_with("## ") {
            if let Some(b) = cur.take() {
                bands.push(b);
            }
            continue;
        }
        let Some((id, title, body, open)) = parse_phs_row(line) else {
            continue;
        };
        let Some(b) = cur.as_mut() else {
            continue;
        };
        let step = ScenarioStep {
            title: format!("{id} {title}"),
            body,
        };
        if open {
            b.open.push(step.clone());
        }
        b.all.push(step);
    }
    if let Some(b) = cur {
        bands.push(b);
    }
    bands
}

/// Open markdown checkboxes (`- [ ]` / `* [ ]`). Cap [`MAX_HOOK_TICKETS`].
pub fn parse_plan_open_items(md: &str) -> Vec<ScenarioStep> {
    let mut out = Vec::new();
    for line in md.lines() {
        let t = line.trim();
        let rest = t
            .strip_prefix("- [ ]")
            .or_else(|| t.strip_prefix("* [ ]"))
            .or_else(|| t.strip_prefix("+ [ ]"));
        let Some(rest) = rest else {
            continue;
        };
        let title = rest.trim();
        if title.is_empty() {
            continue;
        }
        out.push(ScenarioStep {
            title: title_chars(title, 80),
            body: title.to_string(),
        });
        if out.len() >= MAX_HOOK_TICKETS {
            break;
        }
    }
    out
}

fn strip_walk_flag(rest: &str) -> (String, bool) {
    let t = rest.trim();
    let lower = t.to_ascii_lowercase();
    for suffix in [" and walk", " walk"] {
        if let Some(stripped) = lower.strip_suffix(suffix) {
            return (t[..stripped.len()].trim().to_string(), true);
        }
    }
    (t.to_string(), false)
}

fn parse_hook_target(rest: &str) -> Option<HookPhrase> {
    let (target, walk) = strip_walk_flag(rest);
    if target.is_empty() {
        return None;
    }
    let lower = target.to_ascii_lowercase();
    if let Some(num) = lower.strip_prefix("band ") {
        let id = num.trim();
        if id.is_empty() {
            return None;
        }
        return Some(HookPhrase {
            source: "band".into(),
            id: id.to_string(),
            walk,
        });
    }
    if let Some(stem) = lower.strip_prefix("plan ") {
        let id = target.get(5..).unwrap_or(stem).trim();
        if id.is_empty() {
            return None;
        }
        return Some(HookPhrase {
            source: "plan".into(),
            id: id.to_string(),
            walk,
        });
    }
    let id = target.split_whitespace().next()?.to_string();
    Some(HookPhrase {
        source: "scenario".into(),
        id,
        walk,
    })
}

/// Parse `run mcp bot hook up scenario <id|band N|plan stem> [walk]`.
pub fn parse_hook_phrase(text: &str) -> Option<HookPhrase> {
    let t = text.trim();
    if t.is_empty() {
        return None;
    }
    if let Ok(v) = serde_json::from_str::<Value>(t) {
        if v.get("kind").and_then(Value::as_str) != Some("hook") {
            return None;
        }
        let source = v
            .get("source")
            .and_then(Value::as_str)
            .unwrap_or("scenario")
            .trim();
        let id = v.get("id").and_then(Value::as_str).unwrap_or("").trim();
        if id.is_empty() {
            return None;
        }
        let walk = v.get("walk").and_then(Value::as_bool).unwrap_or(false);
        return Some(HookPhrase {
            source: source.to_ascii_lowercase(),
            id: id.to_string(),
            walk,
        });
    }
    let lower = t.to_ascii_lowercase();
    let rest = if let Some(i) = lower.find("hook up scenario") {
        t.get(i + "hook up scenario".len()..)?.trim()
    } else if let Some(i) = lower.find("hookup scenario") {
        t.get(i + "hookup scenario".len()..)?.trim()
    } else if lower.starts_with("/hook ") || lower.starts_with("hook ") {
        t.split_once(' ').map(|(_, r)| r.trim()).unwrap_or("")
    } else {
        return None;
    };
    parse_hook_target(rest)
}

fn sanitize_plan_stem(raw: &str) -> Result<String, TicketError> {
    let s = raw.trim().trim_end_matches(".md");
    if s.is_empty() || s.contains("..") || s.contains('/') || s.contains('\\') {
        return Err(TicketError::BadRequest("invalid plan id".into()));
    }
    if !s
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
    {
        return Err(TicketError::BadRequest("invalid plan id".into()));
    }
    Ok(s.to_string())
}

fn steps_for_band(md: &str, band: u32) -> Result<(String, Vec<ScenarioStep>), TicketError> {
    let found = parse_roadmap_bands(md)
        .into_iter()
        .find(|b| b.band == band)
        .ok_or(TicketError::NotFound)?;
    let scenario = format!("roadmap-band-{band}");
    let steps = if found.open.is_empty() {
        found.all
    } else {
        found.open
    };
    if steps.is_empty() {
        return Err(TicketError::BadRequest("band has no PH-S* rows".into()));
    }
    Ok((scenario, steps.into_iter().take(MAX_HOOK_TICKETS).collect()))
}

fn create_from_steps(
    repo_root: &Path,
    scenario: &str,
    workflow: &str,
    product: &str,
    steps: Vec<ScenarioStep>,
) -> Result<(Vec<Ticket>, usize), TicketError> {
    if steps.is_empty() {
        return Err(TicketError::BadRequest("hook has no tickets".into()));
    }
    let existing = read_tickets(&tickets_path(repo_root))?;
    let mut out = Vec::new();
    let mut skipped = 0usize;
    for step in steps.into_iter().take(MAX_HOOK_TICKETS) {
        let dup = existing.iter().any(|t| {
            t.scenario == scenario
                && t.title == step.title
                && (t.status == "open" || t.status == "in_progress")
        }) || out.iter().any(|t: &Ticket| t.title == step.title);
        if dup {
            skipped += 1;
            continue;
        }
        out.push(create_with_workflow(
            repo_root,
            &step.title,
            &step.body,
            product,
            workflow,
            scenario,
        )?);
    }
    if out.is_empty() {
        let reuse: Vec<Ticket> = existing
            .into_iter()
            .filter(|t| t.scenario == scenario && t.status == "open")
            .collect();
        if reuse.is_empty() {
            return Err(TicketError::BadRequest("hook has no new tickets".into()));
        }
        return Ok((reuse, skipped));
    }
    Ok((out, skipped))
}

/// Place tickets from a catalog scenario, roadmap band, or superpowers plan.
pub fn hook_up(
    repo_root: &Path,
    data_dir: &Path,
    source: &str,
    id: &str,
) -> Result<HookReport, TicketError> {
    let file = settings::load_result(data_dir).map_err(TicketError::Io)?;
    if !settings::ticket_claim_enabled(&file) {
        return Err(TicketError::Forbidden);
    }
    let source = source.trim().to_ascii_lowercase();
    let id = id.trim();
    if id.is_empty() {
        return Err(TicketError::BadRequest("hook id required".into()));
    }
    let (scenario, tickets, skipped) = match source.as_str() {
        "scenario" | "catalog" => {
            let sc = load_scenario_gated(repo_root, data_dir, id)?;
            let steps: Vec<ScenarioStep> = scenario_steps(&sc)
                .into_iter()
                .map(|(title, body)| ScenarioStep { title, body })
                .collect();
            let (tickets, skipped) =
                create_from_steps(repo_root, &sc.id, &sc.workflow, &sc.product, steps)?;
            (sc.id, tickets, skipped)
        }
        "band" => {
            let band: u32 = id
                .parse()
                .map_err(|_| TicketError::BadRequest("band must be a number".into()))?;
            let md = fs::read_to_string(roadmap_path(repo_root))
                .map_err(|_| TicketError::Io("roadmap read failed".into()))?;
            let (scenario, steps) = steps_for_band(&md, band)?;
            let (tickets, skipped) =
                create_from_steps(repo_root, &scenario, "ticket-claim", "gsv", steps)?;
            (scenario, tickets, skipped)
        }
        "plan" => {
            let stem = sanitize_plan_stem(id)?;
            let path = plans_dir(repo_root).join(format!("{stem}.md"));
            let md = fs::read_to_string(&path).map_err(|_| TicketError::NotFound)?;
            let steps = parse_plan_open_items(&md);
            let scenario = format!("plan-{stem}");
            let scenario = title_chars(&scenario, 40);
            let (tickets, skipped) =
                create_from_steps(repo_root, &scenario, "ticket-claim", "gsv", steps)?;
            (scenario, tickets, skipped)
        }
        _ => {
            return Err(TicketError::BadRequest(
                "source must be scenario, band, or plan".into(),
            ))
        }
    };
    Ok(HookReport {
        ok: true,
        source,
        id: id.to_string(),
        scenario,
        tickets,
        skipped,
    })
}

/// HTTP/MCP hook wire. `phrase` wins over `source`+`id`.
pub fn wire_hook(
    repo_root: &Path,
    data_dir: &Path,
    body: &Value,
) -> Result<(HookReport, bool), TicketError> {
    let phrase = body
        .get("phrase")
        .and_then(Value::as_str)
        .or_else(|| body.get("body").and_then(Value::as_str))
        .unwrap_or("")
        .trim();
    let parsed = if !phrase.is_empty() {
        parse_hook_phrase(phrase)
    } else {
        None
    };
    let (source, id, walk) = if let Some(p) = parsed {
        (p.source, p.id, p.walk)
    } else {
        let source = body
            .get("source")
            .and_then(Value::as_str)
            .unwrap_or("scenario")
            .trim()
            .to_string();
        let id = body
            .get("id")
            .and_then(Value::as_str)
            .or_else(|| body.get("scenario_id").and_then(Value::as_str))
            .unwrap_or("")
            .trim()
            .to_string();
        let walk = body.get("walk").and_then(Value::as_bool).unwrap_or(false);
        (source, id, walk)
    };
    let report = hook_up(repo_root, data_dir, &source, &id)?;
    Ok((report, walk))
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

fn elapsed_ns(start: Instant) -> u64 {
    start.elapsed().as_nanos().min(u64::MAX as u128) as u64
}

fn bench_who() -> ClaimedBy {
    ClaimedBy {
        actor: "bench".into(),
        ide: "gsv_dev".into(),
        model: "grok-4.6".into(),
        agent: "bench".into(),
    }
}

const DEFAULT_SESSION_CATALOG: &str = r#"{
  "scenarios": [{
    "id": "abrakadabra-session",
    "title": "Abrakadabra session walk",
    "body": "bench",
    "workflow": "ticket-claim",
    "product": "gsv",
    "tickets": [
      {"title": "Session: S0 disk", "body": "a"},
      {"title": "Session: warnings-first", "body": "b"},
      {"title": "Session: close", "body": "c"}
    ]
  }]
}"#;

fn ensure_session_catalog(kit: &Path) -> Result<(), TicketError> {
    let path = scenarios_path(kit);
    if path.is_file() {
        let raw = fs::read_to_string(&path).unwrap_or_default();
        if raw.contains(SCENARIO_BENCH_ID) {
            return Ok(());
        }
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| TicketError::Io(e.to_string()))?;
    }
    fs::write(path, DEFAULT_SESSION_CATALOG).map_err(|e| TicketError::Io(e.to_string()))
}

fn seed_bench_kit(kit: &Path, repo_root: &Path) -> Result<(), TicketError> {
    fs::create_dir_all(kit.join("docs/gsv")).map_err(|e| TicketError::Io(e.to_string()))?;
    fs::create_dir_all(kit.join("data")).map_err(|e| TicketError::Io(e.to_string()))?;
    let src = scenarios_path(repo_root);
    if src.is_file() {
        let _ = fs::copy(&src, scenarios_path(kit));
    }
    ensure_session_catalog(kit)?;
    settings::save(
        &kit.join("data"),
        &settings::SettingsFile {
            workflows: settings::Workflows {
                enabled: vec!["ticket-claim".into()],
            },
            ..Default::default()
        },
    )
    .map_err(TicketError::Io)?;
    Ok(())
}

/// Time `abrakadabra-session` create_band + solo_walk on `kit`.
pub fn time_session_walk(kit: &Path, data_dir: &Path) -> Result<(u64, u64, usize), TicketError> {
    let t0 = Instant::now();
    let created = create_band_from_scenario(kit, data_dir, SCENARIO_BENCH_ID, "")?;
    let create_ns = elapsed_ns(t0);
    let t1 = Instant::now();
    let _report = solo_walk(kit, data_dir, None, bench_who(), SCENARIO_BENCH_ID)?;
    let walk_ns = elapsed_ns(t1);
    Ok((create_ns, walk_ns, created.len()))
}

/// Godfather / Galaxy line from persisted (or zero) timings.
pub fn scenario_bench_line(b: &ScenarioBench) -> String {
    format!(
        "bench gsv_dev create={} walk={} mds={} enqueue={} session={} ns",
        b.create_ns, b.walk_ns, b.mds_ns, b.enqueue_ns, b.session_walk_ns
    )
}

/// Load last scenario bench. Missing file → zeros (`ok:false`).
pub fn load_scenario_bench(repo_root: &Path) -> ScenarioBench {
    fs::read_to_string(scenario_bench_path(repo_root))
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

/// Persist scenario bench JSON under `docs/gsv/`.
pub fn save_scenario_bench(repo_root: &Path, bench: &ScenarioBench) -> Result<(), TicketError> {
    let path = scenario_bench_path(repo_root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| TicketError::Io(e.to_string()))?;
    }
    let raw = serde_json::to_string_pretty(bench).map_err(|e| TicketError::Io(e.to_string()))?;
    fs::write(path, format!("{raw}\n")).map_err(|e| TicketError::Io(e.to_string()))
}

/// Time a throwaway `abrakadabra-session` walk and write `scenario_bench.json`.
pub fn run_scenario_bench(repo_root: &Path) -> Result<ScenarioBench, TicketError> {
    let kit = std::env::temp_dir().join(format!(
        "gsv-scenario-bench-{}-{}",
        std::process::id(),
        unix_now()
    ));
    seed_bench_kit(&kit, repo_root)?;
    let data = kit.join("data");
    let (create_ns, walk_ns, walked) = time_session_walk(&kit, &data)?;
    let t_mds = Instant::now();
    let _ = crate::boxes::mds::report(repo_root);
    let mds_ns = elapsed_ns(t_mds);
    let t_enq = Instant::now();
    let _ = crate::boxes::telegram::session_line("bench", "bench", "session", "");
    let enqueue_ns = elapsed_ns(t_enq);
    let bench = ScenarioBench {
        ok: true,
        scenario: SCENARIO_BENCH_ID.into(),
        create_ns,
        walk_ns,
        session_walk_ns: create_ns.saturating_add(walk_ns),
        mds_ns,
        enqueue_ns,
        walked,
        recorded_at: now_rfc3339(),
    };
    save_scenario_bench(repo_root, &bench)?;
    Ok(bench)
}

/// `GET /api/tickets/bench` — last timings (empty-ok).
pub fn wire_bench(repo_root: &Path) -> Value {
    let b = load_scenario_bench(repo_root);
    let recorded = scenario_bench_path(repo_root).is_file();
    json!({
        "ok": true,
        "recorded": recorded,
        "scenario": if b.scenario.is_empty() { SCENARIO_BENCH_ID } else { b.scenario.as_str() },
        "create_ns": b.create_ns,
        "walk_ns": b.walk_ns,
        "session_walk_ns": b.session_walk_ns,
        "mds_ns": b.mds_ns,
        "enqueue_ns": b.enqueue_ns,
        "walked": b.walked,
        "recorded_at": b.recorded_at,
        "line": scenario_bench_line(&b),
    })
}

/// `POST /api/tickets/bench` `{run?:true}` — time + persist, or read last.
pub fn wire_bench_post(repo_root: &Path, body: &Value) -> Result<Value, TicketError> {
    let run = body.get("run").and_then(Value::as_bool).unwrap_or(false);
    if run {
        let _ = run_scenario_bench(repo_root)?;
    }
    Ok(wire_bench(repo_root))
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

    #[test]
    fn parse_hook_phrase_catalog_band_plan_and_json() {
        let p = parse_hook_phrase("run mcp bot hook up scenario abrakadabra-session").expect("p");
        assert_eq!(p.source, "scenario");
        assert_eq!(p.id, "abrakadabra-session");
        assert!(!p.walk);
        let b = parse_hook_phrase("run mcp bot hook up scenario band 176 walk").expect("b");
        assert_eq!(b.source, "band");
        assert_eq!(b.id, "176");
        assert!(b.walk);
        let plan =
            parse_hook_phrase("/hook plan 2026-08-19-gsv-settings-telegram-tickets").expect("plan");
        assert_eq!(plan.source, "plan");
        assert_eq!(plan.id, "2026-08-19-gsv-settings-telegram-tickets");
        let json =
            parse_hook_phrase(r#"{"v":1,"kind":"hook","source":"band","id":"177","walk":true}"#)
                .expect("json");
        assert_eq!(json.source, "band");
        assert_eq!(json.id, "177");
        assert!(json.walk);
        assert!(parse_hook_phrase("/ticket hi").is_none());
        assert!(parse_hook_phrase(r#"{"v":1,"kind":"ticket","body":"x"}"#).is_none());
    }

    #[test]
    fn parse_roadmap_open_and_closed_rows() {
        let md = r#"
## Спринти (band 176) — session walk

| Sprint | Фокус | Acceptance |
| **PH-S2399** | Scope | this band — **✅** |
| **PH-S2400** | Catalog | tickets[] — **✅** |

## Спринти (band 177) — hook up

| Sprint | Фокус | Acceptance |
| **PH-S2409** | Scope | owner pick — **[ ]** |
| **PH-S2410** | Parse | phrase grammar — **[ ]** |

## Ключові UX
"#;
        let bands = parse_roadmap_bands(md);
        assert_eq!(bands.len(), 2, "{bands:?}");
        assert_eq!(bands[0].band, 176);
        assert_eq!(bands[0].all.len(), 2);
        assert!(bands[0].open.is_empty());
        assert_eq!(bands[1].band, 177);
        assert_eq!(bands[1].open.len(), 2);
        assert!(bands[1].open[0].title.starts_with("PH-S2409"));
    }

    #[test]
    fn parse_plan_skips_done_checkboxes() {
        let md = "- [x] done already\n- [ ] Place hook tickets\n* [ ] Second open\n";
        let items = parse_plan_open_items(md);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].title, "Place hook tickets");
    }

    #[test]
    fn scenario_bench_round_trip_json() {
        let kit = std::env::temp_dir().join(format!(
            "gsv-bench-unit-{}-{}",
            std::process::id(),
            unix_now()
        ));
        let b = run_scenario_bench(&kit).expect("run");
        assert!(b.ok);
        assert_eq!(b.scenario, SCENARIO_BENCH_ID);
        assert!(b.session_walk_ns > 0, "{b:?}");
        assert!(b.walked >= 3, "{b:?}");
        let loaded = load_scenario_bench(&kit);
        assert_eq!(loaded.create_ns, b.create_ns);
        let wire = wire_bench(&kit);
        assert_eq!(wire["ok"], true);
        assert_eq!(wire["recorded"], true);
        assert!(
            wire["line"].as_str().unwrap_or("").contains("session="),
            "{wire}"
        );
        let empty = wire_bench(Path::new("/no/such/gsv-bench-kit"));
        assert_eq!(empty["ok"], true);
        assert_eq!(empty["recorded"], false);
    }
}
