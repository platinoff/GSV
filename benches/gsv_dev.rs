//! Dev-loop benches for GSV boxes (no Criterion crate — std Instant).
//!
//! `cargo bench --bench gsv_dev`

use std::time::Instant;

use gsv::boxes::tickets::{self, ClaimedBy, Presence, TicketMode};
use gsv::boxes::xtask;

fn who(actor: &str) -> Presence {
    Presence {
        actor: actor.into(),
        ide: "cursor".into(),
        model: "grok-4.6".into(),
        agent: "worker".into(),
        seen_unix: 1,
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
                enabled: vec!["ticket-claim".into(), "ticket-squad".into()],
            },
            tickets: gsv::boxes::settings::TicketsSettings {
                mode: "squad".into(),
            },
            ..Default::default()
        },
    )
    .expect("settings");

    for (name, n) in [
        ("products_tsv", 8usize),
        ("pick_assignee_solo", 10_000usize),
        ("pick_assignee_squad", 10_000usize),
        ("tickets_create_claim_done", 64usize),
        ("tickets_list", 64usize),
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
