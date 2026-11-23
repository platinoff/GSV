//! GASV-G26: read-only ecosystem 1.100 steady-state scanner.
//!
//! Composes facts already exposed elsewhere (`products`, `FUNCTION_MANAGEMENT.md`,
//! vision manifest) into a single JSON report. Never mutates any file or repo.

use serde_json::{json, Value};
use std::path::Path;

/// Accepted toolchain channels for the 1.100 ecosystem: environment-pinned
/// GNU triple and host-agnostic stable (both float to 1.100.0).
fn pin_ok(pin: &str) -> bool {
    let p = pin.trim().trim_matches('"');
    p == "stable" || p == "stable-x86_64-pc-windows-gnu"
}

fn read_pin(root: &Path) -> String {
    std::fs::read_to_string(root.join("rust-toolchain.toml"))
        .ok()
        .and_then(|s| {
            s.lines().find_map(|l| {
                let t = l.trim();
                t.strip_prefix("channel")
                    .and_then(|r| r.trim().strip_prefix('='))
                    .map(|v| v.trim().trim_matches('"').to_string())
            })
        })
        .unwrap_or_default()
}

fn fm_rows(root: &Path) -> usize {
    let text =
        std::fs::read_to_string(root.join("docs/gsv/FUNCTION_MANAGEMENT.md")).unwrap_or_default();
    text.lines()
        .filter(|l| {
            let t = l.trim();
            t.starts_with("| ")
                && !t.starts_with("| #")
                && !t.starts_with("|---")
                && !t.starts_with("| --")
        })
        .count()
}

/// Build the ecosystem report. Read-only; safe to call from a running hub.
pub fn report(root: &Path) -> Value {
    let pin = read_pin(root);
    let ok_pin = pin_ok(&pin);
    let rows = fm_rows(root);
    let manifest = std::fs::read_to_string(root.join("docs/vision/manifest.json"))
        .ok()
        .and_then(|s| serde_json::from_str::<Value>(&s).ok())
        .unwrap_or_else(|| json!({}));
    let rev = manifest
        .get("revision")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let next = manifest
        .get("next")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let products = crate::boxes::products::discover(root);
    let registered = products.iter().filter(|p| p.registered).count();
    // Pin is the hard gate; FM row count is reported separately (fixtures may
    // be smaller than the live canon's 46 rows).
    let ok = ok_pin;
    json!({
        "toolchain": {
            "pin": pin,
            "ok_1_100": ok_pin,
            "expected": "stable 1.100.0 (stable-x86_64-pc-windows-gnu or host-agnostic stable)"
        },
        "products": {
            "discovered": products.len(),
            "registered": registered,
            "host": products.iter().find(|p| p.role == "host").map(|p| &p.id).cloned().unwrap_or_default()
        },
        "fm": { "row_count": rows, "ok": rows >= 40 },
        "vision": { "rev": rev, "next": next },
        "gates": {
            "note": "advisory — run cargo xtask rules-check / fm-xlink / record-rust for authoritative gates"
        },
        "ok": ok
    })
}
