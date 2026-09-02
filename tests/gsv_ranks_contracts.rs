//! Ranks box — merit ladder contracts. No Telegram secrets on the wire.

use gsv::boxes::ranks::{self, LADDER, MAX_LEVEL, MIN_LEVEL};
use gsv::mcp;
use serde_json::json;

#[test]
fn ladder_bounds() {
    assert_eq!(MIN_LEVEL, 0);
    assert_eq!(MAX_LEVEL, 15);
    assert_eq!(LADDER[0].id, "jun-nub");
    assert_eq!(LADDER[15].id, "marshal-orchestrator");
}

#[test]
fn mcp_exposes_ranks_and_doc() {
    assert!(mcp::tool_names().contains(&"gsv_ranks"));
    assert_eq!(mcp::tool_names().len(), 57);
}

#[test]
fn wire_redacts_telegram() {
    let dir = std::env::temp_dir().join(format!(
        "gsv-ranks-ct-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(1)
    ));
    let _ = std::fs::create_dir_all(&dir);
    let id = ranks::identity_from("agent", "cursor", "orchestrator", "999888777");
    ranks::award(&dir, &id, "t1", "ok").unwrap();
    let v = ranks::wire(std::path::Path::new("."), &dir);
    let s = v.to_string();
    assert!(s.contains("telegram_tail"));
    assert!(!s.contains("999888777"), "{s}");
    assert!(s.contains("8777"), "{s}");
    assert_eq!(v["roster"][0]["telegram_set"], true);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn review_unknown_action_errors() {
    let dir = std::env::temp_dir().join("gsv-ranks-ct-err");
    let _ = std::fs::create_dir_all(&dir);
    let err = ranks::wire_post(
        std::path::Path::new("."),
        &dir,
        &json!({"action": "explode"}),
    )
    .unwrap_err();
    assert!(err.contains("unknown action"), "{err}");
    let _ = std::fs::remove_dir_all(&dir);
}
