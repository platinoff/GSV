//! Merit ranks — IT + army mix. Floor 0 (jun-nub). Channel host is marshal-orchestrator.
//!
//! Persist `{data}/gsv_ranks.json` (gitignored). Telegram ids stay off git.
//! Award +1 on ticket `done`. Demote −1 on ticket `error` or a failed `cargo test`
//! after commit (fingerprint + optional Telegram id). Never below 0, never above 15.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::fingerprint::{self, Fingerprint};
use super::settings;

/// Lowest merit (jun-nub). Cannot go below.
pub const MIN_LEVEL: u8 = 0;
/// Marshal-orchestrator (earned cap). Channel host also *displays* this title.
pub const MAX_LEVEL: u8 = 15;
/// Process env for the human Telegram id (`from.id`).
pub const TELEGRAM_ID_ENV: &str = "GSV_TELEGRAM_USER_ID";

static TG_OVERRIDE: Mutex<Option<String>> = Mutex::new(None);

/// Run `f` with a request-scoped Telegram id (HTTP/MCP `telegram_id`).
pub fn with_telegram<T>(telegram_id: &str, f: impl FnOnce() -> T) -> T {
    let t = telegram_id.trim();
    if !t.is_empty() {
        if let Ok(mut g) = TG_OVERRIDE.lock() {
            *g = Some(t.to_string());
        }
    }
    let out = f();
    if let Ok(mut g) = TG_OVERRIDE.lock() {
        *g = None;
    }
    out
}

/// One ladder rung: IT title mixed with a UA/NATO army grade.
#[derive(Debug, Clone, Copy)]
pub struct RankDef {
    pub level: u8,
    pub id: &'static str,
    pub title: &'static str,
    pub it: &'static str,
    pub army: &'static str,
}

/// 16 rungs. Research mix: UA ЗСУ enlisted/officers + NATO OR/OF + IT career.
pub const LADDER: [RankDef; 16] = [
    RankDef {
        level: 0,
        id: "jun-nub",
        title: "Jun-nub",
        it: "Intern",
        army: "Рядовий / Private",
    },
    RankDef {
        level: 1,
        id: "intern-private",
        title: "Intern-private",
        it: "Intern",
        army: "Солдат / Private+",
    },
    RankDef {
        level: 2,
        id: "trainee-soldier",
        title: "Trainee-soldier",
        it: "Trainee",
        army: "Старший солдат / PFC",
    },
    RankDef {
        level: 3,
        id: "junior-corporal",
        title: "Junior-corporal",
        it: "Junior",
        army: "Капрал / Corporal",
    },
    RankDef {
        level: 4,
        id: "associate-sergeant",
        title: "Associate-sergeant",
        it: "Associate",
        army: "Молодший сержант / Sergeant",
    },
    RankDef {
        level: 5,
        id: "middle-staff",
        title: "Middle-staff",
        it: "Middle",
        army: "Сержант / Staff Sergeant",
    },
    RankDef {
        level: 6,
        id: "senior-nco",
        title: "Senior-NCO",
        it: "Senior",
        army: "Старший сержант / SFC",
    },
    RankDef {
        level: 7,
        id: "lead-warrant",
        title: "Lead-warrant",
        it: "Lead",
        army: "Головний сержант / Warrant",
    },
    RankDef {
        level: 8,
        id: "staff-lieutenant",
        title: "Staff-lieutenant",
        it: "Staff",
        army: "Молодший лейтенант / 2LT",
    },
    RankDef {
        level: 9,
        id: "senior-lt",
        title: "Senior-lieutenant",
        it: "Senior+",
        army: "Лейтенант / 1LT",
    },
    RankDef {
        level: 10,
        id: "principal-captain",
        title: "Principal-captain",
        it: "Principal",
        army: "Капітан / Captain",
    },
    RankDef {
        level: 11,
        id: "architect-major",
        title: "Architect-major",
        it: "Architect",
        army: "Майор / Major",
    },
    RankDef {
        level: 12,
        id: "distinguished-ltcol",
        title: "Distinguished-ltcol",
        it: "Distinguished",
        army: "Підполковник / LtCol",
    },
    RankDef {
        level: 13,
        id: "fellow-colonel",
        title: "Fellow-colonel",
        it: "Fellow",
        army: "Полковник / Colonel",
    },
    RankDef {
        level: 14,
        id: "general-fellow",
        title: "General-fellow",
        it: "Distinguished Engineer",
        army: "Генерал / General",
    },
    RankDef {
        level: 15,
        id: "marshal-orchestrator",
        title: "Marshal-orchestrator",
        it: "Orchestrator",
        army: "Маршал / Marshal",
    },
];

/// Ladder slot for `level` (clamped).
pub fn def_for(level: u8) -> RankDef {
    LADDER[level.min(MAX_LEVEL) as usize]
}

/// Redacted badge for a fingerprint (host *displays* marshal-orchestrator).
pub fn badge_for(
    data_dir: &Path,
    actor: &str,
    ide: &str,
    agent: &str,
    host: bool,
) -> (String, String) {
    let file = load(&ranks_path(data_dir));
    let level = file
        .roster
        .iter()
        .find(|r| r.actor == actor && r.ide == ide && r.agent == agent)
        .map(|r| r.level)
        .unwrap_or(MIN_LEVEL);
    let d = if host {
        LADDER[MAX_LEVEL as usize]
    } else {
        def_for(level)
    };
    (d.id.to_string(), d.title.to_string())
}

/// Store path under the data dir (never `docs/`).
pub fn ranks_path(data_dir: &Path) -> PathBuf {
    data_dir.join("gsv_ranks.json")
}

/// Who is ranked (fingerprint + optional Telegram).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Identity {
    #[serde(default)]
    pub actor: String,
    #[serde(default)]
    pub ide: String,
    #[serde(default)]
    pub agent: String,
    /// Telegram `from.id`. Empty when unknown. Never returned in full on the wire.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub telegram_id: String,
}

impl Identity {
    /// Key: Telegram id wins so a human is one row across IDEs.
    pub fn key(&self) -> String {
        let tg = self.telegram_id.trim();
        if !tg.is_empty() {
            format!("tg:{tg}")
        } else {
            format!(
                "fp:{}|{}|{}",
                self.actor.trim(),
                self.ide.trim(),
                self.agent.trim()
            )
        }
    }
}

/// One person on the roster.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RosterRow {
    pub key: String,
    pub actor: String,
    pub ide: String,
    pub agent: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub telegram_id: String,
    pub level: u8,
    #[serde(default)]
    pub done_ok: u32,
    #[serde(default)]
    pub done_bad: u32,
}

impl RosterRow {
    fn from_identity(id: &Identity, level: u8) -> Self {
        Self {
            key: id.key(),
            actor: id.actor.clone(),
            ide: id.ide.clone(),
            agent: id.agent.clone(),
            telegram_id: id.telegram_id.trim().to_string(),
            level,
            done_ok: 0,
            done_bad: 0,
        }
    }
}

/// Last 20 award/demote notes (no full Telegram id).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RankEvent {
    pub ts: String,
    pub kind: String,
    pub actor: String,
    pub ide: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub telegram_tail: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub git_head: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub ticket_id: String,
    pub level: u8,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub note: String,
}

/// On-disk file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RanksFile {
    #[serde(default = "schema_one")]
    pub schema: u32,
    #[serde(default)]
    pub roster: Vec<RosterRow>,
    #[serde(default)]
    pub events: Vec<RankEvent>,
    /// `git_head` values already penalized for a failed test (idempotent).
    #[serde(default)]
    pub penalized_heads: Vec<String>,
}

fn schema_one() -> u32 {
    1
}

impl Default for RanksFile {
    fn default() -> Self {
        Self {
            schema: 1,
            roster: Vec::new(),
            events: Vec::new(),
            penalized_heads: Vec::new(),
        }
    }
}

/// Last 4 characters of a Telegram id (empty if missing).
pub fn telegram_tail(id: &str) -> String {
    let t = id.trim();
    if t.is_empty() {
        return String::new();
    }
    let n = t.chars().count();
    t.chars().skip(n.saturating_sub(4)).collect()
}

/// `GSV_TELEGRAM_USER_ID` or JSON `telegram_id`.
pub fn telegram_from(body: Option<&Value>) -> String {
    if let Some(v) = body
        .and_then(|b| b.get("telegram_id"))
        .and_then(Value::as_str)
    {
        let t = v.trim();
        if !t.is_empty() {
            return t.to_string();
        }
    }
    if let Ok(g) = TG_OVERRIDE.lock() {
        if let Some(s) = g
            .as_ref()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
        {
            return s;
        }
    }
    std::env::var(TELEGRAM_ID_ENV)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_default()
}

/// Identity from ticket claim fields + optional Telegram.
pub fn identity_from(actor: &str, ide: &str, agent: &str, telegram_id: &str) -> Identity {
    Identity {
        actor: actor.trim().to_string(),
        ide: ide.trim().to_string(),
        agent: agent.trim().to_string(),
        telegram_id: telegram_id.trim().to_string(),
    }
}

fn load(path: &Path) -> RanksFile {
    let Ok(raw) = fs::read_to_string(path) else {
        return RanksFile::default();
    };
    serde_json::from_str(&raw).unwrap_or_default()
}

fn save(path: &Path, file: &RanksFile) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let raw = serde_json::to_string_pretty(file).map_err(|e| e.to_string())?;
    fs::write(path, raw).map_err(|e| e.to_string())
}

fn find_row_mut<'a>(file: &'a mut RanksFile, id: &Identity) -> &'a mut RosterRow {
    let key = id.key();
    if let Some(i) = file.roster.iter().position(|r| r.key == key) {
        return &mut file.roster[i];
    }
    if !id.telegram_id.trim().is_empty() {
        let fp = format!(
            "fp:{}|{}|{}",
            id.actor.trim(),
            id.ide.trim(),
            id.agent.trim()
        );
        if let Some(i) = file.roster.iter().position(|r| r.key == fp) {
            file.roster[i].key = key.clone();
            file.roster[i].telegram_id = id.telegram_id.trim().to_string();
            return &mut file.roster[i];
        }
    }
    file.roster.push(RosterRow::from_identity(id, MIN_LEVEL));
    file.roster.last_mut().expect("just pushed")
}

fn push_event(file: &mut RanksFile, ev: RankEvent) {
    file.events.push(ev);
    if file.events.len() > 20 {
        let drop = file.events.len() - 20;
        file.events.drain(0..drop);
    }
}

fn apply(
    data_dir: &Path,
    id: &Identity,
    delta: i8,
    kind: &str,
    ticket_id: &str,
    git_head: &str,
    note: &str,
) -> Result<RosterRow, String> {
    let path = ranks_path(data_dir);
    let mut file = load(&path);
    let level = {
        let row = find_row_mut(&mut file, id);
        if delta > 0 {
            row.level = row.level.saturating_add(delta as u8).min(MAX_LEVEL);
            row.done_ok = row.done_ok.saturating_add(1);
        } else if delta < 0 {
            row.level = row.level.saturating_sub((-delta) as u8);
            row.done_bad = row.done_bad.saturating_add(1);
        }
        row.actor = id.actor.clone();
        row.ide = id.ide.clone();
        row.agent = id.agent.clone();
        row.level
    };
    push_event(
        &mut file,
        RankEvent {
            ts: crate::vision::rfc3339_now(),
            kind: kind.into(),
            actor: id.actor.clone(),
            ide: id.ide.clone(),
            telegram_tail: telegram_tail(&id.telegram_id),
            git_head: git_head.trim().to_string(),
            ticket_id: ticket_id.trim().to_string(),
            level,
            note: note.trim().to_string(),
        },
    );
    save(&path, &file)?;
    let row = file
        .roster
        .iter()
        .find(|r| r.key == id.key())
        .cloned()
        .unwrap_or_else(|| RosterRow::from_identity(id, level));
    Ok(row)
}

/// +1 rank for a completed ticket (cap 15).
pub fn award(
    data_dir: &Path,
    id: &Identity,
    ticket_id: &str,
    note: &str,
) -> Result<RosterRow, String> {
    apply(data_dir, id, 1, "award", ticket_id, "", note)
}

/// −1 rank (floor 0) for a bad ticket or failed tests.
pub fn demote(
    data_dir: &Path,
    id: &Identity,
    ticket_id: &str,
    git_head: &str,
    note: &str,
) -> Result<RosterRow, String> {
    apply(data_dir, id, -1, "demote", ticket_id, git_head, note)
}

/// Ticket `done` helper.
pub fn on_ticket_done(
    data_dir: &Path,
    actor: &str,
    ide: &str,
    agent: &str,
    telegram_id: &str,
    ticket_id: &str,
    note: &str,
) {
    let id = identity_from(actor, ide, agent, telegram_id);
    let _ = award(data_dir, &id, ticket_id, note);
}

/// Ticket `error` helper.
pub fn on_ticket_error(
    data_dir: &Path,
    actor: &str,
    ide: &str,
    agent: &str,
    telegram_id: &str,
    ticket_id: &str,
    note: &str,
) {
    let id = identity_from(actor, ide, agent, telegram_id);
    let _ = demote(data_dir, &id, ticket_id, "", note);
}

fn latest_failed_test(repo_root: &Path) -> Option<(String, bool)> {
    let path = repo_root.join("docs/vision/speed_index.json");
    let raw = fs::read_to_string(path).ok()?;
    let v: Value = serde_json::from_str(&raw).ok()?;
    let ok = v
        .pointer("/latest/test_ci_ok")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let mut head = v
        .get("git_head")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    if let Some(hist) = v.get("test_ci_history").and_then(Value::as_array) {
        if let Some(last) = hist.last() {
            if last.get("ok").and_then(Value::as_bool) == Some(false) {
                if let Some(h) = last.get("git_head").and_then(Value::as_str) {
                    if !h.trim().is_empty() {
                        head = h.trim().to_string();
                    }
                }
            }
        }
    }
    Some((head, ok))
}

fn fingerprint_for_head(repo_root: &Path, head: &str) -> Option<Fingerprint> {
    let head = head.trim();
    if head.is_empty() {
        return None;
    }
    let rows = fingerprint::latest(&fingerprint::jsonl_path(repo_root), 40);
    rows.into_iter().find(|fp| {
        fp.git_head.as_deref().unwrap_or("").starts_with(head)
            || head.starts_with(fp.git_head.as_deref().unwrap_or(""))
    })
}

/// If the latest recorded `cargo test` failed after a commit, demote that fingerprint once.
pub fn review_failed_tests(repo_root: &Path, data_dir: &Path) -> Option<RosterRow> {
    let (head, ok) = latest_failed_test(repo_root)?;
    if ok || head.is_empty() {
        return None;
    }
    let path = ranks_path(data_dir);
    let file = load(&path);
    if file.penalized_heads.iter().any(|h| h == &head) {
        return None;
    }
    let fp = fingerprint_for_head(repo_root, &head);
    let id = if let Some(fp) = fp {
        identity_from(&fp.actor, &fp.ide, &fp.agent, &telegram_from(None))
    } else {
        identity_from("agent", "cursor", "orchestrator", &telegram_from(None))
    };
    let row = demote(data_dir, &id, "", &head, "tests failed after commit").ok()?;
    let mut file = load(&path);
    file.penalized_heads.push(head);
    if file.penalized_heads.len() > 40 {
        let drop = file.penalized_heads.len() - 40;
        file.penalized_heads.drain(0..drop);
    }
    let _ = save(&path, &file);
    Some(row)
}

fn redact_row(row: &RosterRow, host: bool) -> Value {
    let d = def_for(row.level);
    let tg_set = !row.telegram_id.is_empty();
    let key = if tg_set {
        format!("tg:…{}", telegram_tail(&row.telegram_id))
    } else {
        row.key.clone()
    };
    json!({
        "key": key,
        "actor": row.actor,
        "ide": row.ide,
        "agent": row.agent,
        "telegram_set": tg_set,
        "telegram_tail": telegram_tail(&row.telegram_id),
        "level": row.level,
        "rank_id": d.id,
        "title": d.title,
        "it": d.it,
        "army": d.army,
        "done_ok": row.done_ok,
        "done_bad": row.done_bad,
        "channel_marshal": host && row.level < MAX_LEVEL,
        "display_title": if host { LADDER[MAX_LEVEL as usize].title } else { d.title },
    })
}

/// `GET /api/ranks` / Galaxy / MCP (redacted Telegram).
pub fn wire(repo_root: &Path, data_dir: &Path) -> Value {
    let _ = review_failed_tests(repo_root, data_dir);
    let file = load(&ranks_path(data_dir));
    let settings = settings::load_result(data_dir).unwrap_or_default();
    let role = settings::chat_role(&settings);
    let host = role == "host";
    let ladder: Vec<Value> = LADDER
        .iter()
        .map(|d| {
            json!({
                "level": d.level,
                "id": d.id,
                "title": d.title,
                "it": d.it,
                "army": d.army,
            })
        })
        .collect();
    let roster: Vec<Value> = file.roster.iter().map(|r| redact_row(r, host)).collect();
    let events: Vec<Value> = file
        .events
        .iter()
        .rev()
        .take(10)
        .map(|e| {
            json!({
                "ts": e.ts,
                "kind": e.kind,
                "actor": e.actor,
                "ide": e.ide,
                "telegram_tail": e.telegram_tail,
                "git_head": e.git_head,
                "ticket_id": e.ticket_id,
                "level": e.level,
                "note": e.note,
            })
        })
        .collect();
    json!({
        "ok": true,
        "min_level": MIN_LEVEL,
        "max_level": MAX_LEVEL,
        "chat_role": role,
        "host_title": if host { LADDER[MAX_LEVEL as usize].title } else { "" },
        "ladder": ladder,
        "roster": roster,
        "events": events,
    })
}

/// HTTP POST / MCP mutate (`action` = list|award|demote|review).
pub fn wire_post(repo_root: &Path, data_dir: &Path, body: &Value) -> Result<Value, String> {
    let action = body
        .get("action")
        .and_then(Value::as_str)
        .unwrap_or("list")
        .trim();
    match action {
        "" | "list" => Ok(wire(repo_root, data_dir)),
        "award" | "demote" => {
            let actor = body.get("actor").and_then(Value::as_str).unwrap_or("agent");
            let ide = body.get("ide").and_then(Value::as_str).unwrap_or("cursor");
            let agent = body
                .get("agent")
                .and_then(Value::as_str)
                .unwrap_or("orchestrator");
            let tg = telegram_from(Some(body));
            let id = identity_from(actor, ide, agent, &tg);
            let ticket = body.get("ticket_id").and_then(Value::as_str).unwrap_or("");
            let note = body.get("note").and_then(Value::as_str).unwrap_or("");
            let head = body.get("git_head").and_then(Value::as_str).unwrap_or("");
            if action == "award" {
                award(data_dir, &id, ticket, note)?;
            } else {
                demote(data_dir, &id, ticket, head, note)?;
            }
            Ok(wire(repo_root, data_dir))
        }
        "review" => {
            let tests_ok = body
                .get("tests_ok")
                .and_then(Value::as_bool)
                .unwrap_or(true);
            if !tests_ok {
                let actor = body.get("actor").and_then(Value::as_str).unwrap_or("");
                let ide = body.get("ide").and_then(Value::as_str).unwrap_or("");
                let agent = body.get("agent").and_then(Value::as_str).unwrap_or("");
                let tg = telegram_from(Some(body));
                let head = body.get("git_head").and_then(Value::as_str).unwrap_or("");
                if !actor.trim().is_empty() {
                    let id = identity_from(actor, ide, agent, &tg);
                    demote(data_dir, &id, "", head, "tests failed after commit")?;
                    if !head.trim().is_empty() {
                        let path = ranks_path(data_dir);
                        let mut file = load(&path);
                        if !file.penalized_heads.iter().any(|h| h == head) {
                            file.penalized_heads.push(head.to_string());
                            let _ = save(&path, &file);
                        }
                    }
                } else {
                    let _ = review_failed_tests(repo_root, data_dir);
                }
            }
            Ok(wire(repo_root, data_dir))
        }
        other => Err(format!("unknown action {other}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn tmp() -> PathBuf {
        let n = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(1);
        let p = std::env::temp_dir().join(format!("gsv-ranks-{n}"));
        let _ = fs::create_dir_all(&p);
        p
    }

    #[test]
    fn ladder_covers_zero_to_marshal() {
        assert_eq!(LADDER.len(), 16);
        assert_eq!(LADDER[0].id, "jun-nub");
        assert_eq!(LADDER[15].id, "marshal-orchestrator");
        assert_eq!(def_for(99).id, "marshal-orchestrator");
    }

    #[test]
    fn demote_floors_at_zero() {
        let dir = tmp();
        let id = identity_from("alice", "cursor", "bot", "123456789");
        let row = demote(&dir, &id, "t1", "abc", "bad").unwrap();
        assert_eq!(row.level, 0);
        assert_eq!(row.done_bad, 1);
        assert_eq!(telegram_tail("123456789"), "6789");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn award_climbs_and_caps() {
        let dir = tmp();
        let id = identity_from("bob", "opencode", "bot", "");
        let mut level = 0u8;
        for _ in 0..20 {
            level = award(&dir, &id, "t", "ok").unwrap().level;
        }
        assert_eq!(level, MAX_LEVEL);
        let wire = wire(Path::new("."), &dir);
        assert_eq!(wire["ok"], true);
        assert!(wire.to_string().contains("marshal-orchestrator"));
        assert!(!wire.to_string().contains("123456789"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn telegram_key_merges_fingerprint_row() {
        let dir = tmp();
        let fp = identity_from("agent", "cursor", "orchestrator", "");
        award(&dir, &fp, "a", "").unwrap();
        let tg = identity_from("agent", "cursor", "orchestrator", "42");
        let row = award(&dir, &tg, "b", "").unwrap();
        assert_eq!(row.level, 2);
        assert_eq!(row.key, "tg:42");
        let file = load(&ranks_path(&dir));
        assert_eq!(file.roster.len(), 1);
        let _ = fs::remove_dir_all(&dir);
    }
}
