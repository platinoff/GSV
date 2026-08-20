//! Dev-loop benches for GSV boxes (no Criterion crate — std Instant).
//!
//! `cargo bench --bench gsv_dev`

use std::time::Instant;

use gsv::boxes::mds;
use gsv::boxes::telegram;
use gsv::boxes::tickets::{self, ClaimedBy, Presence, TicketMode};
use gsv::boxes::xtask;

fn who(actor: &str) -> Presence {
    Presence {
        actor: actor.into(),
        ide: "cursor".into(),
        model: "grok-4.6".into(),
        agent: "worker".into(),
        seen_unix: 1,
        ..Default::default()
    }
}

fn claimed(actor: &str) -> ClaimedBy {
    ClaimedBy {
        actor: actor.into(),
        ide: "cursor".into(),
        model: "grok-4.6".into(),
        agent: "worker".into(),
    }
}

fn main() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let online: Vec<Presence> = ["alpha", "beta", "gamma", "delta"]
        .into_iter()
        .map(who)
        .collect();

    let kit = std::env::temp_dir().join(format!(
        "gsv-tickets-bench-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    let _ = std::fs::create_dir_all(kit.join("docs/gsv"));
    let _ = std::fs::create_dir_all(kit.join("data"));
    gsv::boxes::settings::save(
        &kit.join("data"),
        &gsv::boxes::settings::SettingsFile {
            workflows: gsv::boxes::settings::Workflows {
                enabled: vec![
                    "ticket-claim".into(),
                    "ticket-squad".into(),
                    "telegram-relay".into(),
                ],
            },
            tickets: gsv::boxes::settings::TicketsSettings {
                mode: "squad".into(),
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .expect("settings");
    std::fs::write(
        tickets::scenarios_path(&kit),
        r#"{
          "scenarios": [{
            "id": "memory-disk-speed",
            "title": "MDS",
            "body": "band",
            "workflow": "ticket-claim",
            "product": "gsv",
            "tickets": [
              {"title": "MDS: scaffold", "body": "a"},
              {"title": "MDS: memory", "body": "b"},
              {"title": "MDS: disk", "body": "c"}
            ]
          }, {
            "id": "abrakadabra-session",
            "title": "session",
            "body": "bench",
            "workflow": "ticket-claim",
            "product": "gsv",
            "tickets": [
              {"title": "Session: S0 disk", "body": "a"},
              {"title": "Session: warnings-first", "body": "b"},
              {"title": "Session: close", "body": "c"}
            ]
          }]
        }"#,
    )
    .expect("scenarios");

    for (name, n) in [
        ("products_tsv", 8usize),
        ("pick_assignee_solo", 10_000usize),
        ("pick_assignee_squad", 10_000usize),
        ("tickets_create_claim_done", 64usize),
        ("tickets_list", 64usize),
        ("telegram_parse_ticket", 10_000usize),
        ("telegram_classify_inbound", 10_000usize),
        ("telegram_extract_envelope", 10_000usize),
        ("scenario_band_create", 16usize),
        ("solo_walk_mds", 8usize),
        ("mds_report", 8usize),
        ("telegram_enqueue_sync", 1_000usize),
        ("hook_parse_phrase", 10_000usize),
        ("hook_roadmap_band", 16usize),
        ("session_walk_abrakadabra", 4usize),
        ("tickets_next_action", 10_000usize),
    ] {
        let start = Instant::now();
        for i in 0..n {
            match name {
                "products_tsv" => {
                    let _ = xtask::products_tsv(&root);
                }
                "pick_assignee_solo" => {
                    let _ = tickets::pick_assignee(TicketMode::Solo, &online, i as u64);
                }
                "pick_assignee_squad" => {
                    let _ = tickets::pick_assignee(TicketMode::Squad, &online, i as u64);
                }
                "tickets_create_claim_done" => {
                    let t = tickets::create(&kit, "bench", "body", "gsv").expect("create");
                    let _ = tickets::claim(&kit, &kit.join("data"), &t.id, claimed("alpha"))
                        .expect("claim");
                    let _ =
                        tickets::done(&kit, &kit.join("data"), &t.id, claimed("alpha"), "ok", None)
                            .expect("done");
                }
                "tickets_list" => {
                    let _ = tickets::list(&kit);
                }
                "telegram_parse_ticket" => {
                    let _ = telegram::parse_ticket_body("/ticket bench title");
                }
                "telegram_classify_inbound" => {
                    let _ = telegram::classify_inbound("/ticket bench title");
                }
                "telegram_extract_envelope" => {
                    let _ = telegram::extract_envelope(
                        "solo claimed Session: S0 disk\n{\"v\":1,\"kind\":\"sync\",\"from\":\"solo\",\"ticket_id\":\"t-1\",\"body\":\"solo claimed Session: S0 disk\",\"data\":{\"hint\":\"work-ticket\"}}",
                    );
                }
                "scenario_band_create" => {
                    let _ = tickets::create_band_from_scenario(
                        &kit,
                        &kit.join("data"),
                        "memory-disk-speed",
                        "",
                    );
                }
                "solo_walk_mds" => {
                    let _ = tickets::create_band_from_scenario(
                        &kit,
                        &kit.join("data"),
                        "memory-disk-speed",
                        "",
                    );
                    let _ = tickets::solo_walk(
                        &kit,
                        &kit.join("data"),
                        None,
                        claimed("alpha"),
                        "memory-disk-speed",
                    );
                }
                "mds_report" => {
                    let _ = mds::report(&root);
                }
                "telegram_enqueue_sync" => {
                    let _ = telegram::enqueue_sync("solo", "t-bench", "claimed");
                }
                "hook_parse_phrase" => {
                    let _ =
                        tickets::parse_hook_phrase("run mcp bot hook up scenario band 177 walk");
                }
                "hook_roadmap_band" => {
                    let md = "## Спринти (band 177)\n\n| **PH-S2409** | Scope | x — **[ ]** |\n";
                    let _ = tickets::parse_roadmap_bands(md);
                }
                "session_walk_abrakadabra" => {
                    let _ = tickets::time_session_walk(&kit, &kit.join("data"));
                }
                "tickets_next_action" => {
                    let _ = tickets::next_action(
                        &kit,
                        &kit.join("data"),
                        None,
                        &claimed("alpha"),
                        "claim-next",
                        "PH-S2469",
                    );
                }
                "disk_report" => {
                    let _ = xtask::disk_report(&root, false);
                }
                _ => {}
            }
        }
        let ns = start.elapsed().as_nanos() / n as u128;
        println!("gsv_dev {name}: median-ish {ns} ns/iter ({n} runs)");
    }
}
