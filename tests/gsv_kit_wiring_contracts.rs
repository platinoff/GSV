//! Kit-wiring band contracts (Kit-Eco 3, owner 2026-10-15).
//!
//! Every registered, non-nested plugin tree discovered on THIS machine must
//! load the GSV kit from its root `opencode.json` (global-first instructions,
//! kit skills path, hub MCP stdio). On CI the sibling trees simply do not
//! exist, so the loop is honestly empty — same keep-what-is-on-disk pattern
//! as the grid ring.

use gsv::boxes::products;
use std::path::{Path, PathBuf};

fn kit_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn every_registered_plugin_window_is_wired() {
    let rows = products::discover(&kit_root());
    let mut checked = 0usize;
    for row in rows {
        if row.role != "plugin" || !row.registered || row.nested {
            continue;
        }
        let path = Path::new(&row.path);
        if !path.exists() {
            continue;
        }
        let status = products::wiring_status(path);
        assert_eq!(
            status, "ok",
            "plugin {} ({}) must carry the kit wiring, got {status}",
            row.id, row.path
        );
        checked += 1;
    }
    if checked == 0 {
        eprintln!("kit wiring: no sibling plugin trees on this host (CI) — nothing to audit");
    } else {
        println!("kit wiring: {checked} plugin windows ok");
    }
}

#[test]
fn wiring_status_reports_missing_and_drift() {
    let dir = kit_root().join("target/wiring_audit");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("mkdir");

    assert_eq!(products::wiring_status(&dir), "missing");

    std::fs::write(dir.join("opencode.json"), "{ not json").unwrap();
    let drift = products::wiring_status(&dir);
    assert!(drift.starts_with("drift:"), "{drift}");
    assert!(drift.contains("unparseable"), "{drift}");

    std::fs::write(dir.join("opencode.json"), "{}").unwrap();
    let drift = products::wiring_status(&dir);
    assert!(drift.contains("instructions"), "{drift}");
    assert!(drift.contains("skills.paths"), "{drift}");
    assert!(drift.contains("mcp.gsv_mcp_openbot"), "{drift}");

    std::fs::write(
        dir.join("opencode.json"),
        r#"{"instructions":["S:/rust/GSV/AGENTS.md","AGENTS.md"],"skills":{"paths":["S:/rust/GSV/.agents/skills"]},"mcp":{"gsv_mcp_openbot":{"type":"local","command":["S:/rust/GSV/target/live/gsv-mcp.exe","--repo-root","S:/rust/GSV"],"cwd":"S:/rust/GSV"}}}"#,
    )
    .unwrap();
    assert_eq!(products::wiring_status(&dir), "ok");

    let _ = std::fs::remove_dir_all(&dir);
}
