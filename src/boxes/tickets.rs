//! Ticket board + MCP claim (band 168).
//!
//! Source of truth: `{kit}/docs/gsv/tickets.jsonl` (git-tracked, no secrets).
//! Claims append `{kit}/docs/gsv/ticket_claims.jsonl` — sibling of fingerprints,
//! never mixed into drain JSONL. Missing files → empty list `{ok:true,tickets:[]}`.

use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::fingerprint;
use super::settings;

/// Canonical tickets JSONL (kit repo, not `data/`).
pub fn tickets_path(repo_root: &Path) -> PathBuf {
    repo_root.join("docs/gsv/tickets.jsonl")
}

/// Canonical claim log (sibling of fingerprints).
pub fn claims_path(repo_root: &Path) -> PathBuf {
    repo_root.join("docs/gsv/ticket_claims.jsonl")
}

/// Who claimed a ticket (fingerprint-class fields).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaimedBy {
    pub actor: String,
    pub ide: String,
    pub model: String,
    pub agent: String,
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
}

fn default_product() -> String {
    "gsv".to_string()
}

/// Append-only claim row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TicketClaim {
    pub ticket_id: String,
    pub ts: String,
    pub actor: String,
    pub ide: String,
    pub model: String,
    pub agent: String,
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

/// `GET /api/tickets` / MCP `gsv_tickets`.
pub fn list(repo_root: &Path) -> Value {
    match read_tickets(&tickets_path(repo_root)) {
        Ok(tickets) => json!({ "ok": true, "tickets": tickets }),
        Err(e) => json!({ "ok": false, "error": e.to_string() }),
    }
}

/// Create an `open` ticket (no workflow gate).
pub fn create(
    repo_root: &Path,
    title: &str,
    body: &str,
    product: &str,
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
    };
    tickets.push(ticket.clone());
    write_tickets(&path, &tickets)?;
    Ok(ticket)
}

/// Claim an open ticket: `open` → `in_progress`, append claim JSONL.
pub fn claim(
    repo_root: &Path,
    data_dir: &Path,
    id: &str,
    who: ClaimedBy,
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
    let ts = now_rfc3339();
    tickets[pos].status = "in_progress".into();
    tickets[pos].claimed_by = Some(who.clone());
    let claimed = tickets[pos].clone();
    write_tickets(&path, &tickets)?;
    append_claim(
        &claims_path(repo_root),
        &TicketClaim {
            ticket_id: claimed.id.clone(),
            ts,
            actor: who.actor,
            ide: who.ide,
            model: who.model,
            agent: who.agent,
        },
    )?;
    Ok(claimed)
}

/// HTTP POST create wire.
pub fn wire_create(repo_root: &Path, body: &Value) -> Result<Value, TicketError> {
    let title = body.get("title").and_then(Value::as_str).unwrap_or("");
    let tbody = body.get("body").and_then(Value::as_str).unwrap_or("");
    let product = body.get("product").and_then(Value::as_str).unwrap_or("gsv");
    let ticket = create(repo_root, title, tbody, product)?;
    Ok(json!({ "ok": true, "ticket": ticket }))
}

/// HTTP POST claim wire.
pub fn wire_claim(repo_root: &Path, data_dir: &Path, body: &Value) -> Result<Value, TicketError> {
    let id = body.get("id").and_then(Value::as_str).unwrap_or("");
    let ticket = claim(repo_root, data_dir, id, resolve_claimed_by())?;
    Ok(json!({ "ok": true, "ticket": ticket }))
}
