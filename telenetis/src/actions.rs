//! Mini App board actions (band 218, plan P4).
//!
//! The browser board shows a Claim / Done / Error button per ticket. Those
//! buttons are thin glue; the *semantics* live here so they are exercised in
//! Rust: the action vocabulary, the JSON body parsing, the i18n labels and the
//! GSV forwarding path each action maps to. The HTTP surface in `ui::mod`
//! verifies the Telegram `initData` handshake (band 214) before forwarding the
//! action server-side to GSV (`/api/tickets/{claim,done,error}`), so an
//! anonymous caller on a public tunnel cannot mutate the board.

use serde_json::{json, Value};

/// The three board actions a Mini App user can take on a ticket. They mirror
/// the three GSV ticket verbs: `claim` (start working), `done` (finish ok),
/// `error` (finish failed).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoardAction {
    Claim,
    Done,
    Error,
}

impl BoardAction {
    /// Map an HTTP verb string to an action. Unknown/blank -> `None`.
    pub fn parse(raw: &str) -> Option<BoardAction> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "claim" => Some(BoardAction::Claim),
            "done" => Some(BoardAction::Done),
            "error" => Some(BoardAction::Error),
            _ => None,
        }
    }

    /// Canonical verb (used as the HTTP route segment and the `url` marker).
    pub fn as_str(self) -> &'static str {
        match self {
            BoardAction::Claim => "claim",
            BoardAction::Done => "done",
            BoardAction::Error => "error",
        }
    }

    /// The GSV endpoint this action forwards to.
    pub fn gsv_path(self) -> &'static str {
        match self {
            BoardAction::Claim => "/api/tickets/claim",
            BoardAction::Done => "/api/tickets/done",
            BoardAction::Error => "/api/tickets/error",
        }
    }

    /// i18n key for the short button label (en/uk/ru in `ui::miniapp`).
    pub fn label_key(self) -> &'static str {
        match self {
            BoardAction::Claim => "action.claim",
            BoardAction::Done => "action.done",
            BoardAction::Error => "action.error",
        }
    }

    /// i18n key for the in-progress / busy button text while forwarding.
    pub fn busy_key(self) -> &'static str {
        match self {
            BoardAction::Claim => "action.claiming",
            BoardAction::Done => "action.doing",
            BoardAction::Error => "action.erroring",
        }
    }

    /// i18n key for the success toast after a forward.
    pub fn ok_key(self) -> &'static str {
        match self {
            BoardAction::Claim => "action.claimed",
            BoardAction::Done => "action.done_ok",
            BoardAction::Error => "action.error_ok",
        }
    }

    /// All actions — used to render the header/button set and in tests.
    pub const ALL: [BoardAction; 3] = [BoardAction::Claim, BoardAction::Done, BoardAction::Error];
}

/// Parsed body of a board-action POST: `{id, note?}`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoardActionBody {
    pub id: String,
    pub note: Option<String>,
}

/// Parse the JSON body of a board-action POST. A non-empty `id` is required.
pub fn parse_body(v: &Value) -> Result<BoardActionBody, String> {
    let id = match v.get("id").and_then(Value::as_str) {
        Some(id) if !id.trim().is_empty() => id.trim().to_string(),
        _ => return Err("missing ticket id".to_string()),
    };
    let note = v
        .get("note")
        .and_then(Value::as_str)
        .map(ToString::to_string);
    Ok(BoardActionBody { id, note })
}

/// Canonical JSON error shape shared by the board action HTTP surface.
pub fn err_json(message: &str) -> Value {
    json!({ "ok": false, "error": message })
}

/// The JSON body forwarded to GSV for an action.
pub fn forward_body(body: &BoardActionBody) -> Value {
    json!({
        "id": body.id,
        "note": body.note,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn verbs_map_to_actions() {
        assert_eq!(BoardAction::parse("claim"), Some(BoardAction::Claim));
        assert_eq!(BoardAction::parse("DONE"), Some(BoardAction::Done));
        assert_eq!(BoardAction::parse(" error "), Some(BoardAction::Error));
        assert_eq!(BoardAction::parse(""), None);
        assert_eq!(BoardAction::parse("reclaim"), None);
        assert_eq!(BoardAction::parse("garbage"), None);
    }

    #[test]
    fn action_verb_labels_and_paths_are_consistent() {
        for action in BoardAction::ALL {
            assert_eq!(BoardAction::parse(action.as_str()), Some(action));
            assert!(action.gsv_path().starts_with("/api/tickets/"));
            assert!(action.gsv_path().ends_with(action.as_str()));
            assert!(
                !crate::ui::miniapp::t(action.label_key(), crate::ui::miniapp::Lang::En).is_empty()
            );
            assert!(
                !crate::ui::miniapp::t(action.busy_key(), crate::ui::miniapp::Lang::En).is_empty()
            );
            assert!(
                !crate::ui::miniapp::t(action.ok_key(), crate::ui::miniapp::Lang::En).is_empty()
            );
        }
    }

    #[test]
    fn parse_body_requires_non_empty_id() {
        let ok = parse_body(&json!({"id": "T-1", "note": "ship it"})).unwrap();
        assert_eq!(ok.id, "T-1");
        assert_eq!(ok.note.as_deref(), Some("ship it"));
    }

    #[test]
    fn parse_body_accepts_optional_note() {
        let no_note = parse_body(&json!({"id": "T-2"})).unwrap();
        assert_eq!(no_note.id, "T-2");
        assert!(no_note.note.is_none());
    }

    #[test]
    fn parse_body_rejects_missing_or_blank_id() {
        assert!(parse_body(&json!({})).is_err());
        assert!(parse_body(&json!({"id": ""})).is_err());
        assert!(parse_body(&json!({"id": "   "})).is_err());
        assert!(parse_body(&json!({"id": 42})).is_err());
    }

    #[test]
    fn parse_body_trims_id() {
        let body = parse_body(&json!({"id": "  T-9  "})).unwrap();
        assert_eq!(body.id, "T-9");
    }

    #[test]
    fn err_json_has_canonical_shape() {
        let v = err_json("boom");
        assert_eq!(v["ok"], false);
        assert_eq!(v["error"], "boom");
        assert_eq!(v.as_object().unwrap().len(), 2);
    }

    #[test]
    fn forward_body_round_trips() {
        let body = BoardActionBody {
            id: "T-1".to_string(),
            note: Some("n".to_string()),
        };
        let fwd = forward_body(&body);
        assert_eq!(fwd["id"], "T-1");
        assert_eq!(fwd["note"], "n");
        let body = BoardActionBody {
            id: "T-2".to_string(),
            note: None,
        };
        let fwd = forward_body(&body);
        assert_eq!(fwd["id"], "T-2");
        assert!(fwd["note"].is_null());
    }
}
