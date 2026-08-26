use super::client::GsvClient;
use crate::error::TelenetisError;
use crate::state::TicketRow;

pub async fn sync_tickets(
    client: &GsvClient,
    state: &crate::state::AppState,
) -> Result<(), TelenetisError> {
    let resp = client.tickets().await?;
    let rows = resp["wire"]
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
        .unwrap_or_default();
    state.set_tickets(rows).await;
    Ok(())
}
