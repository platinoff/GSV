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
