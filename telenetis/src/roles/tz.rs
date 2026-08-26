use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct TimezoneStore {
    user_tz: HashMap<String, Tz>,
}

impl TimezoneStore {
    pub fn new() -> Self {
        Self {
            user_tz: HashMap::new(),
        }
    }

    pub fn set_user_tz(&mut self, user_id: &str, tz: Tz) {
        self.user_tz.insert(user_id.to_string(), tz);
    }

    pub fn get_user_tz(&self, user_id: &str) -> Tz {
        self.user_tz.get(user_id).copied().unwrap_or(Tz::UTC)
    }

    pub fn convert_event_time(&self, event_ts: DateTime<Utc>, user_id: &str) -> String {
        let tz = self.get_user_tz(user_id);
        event_ts
            .with_timezone(&tz)
            .format("%Y-%m-%d %H:%M:%S %Z")
            .to_string()
    }
}
