//! Client-side (browser) JS error ring reported by the Galaxy UI.
//!
//! The page hooks `window.onerror` / `unhandledrejection` and posts each
//! event to `POST /api/ui/error` (beacon); this box keeps the last
//! [`MAX_ERRORS`] rows in `{data_dir}/ui_client_errors.jsonl` and serves
//! `GET /api/ui/errors`. The health wire carries [`count`] so the ops card
//! can show "UI JS errors: N" without an extra round-trip.

use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Ring capacity (newest [`MAX_ERRORS`] lines kept).
pub const MAX_ERRORS: usize = 50;
/// Message clamp (chars) so a flooded page cannot bloat the store.
pub const MAX_MESSAGE: usize = 400;
/// Source URL clamp (chars).
pub const MAX_SOURCE: usize = 300;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UiError {
    pub ts: String,
    pub message: String,
    pub source: String,
    pub line: u32,
}

fn store_path(data_dir: &Path) -> PathBuf {
    data_dir.join("ui_client_errors.jsonl")
}

fn clamp(text: &str, max: usize) -> String {
    text.chars().take(max).collect()
}

/// Append one browser-reported error, trimming the ring to [`MAX_ERRORS`].
pub fn record(data_dir: &Path, body: &Value) -> UiError {
    let err = UiError {
        ts: Utc::now().to_rfc3339(),
        message: clamp(
            body.get("message").and_then(Value::as_str).unwrap_or(""),
            MAX_MESSAGE,
        ),
        source: clamp(
            body.get("source").and_then(Value::as_str).unwrap_or(""),
            MAX_SOURCE,
        ),
        line: body.get("line").and_then(Value::as_u64).unwrap_or(0) as u32,
    };
    let path = store_path(data_dir);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let mut lines: Vec<String> = read(data_dir)
        .iter()
        .filter_map(|e| serde_json::to_string(e).ok())
        .collect();
    if let Ok(raw) = serde_json::to_string(&err) {
        lines.push(raw);
    }
    let start = lines.len().saturating_sub(MAX_ERRORS);
    let joined = lines[start..].join("\n");
    let _ = fs::write(&path, format!("{joined}\n"));
    err
}

/// Newest-last ring contents (missing/blank file → empty).
pub fn read(data_dir: &Path) -> Vec<UiError> {
    let raw = match fs::read_to_string(store_path(data_dir)) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    raw.lines()
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect()
}

/// Ring size (health row).
pub fn count(data_dir: &Path) -> usize {
    read(data_dir).len()
}

/// `GET /api/ui/errors` wire: `{ok, count, latest[≤10 newest-first]}`.
pub fn wire(data_dir: &Path) -> Value {
    let errs = read(data_dir);
    let latest: Vec<Value> = errs
        .iter()
        .rev()
        .take(10)
        .filter_map(|e| serde_json::to_value(e).ok())
        .collect();
    json!({"ok": true, "count": errs.len(), "latest": latest})
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("gsv-ui-errors-{name}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("mkdir");
        dir
    }

    #[test]
    fn record_then_read_roundtrip() {
        let dir = temp_dir("roundtrip");
        record(&dir, &json!({"message": "boom", "source": "ui", "line": 3}));
        let errs = read(&dir);
        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].message, "boom");
        assert_eq!(errs[0].line, 3);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn ring_trims_to_capacity_newest_wins() {
        let dir = temp_dir("ring");
        for i in 0..(MAX_ERRORS + 10) {
            record(&dir, &json!({"message": format!("m{i}")}));
        }
        let errs = read(&dir);
        assert_eq!(errs.len(), MAX_ERRORS);
        assert_eq!(errs.last().map(|e| e.message.as_str()), Some("m59"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn clamps_message_and_source() {
        let dir = temp_dir("clamp");
        let big = "x".repeat(MAX_MESSAGE + 100);
        let src = "y".repeat(MAX_SOURCE + 100);
        let err = record(&dir, &json!({"message": big, "source": src}));
        assert_eq!(err.message.chars().count(), MAX_MESSAGE);
        assert_eq!(err.source.chars().count(), MAX_SOURCE);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn wire_shape_latest_newest_first() {
        let dir = temp_dir("wire");
        record(&dir, &json!({"message": "old"}));
        record(&dir, &json!({"message": "new"}));
        let w = wire(&dir);
        assert_eq!(w["ok"], json!(true));
        assert_eq!(w["count"], json!(2));
        assert_eq!(w["latest"][0]["message"], json!("new"));
        assert_eq!(count(&dir), 2);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_store_is_empty() {
        let dir = temp_dir("missing");
        let _ = fs::remove_dir_all(&dir);
        assert!(read(&dir).is_empty());
        assert_eq!(count(&dir), 0);
    }
}
