//! Host-test consent registry (T16).
//!
//! The owner opts a phone into host-driven testing from inside the Mini App
//! ("allow tests from host" toggle = the cookie/permission checkbox): the
//! toggle POSTs here with the Telegram `initData` handshake, and the server
//! records the *verified* user id — never a client-claimed one. The test
//! harness (host/KVM side) reads the opted-in list and enqueues only
//! allowlisted test tasks (`test_ping` in v1 — no model, no downloads) to
//! those users; the phone worker additionally checks its own local toggle.
//!
//! Storage mirrors [`crate::roles::store`]: JSONL in the crate `data/` dir
//! (`TELENETIS_TESTMODE_FILE` overrides), gitignored runtime data.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// One consent record: a Telegram user id plus the switch position.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConsentEntry {
    pub user_id: String,
    pub on: bool,
    pub updated_at: String,
}

/// In-memory consent set keyed by Telegram user id.
#[derive(Debug, Clone, Default)]
pub struct ConsentStore {
    entries: HashMap<String, ConsentEntry>,
}

impl ConsentStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set (or overwrite) the switch. `on=false` revokes: the entry is
    /// removed, so absence always means "no consent".
    pub fn set(&mut self, user_id: &str, on: bool) {
        let id = user_id.trim().to_string();
        if id.is_empty() {
            return;
        }
        if on {
            self.entries.insert(
                id.clone(),
                ConsentEntry {
                    user_id: id,
                    on: true,
                    updated_at: chrono::Utc::now().to_rfc3339(),
                },
            );
        } else {
            self.entries.remove(&id);
        }
    }

    pub fn opted_in(&self, user_id: &str) -> bool {
        self.entries.contains_key(user_id.trim())
    }

    /// Opted-in user ids, sorted (deterministic wire + tests).
    pub fn list_opted_in(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.entries.keys().cloned().collect();
        ids.sort();
        ids
    }

    pub fn save_jsonl(&self, path: &std::path::Path) -> std::io::Result<()> {
        let jsonl = self
            .entries
            .values()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .map(|v| v.join("\n"))
            .unwrap_or_default();
        std::fs::write(path, jsonl)
    }

    pub fn load_jsonl(path: &std::path::Path) -> Self {
        let content = std::fs::read_to_string(path).unwrap_or_default();
        let mut store = Self::new();
        for line in content.lines() {
            if let Ok(entry) = serde_json::from_str::<ConsentEntry>(line) {
                if entry.on && !entry.user_id.trim().is_empty() {
                    store.entries.insert(entry.user_id.clone(), entry);
                }
            }
        }
        store
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_on_off_roundtrip() {
        let mut s = ConsentStore::new();
        assert!(!s.opted_in("123"));
        s.set("123", true);
        assert!(s.opted_in("123"));
        s.set("123", false);
        assert!(!s.opted_in("123"));
    }

    #[test]
    fn blank_user_never_stored() {
        let mut s = ConsentStore::new();
        s.set("   ", true);
        assert!(s.list_opted_in().is_empty());
    }

    #[test]
    fn list_is_sorted_and_deduped() {
        let mut s = ConsentStore::new();
        s.set("999", true);
        s.set("111", true);
        s.set("999", true);
        assert_eq!(
            s.list_opted_in(),
            vec!["111".to_string(), "999".to_string()]
        );
    }

    #[test]
    fn persist_roundtrip_keeps_only_on() {
        let dir = std::env::temp_dir().join(format!("tns-consent-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("testmode.jsonl");
        let mut s = ConsentStore::new();
        s.set("123", true);
        s.set("456", false);
        s.save_jsonl(&file).unwrap();
        let reloaded = ConsentStore::load_jsonl(&file);
        assert_eq!(reloaded.list_opted_in(), vec!["123".to_string()]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_ignores_garbage_lines() {
        let dir = std::env::temp_dir().join(format!("tns-consent-g-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("testmode.jsonl");
        std::fs::write(
            &file,
            "not json\n{\"user_id\":\"7\",\"on\":true,\"updated_at\":\"t\"}\n",
        )
        .unwrap();
        let reloaded = ConsentStore::load_jsonl(&file);
        assert_eq!(reloaded.list_opted_in(), vec!["7".to_string()]);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
