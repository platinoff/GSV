use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct TimezoneStore {
    user_tz: HashMap<String, Tz>,
}

impl Default for TimezoneStore {
    fn default() -> Self {
        Self::new()
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn default_tz_is_utc() {
        let store = TimezoneStore::new();
        assert_eq!(store.get_user_tz("unknown"), Tz::UTC);
    }

    #[test]
    fn set_and_get_tz() {
        let mut store = TimezoneStore::new();
        store.set_user_tz("user1", Tz::EST);
        assert_eq!(store.get_user_tz("user1"), Tz::EST);
    }

    #[test]
    fn convert_time_contains_year() {
        let store = TimezoneStore::new();
        let ts = Utc.with_ymd_and_hms(2026, 8, 26, 12, 0, 0).unwrap();
        let s = store.convert_event_time(ts, "unknown");
        assert!(s.contains("2026"));
        assert!(s.contains("UTC"));
    }
}
