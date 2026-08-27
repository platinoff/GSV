use super::client::GsvClient;
use crate::error::TelenetisError;
use crate::state::TicketRow;

pub async fn sync_tickets(
    client: &GsvClient,
    state: &crate::state::AppState,
) -> Result<(), TelenetisError> {
    let resp = client.tickets().await?;
    let rows = parse_ticket_rows(&resp);
    state.set_tickets(rows).await;
    Ok(())
}

pub fn parse_ticket_rows(resp: &serde_json::Value) -> Vec<TicketRow> {
    resp["wire"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| {
                    Some(TicketRow {
                        id: v["id"].as_str()?.to_string(),
                        title: v["title"].as_str().unwrap_or("").to_string(),
                        body: v["body"].as_str().unwrap_or("").to_string(),
                        status: v["status"].as_str().unwrap_or("open").to_string(),
                        product: v["product"].as_str().unwrap_or("gsv").to_string(),
                        claimed_by: v["claimed_by"].as_str().map(|s| s.to_string()),
                        scenario: v["scenario"].as_str().map(|s| s.to_string()),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_empty_wire() {
        let resp = json!({});
        assert!(parse_ticket_rows(&resp).is_empty());
    }

    #[test]
    fn parse_empty_array() {
        let resp = json!({"wire": []});
        assert!(parse_ticket_rows(&resp).is_empty());
    }

    #[test]
    fn parse_full_row() {
        let resp = json!({"wire": [{
            "id": "t-1",
            "title": "Fix bug",
            "body": "desc",
            "status": "open",
            "product": "gsv",
            "claimed_by": "agent-01",
            "scenario": "telenetis-setup"
        }]});
        let rows = parse_ticket_rows(&resp);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, "t-1");
        assert_eq!(rows[0].title, "Fix bug");
        assert_eq!(rows[0].status, "open");
        assert_eq!(rows[0].product, "gsv");
        assert_eq!(rows[0].claimed_by.as_deref(), Some("agent-01"));
        assert_eq!(rows[0].scenario.as_deref(), Some("telenetis-setup"));
    }

    #[test]
    fn parse_missing_optional_fields() {
        let resp = json!({"wire": [{
            "id": "t-2",
            "title": null,
            "body": null,
            "status": null,
            "product": null
        }]});
        let rows = parse_ticket_rows(&resp);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, "t-2");
        assert_eq!(rows[0].title, "");
        assert_eq!(rows[0].status, "open");
        assert_eq!(rows[0].product, "gsv");
        assert!(rows[0].claimed_by.is_none());
        assert!(rows[0].scenario.is_none());
    }

    #[test]
    fn parse_skips_row_without_id() {
        let resp = json!({"wire": [
            {"title": "no id"},
            {"id": "t-3", "title": "has id"}
        ]});
        let rows = parse_ticket_rows(&resp);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, "t-3");
    }

    #[test]
    fn parse_multiple_rows() {
        let resp = json!({"wire": [
            {"id": "t-10", "title": "A"},
            {"id": "t-11", "title": "B"},
            {"id": "t-12", "title": "C"}
        ]});
        assert_eq!(parse_ticket_rows(&resp).len(), 3);
    }
}
