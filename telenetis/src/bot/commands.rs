use crate::state::AppState;

pub fn parse_command(text: &str) -> Option<String> {
    let cmd = text.split_whitespace().next()?;
    cmd.strip_prefix('/').map(|s| s.to_string())
}

pub fn parse_command_args(text: &str) -> (Option<String>, String) {
    let mut parts = text.splitn(2, |c: char| c.is_whitespace());
    let cmd = parts
        .next()
        .and_then(|s| s.strip_prefix('/'))
        .map(|s| s.to_string());
    let args = parts.next().unwrap_or("").trim().to_string();
    (cmd, args)
}

pub enum Command {
    Start,
    Status,
    Board,
    BoardScenario(String),
    Flows,
    Roles,
    Ranks,
    Scenarios,
    Ticket(String),
    Claim(String),
    Done(String),
    Sync,
    App,
    Help,
    Unknown(String),
}

impl Command {
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s {
            "start" => Self::Start,
            "status" => Self::Status,
            "board" => Self::Board,
            "flows" => Self::Flows,
            "roles" => Self::Roles,
            "ranks" => Self::Ranks,
            "scenarios" => Self::Scenarios,
            "sync" => Self::Sync,
            "app" => Self::App,
            "help" => Self::Help,
            other => Self::Unknown(other.to_string()),
        }
    }

    pub fn from_text(text: &str) -> Self {
        let (cmd, args) = parse_command_args(text);
        let name = cmd.unwrap_or_default();
        match name.as_str() {
            "start" => Self::Start,
            "status" => Self::Status,
            "board" if !args.is_empty() => Self::BoardScenario(args),
            "board" => Self::Board,
            "flows" => Self::Flows,
            "roles" => Self::Roles,
            "ranks" => Self::Ranks,
            "scenarios" => Self::Scenarios,
            "ticket" => Self::Ticket(args),
            "claim" => Self::Claim(args),
            "done" => Self::Done(args),
            "sync" => Self::Sync,
            "app" => Self::App,
            "help" => Self::Help,
            other => Self::Unknown(other.to_string()),
        }
    }
}

pub fn command_response(cmd: &Command) -> String {
    match cmd {
        Command::Start | Command::Help => "Welcome to *Telenetis*!\n\n\
             Telegram dashboard for GSV Godfather channel coordination.\n\n\
             *Commands:*\n\
             /status — Bot + GSV status\n\
             /board — Ticket board\n\
             /board <scenario> — Filter by scenario\n\
             /scenarios — List scenarios\n\
             /flows — Recent bot flows\n\
             /roles — Role management\n\
             /ranks — Worker ranks\n\
             /ticket <id> — View ticket details\n\
             /claim <id> — Claim a ticket\n\
             /done <id> — Mark ticket done\n\
             /sync — Force sync from GSV\n\
             /app — Open Mini App\n\
             /help — This message"
            .to_string(),
        Command::Status => "Fetching status...".to_string(),
        Command::Board => "Fetching ticket board...".to_string(),
        Command::BoardScenario(s) => format!("Fetching board for scenario `{s}`..."),
        Command::Flows => "Opening live flows...".to_string(),
        Command::Roles => "Opening role manager...".to_string(),
        Command::Ranks => "Fetching ranks...".to_string(),
        Command::Scenarios => "Fetching scenarios...".to_string(),
        Command::Ticket(id) => format!("Looking up ticket `{id}`..."),
        Command::Claim(id) => format!("Claiming ticket `{id}`..."),
        Command::Done(id) => format!("Marking ticket `{id}` done..."),
        Command::Sync => "Syncing from GSV...".to_string(),
        Command::App => "Opening Mini App...".to_string(),
        Command::Unknown(cmd) => format!("Unknown command: /{cmd}"),
    }
}

pub async fn handle_command(cmd: &Command, state: &AppState) -> String {
    match cmd {
        Command::Start | Command::Help => command_response(cmd),
        Command::Status => handle_status(state).await,
        Command::Board => handle_board(state, None).await,
        Command::BoardScenario(s) => handle_board(state, Some(s)).await,
        Command::Flows => handle_flows(state).await,
        Command::Roles => handle_roles(state).await,
        Command::Ranks => handle_ranks(state).await,
        Command::Scenarios => handle_scenarios(state).await,
        Command::Ticket(id) => handle_ticket_detail(id, state).await,
        Command::Claim(id) => handle_claim(id, state).await,
        Command::Done(id) => handle_done(id, state).await,
        Command::Sync => handle_sync(state).await,
        Command::App => command_response(cmd),
        Command::Unknown(_) => command_response(cmd),
    }
}

async fn handle_status(state: &AppState) -> String {
    let tickets = state.tickets().await;
    let presence = state.presence_map().await;
    let bus = state.bus_queue().await;
    let online = state.is_online();

    let open = tickets.iter().filter(|t| t.status == "open").count();
    let in_progress = tickets.iter().filter(|t| t.status == "in_progress").count();
    let done = tickets.iter().filter(|t| t.status == "done").count();
    let blocked = tickets.iter().filter(|t| t.status == "blocked").count();

    let workers: Vec<String> = presence
        .values()
        .map(|w| {
            let status_str = match w.status {
                crate::state::WorkerStatus::Ready => "ready",
                crate::state::WorkerStatus::Busy => "busy",
                crate::state::WorkerStatus::Offline => "offline",
            };
            format!("  {} [{}] rank={} {}", w.jail_id, status_str, w.rank, w.ide)
        })
        .collect();

    format!(
        "*Telenetis Status*\n\n\
         Online: {} | Jail: `{}`\n\
         Tickets: {} open / {} in-progress / {} done / {} blocked\n\
         Bus envelopes: {}\n\
         Workers: {}\n{}",
        if online { "yes" } else { "no" },
        state.jail_id(),
        open,
        in_progress,
        done,
        blocked,
        bus.len(),
        presence.len(),
        if workers.is_empty() {
            "  (none)".to_string()
        } else {
            workers.join("\n")
        },
    )
}

async fn handle_board(state: &AppState, scenario_filter: Option<&str>) -> String {
    let tickets = state.tickets().await;
    let filtered: Vec<_> = match scenario_filter {
        Some(s) => tickets
            .iter()
            .filter(|t| t.scenario.as_deref() == Some(s))
            .collect(),
        None => tickets.iter().collect(),
    };
    if filtered.is_empty() {
        return if let Some(s) = scenario_filter {
            format!("No tickets for scenario `{s}`.")
        } else {
            "No tickets on the board.".to_string()
        };
    }

    let header = match scenario_filter {
        Some(s) => format!("*Board — {s}*"),
        None => "*Ticket Board*".to_string(),
    };
    let mut lines: Vec<String> = vec![header, "".to_string()];
    for t in &filtered {
        let status_icon = match t.status.as_str() {
            "open" => "🟢",
            "in_progress" => "🟡",
            "done" => "⬜",
            "blocked" => "🔴",
            _ => "⚪",
        };
        let claimed = t
            .claimed_by
            .as_deref()
            .map(|c| format!(" @{c}"))
            .unwrap_or_default();
        let scenario = t
            .scenario
            .as_deref()
            .map(|s| format!(" [{s}]"))
            .unwrap_or_default();
        lines.push(format!(
            "{} `{}` {} — {}{}{}",
            status_icon, t.id, t.title, t.product, claimed, scenario
        ));
    }

    format!("{}\n\nUse /ticket <id> for details.", lines.join("\n"))
}

async fn handle_scenarios(state: &AppState) -> String {
    let tickets = state.tickets().await;
    let mut scenario_map: std::collections::HashMap<String, Vec<&str>> =
        std::collections::HashMap::new();
    for t in &tickets {
        let scenario = t.scenario.as_deref().unwrap_or("(none)");
        scenario_map
            .entry(scenario.to_string())
            .or_default()
            .push(&t.id);
    }
    if scenario_map.is_empty() {
        return "No tickets — no scenarios.".to_string();
    }

    let mut lines: Vec<String> = vec!["*Scenarios*".to_string(), "".to_string()];
    let mut sorted: Vec<_> = scenario_map.into_iter().collect();
    sorted.sort_by(|a, b| b.1.len().cmp(&a.1.len()));
    for (name, ids) in &sorted {
        lines.push(format!(
            "*{}* — {} tickets: {}",
            name,
            ids.len(),
            ids.join(", ")
        ));
    }

    lines.join("\n")
}

async fn handle_flows(state: &AppState) -> String {
    let flows = state.recent_flows(15).await;
    if flows.is_empty() {
        return "No recent flows.".to_string();
    }

    let mut lines: Vec<String> = vec!["*Recent Flows*".to_string(), "".to_string()];
    for f in &flows {
        let ts = f.ts.format("%H:%M:%S");
        lines.push(format!("`{ts}` [{}] {}", f.jail_id, f.detail));
    }

    lines.join("\n")
}

async fn handle_roles(state: &AppState) -> String {
    let presence = state.presence_map().await;
    if presence.is_empty() {
        return "No workers online.".to_string();
    }

    let mut lines: Vec<String> = vec!["*Workers (Roles)*".to_string(), "".to_string()];
    for w in presence.values() {
        let status_str = match w.status {
            crate::state::WorkerStatus::Ready => "Ready",
            crate::state::WorkerStatus::Busy => "Busy",
            crate::state::WorkerStatus::Offline => "Offline",
        };
        lines.push(format!(
            "*{}* — {} | {} | rank={} | {}",
            w.jail_id, w.agent, w.ide, w.rank, status_str
        ));
    }

    lines.join("\n")
}

async fn handle_ranks(state: &AppState) -> String {
    let presence = state.presence_map().await;
    if presence.is_empty() {
        return "No workers online.".to_string();
    }

    let mut workers: Vec<_> = presence.values().collect();
    workers.sort_by(|a, b| b.rank.cmp(&a.rank));

    let mut lines: Vec<String> = vec!["*Worker Ranks*".to_string(), "".to_string()];
    for (i, w) in workers.iter().enumerate() {
        let medal = match i {
            0 => "🥇",
            1 => "🥈",
            2 => "🥉",
            _ => "  ",
        };
        lines.push(format!(
            "{} L{} — `{}` {} {}",
            medal, w.rank, w.jail_id, w.ide, w.agent
        ));
    }

    lines.join("\n")
}

async fn handle_ticket_detail(id: &str, state: &AppState) -> String {
    let tickets = state.tickets().await;
    match tickets.iter().find(|t| t.id == id) {
        Some(t) => {
            let claimed = t
                .claimed_by
                .as_deref()
                .map(|c| format!("@{c}"))
                .unwrap_or_else(|| "(unclaimed)".to_string());
            let scenario = t
                .scenario
                .as_deref()
                .map(|s| format!("\nScenario: `{s}`"))
                .unwrap_or_default();
            format!(
                "*Ticket {}*\n\n\
                 Title: {}\n\
                 Status: {}\n\
                 Product: {}\n\
                 Claimed by: {}{}\n\
                 \n{}",
                t.id, t.title, t.status, t.product, claimed, scenario, t.body
            )
        }
        None => format!("Ticket `{id}` not found."),
    }
}

async fn handle_claim(id: &str, state: &AppState) -> String {
    let mut tickets = state.tickets().await;
    match tickets.iter_mut().find(|t| t.id == id) {
        Some(t) => {
            if t.status != "open" {
                return format!(
                    "Ticket `{id}` is `{}`, only `open` tickets can be claimed.",
                    t.status
                );
            }
            t.status = "in_progress".to_string();
            t.claimed_by = Some(state.jail_id().to_string());
            state.set_tickets(tickets).await;
            if let Err(e) = crate::gsv::poll::post_bus_envelope(
                state,
                "claim",
                &format!("claimed {id}"),
                Some(id),
            )
            .await
            {
                tracing::warn!("Failed to post claim bus envelope: {e}");
            }
            format!("Ticket `{id}` claimed by `{}`.", state.jail_id())
        }
        None => format!("Ticket `{id}` not found."),
    }
}

async fn handle_done(id: &str, state: &AppState) -> String {
    let mut tickets = state.tickets().await;
    match tickets.iter_mut().find(|t| t.id == id) {
        Some(t) => {
            if t.status != "in_progress" {
                return format!(
                    "Ticket `{id}` is `{}`, only `in_progress` tickets can be marked done.",
                    t.status
                );
            }
            t.status = "done".to_string();
            state.set_tickets(tickets).await;
            if let Err(e) = crate::gsv::poll::post_bus_envelope(
                state,
                "done",
                &format!("completed {id}"),
                Some(id),
            )
            .await
            {
                tracing::warn!("Failed to post done bus envelope: {e}");
            }
            format!("Ticket `{id}` marked done.")
        }
        None => format!("Ticket `{id}` not found."),
    }
}

async fn handle_sync(state: &AppState) -> String {
    let config = state.config();
    let client = crate::gsv::client::GsvClient::new(config);
    match crate::gsv::tickets::sync_tickets(&client, state).await {
        Ok(()) => {
            let tickets = state.tickets().await;
            format!("Synced from GSV — {} tickets on board.", tickets.len())
        }
        Err(e) => format!("Sync failed: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    fn test_config() -> Config {
        Config {
            bot_token: "test".to_string(),
            gsv_url: "http://127.0.0.1:9999".to_string(),
            port: 9800,
            jail_id: "test-jail".to_string(),
            godfather_channel_id: 0,
            webhook_url: None,
        }
    }

    #[test]
    fn parse_start_command() {
        assert_eq!(parse_command("/start"), Some("start".to_string()));
        assert_eq!(parse_command("/board extra"), Some("board".to_string()));
    }

    #[test]
    fn parse_non_command_returns_none() {
        assert_eq!(parse_command("hello"), None);
        assert_eq!(parse_command(""), None);
    }

    #[test]
    fn parse_command_args_extracts_args() {
        let (cmd, args) = parse_command_args("/ticket T-123");
        assert_eq!(cmd, Some("ticket".to_string()));
        assert_eq!(args, "T-123");
    }

    #[test]
    fn parse_command_args_no_args() {
        let (cmd, args) = parse_command_args("/status");
        assert_eq!(cmd, Some("status".to_string()));
        assert_eq!(args, "");
    }

    #[test]
    fn command_from_str_known() {
        assert!(matches!(Command::from_str("start"), Command::Start));
        assert!(matches!(Command::from_str("status"), Command::Status));
        assert!(matches!(Command::from_str("board"), Command::Board));
        assert!(matches!(Command::from_str("flows"), Command::Flows));
        assert!(matches!(Command::from_str("roles"), Command::Roles));
        assert!(matches!(Command::from_str("ranks"), Command::Ranks));
        assert!(matches!(Command::from_str("sync"), Command::Sync));
        assert!(matches!(Command::from_str("app"), Command::App));
        assert!(matches!(Command::from_str("help"), Command::Help));
    }

    #[test]
    fn command_from_text_parses_args() {
        assert!(matches!(Command::from_text("/ticket T-1"), Command::Ticket(a) if a == "T-1"));
        assert!(matches!(Command::from_text("/claim T-2"), Command::Claim(a) if a == "T-2"));
        assert!(matches!(Command::from_text("/done T-3"), Command::Done(a) if a == "T-3"));
    }

    #[test]
    fn command_unknown() {
        match Command::from_str("foobar") {
            Command::Unknown(s) => assert_eq!(s, "foobar"),
            _ => panic!("expected unknown"),
        }
    }

    #[test]
    fn command_response_unknown_contains_cmd() {
        let r = command_response(&Command::Unknown("xyz".to_string()));
        assert!(r.contains("xyz"));
    }

    #[test]
    fn command_response_help_lists_commands() {
        let r = command_response(&Command::Help);
        assert!(r.contains("/status"));
        assert!(r.contains("/board"));
        assert!(r.contains("/ranks"));
        assert!(r.contains("/claim"));
        assert!(r.contains("/done"));
        assert!(r.contains("/sync"));
        assert!(r.contains("/app"));
    }

    #[test]
    fn command_response_ticket_shows_id() {
        let r = command_response(&Command::Ticket("T-5".to_string()));
        assert!(r.contains("T-5"));
    }

    #[tokio::test]
    async fn handle_status_empty_state() {
        let state = crate::state::AppState::new(test_config());
        let resp = handle_command(&Command::Status, &state).await;
        assert!(resp.contains("0 open"));
        assert!(resp.contains("test-jail"));
    }

    #[tokio::test]
    async fn handle_board_empty() {
        let state = crate::state::AppState::new(test_config());
        let resp = handle_command(&Command::Board, &state).await;
        assert!(resp.contains("No tickets"));
    }

    #[tokio::test]
    async fn handle_board_with_tickets() {
        let state = crate::state::AppState::new(test_config());
        state
            .set_tickets(vec![crate::state::TicketRow {
                id: "T-1".to_string(),
                title: "Fix bug".to_string(),
                body: "desc".to_string(),
                status: "open".to_string(),
                product: "gsv".to_string(),
                claimed_by: None,
                scenario: None,
            }])
            .await;
        let resp = handle_command(&Command::Board, &state).await;
        assert!(resp.contains("T-1"));
        assert!(resp.contains("Fix bug"));
    }

    #[tokio::test]
    async fn handle_flows_empty() {
        let state = crate::state::AppState::new(test_config());
        let resp = handle_command(&Command::Flows, &state).await;
        assert!(resp.contains("No recent flows"));
    }

    #[tokio::test]
    async fn handle_ticket_detail_found() {
        let state = crate::state::AppState::new(test_config());
        state
            .set_tickets(vec![crate::state::TicketRow {
                id: "T-9".to_string(),
                title: "Task".to_string(),
                body: "do stuff".to_string(),
                status: "open".to_string(),
                product: "gsv".to_string(),
                claimed_by: None,
                scenario: Some("setup".to_string()),
            }])
            .await;
        let resp = handle_command(&Command::Ticket("T-9".to_string()), &state).await;
        assert!(resp.contains("Task"));
        assert!(resp.contains("setup"));
    }

    #[tokio::test]
    async fn handle_ticket_detail_not_found() {
        let state = crate::state::AppState::new(test_config());
        let resp = handle_command(&Command::Ticket("NOPE".to_string()), &state).await;
        assert!(resp.contains("not found"));
    }

    #[tokio::test]
    async fn handle_claim_open_ticket() {
        let state = crate::state::AppState::new(test_config());
        state
            .set_tickets(vec![crate::state::TicketRow {
                id: "T-1".to_string(),
                title: "Task".to_string(),
                body: String::new(),
                status: "open".to_string(),
                product: "gsv".to_string(),
                claimed_by: None,
                scenario: None,
            }])
            .await;
        let resp = handle_command(&Command::Claim("T-1".to_string()), &state).await;
        assert!(resp.contains("claimed"));
        let tickets = state.tickets().await;
        assert_eq!(tickets[0].status, "in_progress");
    }

    #[tokio::test]
    async fn handle_claim_non_open_rejected() {
        let state = crate::state::AppState::new(test_config());
        state
            .set_tickets(vec![crate::state::TicketRow {
                id: "T-2".to_string(),
                title: "Task".to_string(),
                body: String::new(),
                status: "in_progress".to_string(),
                product: "gsv".to_string(),
                claimed_by: Some("other".to_string()),
                scenario: None,
            }])
            .await;
        let resp = handle_command(&Command::Claim("T-2".to_string()), &state).await;
        assert!(resp.contains("only `open`"));
    }

    #[tokio::test]
    async fn handle_done_in_progress_ticket() {
        let state = crate::state::AppState::new(test_config());
        state
            .set_tickets(vec![crate::state::TicketRow {
                id: "T-3".to_string(),
                title: "Task".to_string(),
                body: String::new(),
                status: "in_progress".to_string(),
                product: "gsv".to_string(),
                claimed_by: Some("test-jail".to_string()),
                scenario: None,
            }])
            .await;
        let resp = handle_command(&Command::Done("T-3".to_string()), &state).await;
        assert!(resp.contains("done"));
        let tickets = state.tickets().await;
        assert_eq!(tickets[0].status, "done");
    }

    #[tokio::test]
    async fn handle_done_non_progress_rejected() {
        let state = crate::state::AppState::new(test_config());
        state
            .set_tickets(vec![crate::state::TicketRow {
                id: "T-4".to_string(),
                title: "Task".to_string(),
                body: String::new(),
                status: "open".to_string(),
                product: "gsv".to_string(),
                claimed_by: None,
                scenario: None,
            }])
            .await;
        let resp = handle_command(&Command::Done("T-4".to_string()), &state).await;
        assert!(resp.contains("only `in_progress`"));
    }

    #[tokio::test]
    async fn handle_ranks_sorted() {
        let state = crate::state::AppState::new(test_config());
        state
            .update_presence(crate::state::WorkerPresence {
                jail_id: "jail-low".to_string(),
                actor: "a".to_string(),
                ide: "cursor".to_string(),
                model: "m".to_string(),
                agent: "orchestrator".to_string(),
                rank: 3,
                status: crate::state::WorkerStatus::Ready,
                last_heartbeat: chrono::Utc::now(),
                timezone: "UTC".to_string(),
            })
            .await;
        state
            .update_presence(crate::state::WorkerPresence {
                jail_id: "jail-high".to_string(),
                actor: "b".to_string(),
                ide: "opencode".to_string(),
                model: "m".to_string(),
                agent: "coder".to_string(),
                rank: 8,
                status: crate::state::WorkerStatus::Busy,
                last_heartbeat: chrono::Utc::now(),
                timezone: "UTC".to_string(),
            })
            .await;
        let resp = handle_command(&Command::Ranks, &state).await;
        assert!(resp.contains("jail-high"));
        assert!(resp.contains("jail-low"));
        let high_pos = resp.find("jail-high").unwrap();
        let low_pos = resp.find("jail-low").unwrap();
        assert!(high_pos < low_pos);
    }

    #[tokio::test]
    async fn handle_board_filters_by_scenario() {
        let state = crate::state::AppState::new(test_config());
        state
            .set_tickets(vec![
                crate::state::TicketRow {
                    id: "T-1".to_string(),
                    title: "A".to_string(),
                    body: String::new(),
                    status: "open".to_string(),
                    product: "gsv".to_string(),
                    claimed_by: None,
                    scenario: Some("setup".to_string()),
                },
                crate::state::TicketRow {
                    id: "T-2".to_string(),
                    title: "B".to_string(),
                    body: String::new(),
                    status: "open".to_string(),
                    product: "gsv".to_string(),
                    claimed_by: None,
                    scenario: Some("drain".to_string()),
                },
            ])
            .await;
        let resp = handle_command(&Command::BoardScenario("setup".to_string()), &state).await;
        assert!(resp.contains("T-1"));
        assert!(!resp.contains("T-2"));
        assert!(resp.contains("setup"));
    }

    #[tokio::test]
    async fn handle_board_filters_empty_scenario() {
        let state = crate::state::AppState::new(test_config());
        state
            .set_tickets(vec![crate::state::TicketRow {
                id: "T-1".to_string(),
                title: "A".to_string(),
                body: String::new(),
                status: "open".to_string(),
                product: "gsv".to_string(),
                claimed_by: None,
                scenario: Some("setup".to_string()),
            }])
            .await;
        let resp = handle_command(&Command::BoardScenario("nope".to_string()), &state).await;
        assert!(resp.contains("No tickets for scenario"));
    }

    #[tokio::test]
    async fn handle_scenarios_groups_tickets() {
        let state = crate::state::AppState::new(test_config());
        state
            .set_tickets(vec![
                crate::state::TicketRow {
                    id: "T-1".to_string(),
                    title: "A".to_string(),
                    body: String::new(),
                    status: "open".to_string(),
                    product: "gsv".to_string(),
                    claimed_by: None,
                    scenario: Some("setup".to_string()),
                },
                crate::state::TicketRow {
                    id: "T-2".to_string(),
                    title: "B".to_string(),
                    body: String::new(),
                    status: "open".to_string(),
                    product: "gsv".to_string(),
                    claimed_by: None,
                    scenario: Some("setup".to_string()),
                },
                crate::state::TicketRow {
                    id: "T-3".to_string(),
                    title: "C".to_string(),
                    body: String::new(),
                    status: "open".to_string(),
                    product: "gsv".to_string(),
                    claimed_by: None,
                    scenario: None,
                },
            ])
            .await;
        let resp = handle_command(&Command::Scenarios, &state).await;
        assert!(resp.contains("setup"));
        assert!(resp.contains("2 tickets"));
        assert!(resp.contains("(none)"));
    }

    #[tokio::test]
    async fn handle_scenarios_empty() {
        let state = crate::state::AppState::new(test_config());
        let resp = handle_command(&Command::Scenarios, &state).await;
        assert!(resp.contains("No tickets"));
    }

    #[test]
    fn command_from_text_board_with_scenario() {
        assert!(matches!(
            Command::from_text("/board setup"),
            Command::BoardScenario(a) if a == "setup"
        ));
    }

    #[test]
    fn command_from_text_scenarios() {
        assert!(matches!(
            Command::from_text("/scenarios"),
            Command::Scenarios
        ));
    }
}
