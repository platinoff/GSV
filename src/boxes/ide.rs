//! IDE box — opencode + cursor chat sessions, read-only scan, selectable.
//!
//! Discovers chat/session artifacts under `~/.local/share/opencode/` (opencode
//! storage) and `.cursor/` (cursor config/sessions). The active selection is kept
//! in-memory on `AppState.ide_selection`.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::vision;

/// One discovered session/chat artifact.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IdeSession {
    /// Tool: opencode | cursor.
    pub tool: String,
    /// Stable session id (relative path).
    pub id: String,
    /// Human label (file/dir name).
    pub label: String,
    /// Absolute path (read-only).
    pub path: String,
    /// Modified timestamp (RFC3339 best effort).
    pub modified: String,
}

/// User selection of an IDE session.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IdeSelection {
    pub tool: String,
    pub session: String,
    pub selected_at: String,
}

/// One preview line from a selected `.jsonl` session.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IdePreviewLine {
    pub role: String,
    pub text: String,
}

/// `/api/ide/sessions` response wire.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdeWire {
    pub sessions: Vec<IdeSession>,
    pub selection: Option<IdeSelection>,
    #[serde(default)]
    pub preview: Vec<IdePreviewLine>,
    pub generated_at: String,
}

/// Home directory (USERPROFILE on Windows, HOME elsewhere).
pub fn home_dir() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(PathBuf::from))
}

fn mtime_rfc3339(path: &Path) -> String {
    let m = fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok())
        .unwrap_or(SystemTime::UNIX_EPOCH);
    crate::vision::system_to_rfc3339(m)
}

/// Scan opencode sessions under `{home}/.local/share/opencode/`.
fn scan_opencode() -> Vec<IdeSession> {
    let Some(home) = home_dir() else {
        return Vec::new();
    };
    let base = home.join(".local/share/opencode");
    let mut out = Vec::new();
    collect_jsonl(&base, "opencode", &mut out);
    out
}

/// Scan cursor artifacts under `{home}/.cursor/`.
fn scan_cursor() -> Vec<IdeSession> {
    let Some(home) = home_dir() else {
        return Vec::new();
    };
    let base = home.join(".cursor");
    let mut out = Vec::new();
    collect_jsonl(&base, "cursor", &mut out);
    if out.is_empty() {
        // Fall back to the repo-local `.cursor/` config (rules, README).
        if let Ok(read) = fs::read_dir(base.join("rules")) {
            for e in read.flatten() {
                if e.path().extension().is_some_and(|x| x == "mdc") {
                    out.push(IdeSession {
                        tool: "cursor".to_string(),
                        id: format!("cursor/rules/{}", e.file_name().to_string_lossy()),
                        label: e.file_name().to_string_lossy().to_string(),
                        path: e.path().to_string_lossy().to_string(),
                        modified: mtime_rfc3339(&e.path()),
                    });
                }
            }
        }
    }
    out
}

/// Collect `*.jsonl` chat/session files under `base` (recursive, capped).
fn collect_jsonl(base: &Path, tool: &str, out: &mut Vec<IdeSession>) {
    let Ok(read) = fs::read_dir(base) else {
        return;
    };
    let mut stack: Vec<PathBuf> = read.flatten().map(|e| e.path()).collect();
    let mut guard = 0usize;
    while let Some(p) = stack.pop() {
        guard += 1;
        if guard > 2000 {
            break;
        }
        let Ok(meta) = fs::metadata(&p) else {
            continue;
        };
        if meta.is_dir() {
            if let Ok(rd) = fs::read_dir(&p) {
                stack.extend(rd.flatten().map(|e| e.path()));
            }
            continue;
        }
        let is_chat = p.extension().is_some_and(|x| x == "jsonl")
            || p.file_name()
                .is_some_and(|n| n.to_string_lossy().contains("chat"));
        if !is_chat {
            continue;
        }
        out.push(IdeSession {
            tool: tool.to_string(),
            id: format!(
                "{tool}/{}",
                p.strip_prefix(base).unwrap_or(&p).to_string_lossy()
            ),
            label: p
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default(),
            path: p.to_string_lossy().to_string(),
            modified: mtime_rfc3339(&p),
        });
    }
    out.sort_by(|a, b| b.modified.cmp(&a.modified));
}

/// Discover all IDE sessions (opencode + cursor).
pub fn discover() -> Vec<IdeSession> {
    let mut sessions = scan_opencode();
    sessions.extend(scan_cursor());
    sessions
}

fn extract_preview_text(v: &Value) -> String {
    if let Some(s) = v.get("text").and_then(Value::as_str) {
        return s.to_string();
    }
    if let Some(s) = v.get("content").and_then(Value::as_str) {
        return s.to_string();
    }
    if let Some(s) = v
        .get("message")
        .and_then(|m| m.get("content"))
        .and_then(Value::as_str)
    {
        return s.to_string();
    }
    if let Some(parts) = v.get("parts").and_then(Value::as_array) {
        let joined: String = parts
            .iter()
            .filter_map(|p| p.get("text").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join(" ");
        if !joined.is_empty() {
            return joined;
        }
    }
    String::new()
}

fn truncate_preview(s: &str, max: usize) -> String {
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i >= max {
            out.push('…');
            break;
        }
        out.push(ch);
    }
    out
}

/// Last `n` messages from a session file (jsonl or small text). Never panics.
pub fn preview_messages(path: &Path, n: usize) -> Vec<IdePreviewLine> {
    let Ok(raw) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let slice = if raw.len() > 65_536 {
        let start = raw.len().saturating_sub(65_536);
        &raw[start..]
    } else {
        &raw
    };
    let mut out = Vec::new();
    for line in slice.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(v) = serde_json::from_str::<Value>(trimmed) {
            let role = v
                .get("role")
                .and_then(Value::as_str)
                .or_else(|| v.get("type").and_then(Value::as_str))
                .unwrap_or("message");
            let text = extract_preview_text(&v);
            if text.is_empty() {
                continue;
            }
            out.push(IdePreviewLine {
                role: role.to_string(),
                text: truncate_preview(text.trim(), 280),
            });
        } else if trimmed.len() > 8 {
            out.push(IdePreviewLine {
                role: "line".into(),
                text: truncate_preview(trimmed, 280),
            });
        }
    }
    if out.len() > n {
        let drop_n = out.len() - n;
        out.drain(0..drop_n);
    }
    out
}

/// Serve `/api/ide/sessions`.
pub fn wire(selection: Option<&IdeSelection>) -> IdeWire {
    let sessions = discover();
    let preview = selection
        .and_then(|sel| {
            sessions
                .iter()
                .find(|s| s.id == sel.session || s.path == sel.session)
        })
        .map(|s| preview_messages(Path::new(&s.path), 8))
        .unwrap_or_default();
    IdeWire {
        sessions,
        selection: selection.cloned(),
        preview,
        generated_at: vision::rfc3339_now(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discover_is_read_only() {
        // Must never panic; returns a list (possibly empty on CI).
        let _ = discover();
    }

    #[test]
    fn selection_serializes() {
        let sel = IdeSelection {
            tool: "opencode".to_string(),
            session: "opencode/abc.jsonl".to_string(),
            selected_at: "now".to_string(),
        };
        let raw = serde_json::to_string(&sel).expect("json");
        assert!(raw.contains("opencode"));
    }

    #[test]
    fn preview_messages_parses_jsonl_and_caps() {
        let dir = std::env::temp_dir().join("gsv_ide_preview_test");
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("chat.jsonl");
        let body = (0..12)
            .map(|i| format!(r#"{{"role":"user","text":"msg-{i}"}}"#))
            .collect::<Vec<_>>()
            .join("\n");
        fs::write(&path, body).expect("write");
        let lines = preview_messages(&path, 8);
        assert_eq!(lines.len(), 8);
        assert_eq!(lines[0].text, "msg-4");
        assert_eq!(lines[7].text, "msg-11");
        let _ = fs::remove_file(&path);
    }
}
