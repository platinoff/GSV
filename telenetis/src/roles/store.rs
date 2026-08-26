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
}
