use crate::state::BusEnvelope;
use chrono::{DateTime, Utc};
use serde_json::Value;

#[derive(Debug, serde::Deserialize)]
struct RawEnvelope {
    v: u8,
    kind: String,
    body: String,
    from: Option<String>,
    ts: Option<String>,
    data: Option<Value>,
}

pub fn parse_bus_envelope(json_str: &str) -> Result<BusEnvelope, serde_json::Error> {
    let raw: RawEnvelope = serde_json::from_str(json_str)?;
    Ok(BusEnvelope {
        v: raw.v,
        kind: raw.kind,
        body: raw.body,
        from: raw.from.unwrap_or_default(),
        ts: raw
            .ts
            .and_then(|s| s.parse::<DateTime<Utc>>().ok())
            .unwrap_or_else(Utc::now),
        data: raw.data,
    })
}

pub fn format_bus_envelope(env: &BusEnvelope) -> String {
    serde_json::json!({
        "v": env.v,
        "kind": env.kind,
        "body": env.body,
        "from": env.from,
        "ts": env.ts.to_rfc3339(),
        "data": env.data,
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn parse_presence_envelope() {
        let json = r#"{"v":1,"kind":"presence","body":"hello","from":"jail-01","ts":"2026-08-26T00:00:00Z"}"#;
        let env = parse_bus_envelope(json).unwrap();
        assert_eq!(env.kind, "presence");
        assert_eq!(env.from, "jail-01");
        assert_eq!(env.v, 1);
    }

    #[test]
    fn parse_sync_envelope_with_data() {
        let json =
            r#"{"v":1,"kind":"sync","body":"sync body","from":"gsv","data":{"hint":"test"}}"#;
        let env = parse_bus_envelope(json).unwrap();
        assert_eq!(env.kind, "sync");
        assert!(env.data.is_some());
    }

    #[test]
    fn format_roundtrip() {
        let env = crate::state::BusEnvelope {
            v: 1,
            kind: "bus".to_string(),
            body: "test body".to_string(),
            from: "jail-01".to_string(),
            ts: Utc::now(),
            data: None,
        };
        let s = format_bus_envelope(&env);
        let parsed = parse_bus_envelope(&s).unwrap();
        assert_eq!(parsed.kind, env.kind);
        assert_eq!(parsed.body, env.body);
    }

    #[test]
    fn parse_invalid_json_errors() {
        assert!(parse_bus_envelope("not json").is_err());
    }
}
