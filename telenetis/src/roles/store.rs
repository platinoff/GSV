use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Role {
    Host,
    Mate,
    Guest,
    Observer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleEntry {
    pub jail_id: String,
    pub role: Role,
    pub assigned_at: String,
}

#[derive(Debug, Clone, Default)]
pub struct RoleStore {
    entries: HashMap<String, RoleEntry>,
}

impl RoleStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn assign_role(&mut self, jail_id: &str, role: Role) {
        self.entries.insert(
            jail_id.to_string(),
            RoleEntry {
                jail_id: jail_id.to_string(),
                role,
                assigned_at: chrono::Utc::now().to_rfc3339(),
            },
        );
    }

    pub fn list_roles(&self) -> Vec<RoleEntry> {
        self.entries.values().cloned().collect()
    }

    pub fn remove_role(&mut self, jail_id: &str) -> Option<RoleEntry> {
        self.entries.remove(jail_id)
    }

    pub fn get_role(&self, jail_id: &str) -> Option<Role> {
        self.entries.get(jail_id).map(|e| e.role.clone())
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
            if let Ok(entry) = serde_json::from_str::<RoleEntry>(line) {
                store.entries.insert(entry.jail_id.clone(), entry);
            }
        }
        store
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assign_and_get_role() {
        let mut store = RoleStore::new();
        store.assign_role("jail-01", Role::Host);
        assert_eq!(store.get_role("jail-01"), Some(Role::Host));
    }

    #[test]
    fn list_and_remove() {
        let mut store = RoleStore::new();
        store.assign_role("j1", Role::Mate);
        store.assign_role("j2", Role::Guest);
        assert_eq!(store.list_roles().len(), 2);
        let removed = store.remove_role("j1");
        assert!(removed.is_some());
        assert_eq!(store.list_roles().len(), 1);
    }

    #[test]
    fn overwrite_role() {
        let mut store = RoleStore::new();
        store.assign_role("j1", Role::Guest);
        store.assign_role("j1", Role::Host);
        assert_eq!(store.get_role("j1"), Some(Role::Host));
    }

    #[test]
    fn persist_jsonl_roundtrip() {
        let mut store = RoleStore::new();
        store.assign_role("j1", Role::Host);
        store.assign_role("j2", Role::Mate);
        let path = std::env::temp_dir().join("telenetis_roles_test.jsonl");
        store.save_jsonl(&path).unwrap();
        let loaded = RoleStore::load_jsonl(&path);
        assert_eq!(loaded.get_role("j1"), Some(Role::Host));
        assert_eq!(loaded.get_role("j2"), Some(Role::Mate));
        let _ = std::fs::remove_file(&path);
    }
}
