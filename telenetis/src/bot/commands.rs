use crate::state::AppState;

pub fn parse_command(text: &str) -> Option<String> {
    let cmd = text.split_whitespace().next()?;
    let stripped = cmd.strip_prefix('/')?;
    if let Some(idx) = stripped.find('@') {
        Some(stripped[..idx].to_string())
    } else {
        Some(stripped.to_string())
    }
}

pub fn parse_command_args(text: &str) -> (Option<String>, String) {
    let mut parts = text.splitn(2, |c: char| c.is_whitespace());
    let raw_cmd = parts
        .next()
        .and_then(|s| s.strip_prefix('/'))
        .map(|s| s.to_string());
    let cmd = raw_cmd.map(|c| {
        if let Some(idx) = c.find('@') {
            c[..idx].to_string()
        } else {
            c
        }
    });
    let args = parts.next().unwrap_or("").trim().to_string();
    (cmd, args)
}

pub enum Command {
    Start,
    Status,
    Board,
    Worker(Option<String>),
    Vm(Option<String>),
    Chat(String),
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
    Probe,
    Tensor,
    Tunnel,
    Reconnect,
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
            "worker" => Self::Worker(None),
            "chat" => Self::Chat(String::new()),
            "vm" => Self::Vm(None),
            "flows" => Self::Flows,
            "roles" => Self::Roles,
            "ranks" => Self::Ranks,
            "scenarios" => Self::Scenarios,
            "sync" => Self::Sync,
            "app" => Self::App,
            "probe" => Self::Probe,
            "tensor" => Self::Tensor,
            "tunnel" => Self::Tunnel,
            "reconnect" => Self::Reconnect,
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
            "worker" if !args.is_empty() => Self::Worker(Some(args)),
            "worker" => Self::Worker(None),
            "vm" if !args.is_empty() => Self::Vm(Some(args)),
            "vm" => Self::Vm(None),
            "chat" => Self::Chat(args),
            "sync" => Self::Sync,
            "app" => Self::App,
            "probe" => Self::Probe,
            "tensor" => Self::Tensor,
            "tunnel" => Self::Tunnel,
            "reconnect" => Self::Reconnect,
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
              /worker — Edge workers (poolAI telegram bindings)\n\
              /worker <user|peer> — One worker detail\n\
              /vm — My VMs (poolAI instances behind edge peers)\n\
              /vm <user|peer> — One VM detail\n\
              /chat [fast|deep] <text> — Ask llama (answer lands in Mini App chat)\n\
             /sync — Force sync from GSV\n\
             /app — Open Mini App\n\
              /probe — WebGPU adapter probe (phone GPU → hub profile)\n\
              /tensor — Browser tensor worker (phone LLM, tasks from poolAI)\n\
             /tunnel — Show / refresh public tunnel URL\n\
             /reconnect — Reconnect bot to channel\n\
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
        Command::Worker(None) => "Fetching edge workers...".to_string(),
        Command::Worker(Some(id)) => format!("Looking up edge worker `{id}`..."),
        Command::Vm(None) => "Fetching virtual workers...".to_string(),
        Command::Vm(Some(id)) => format!("Looking up virtual worker `{id}`..."),
        Command::Chat(prompt) if prompt.trim().is_empty() => {
            "Usage: /chat <text> — asks llama, answer in Mini App chat.".to_string()
        }
        Command::Chat(_) => "Queueing chat...".to_string(),
        Command::Sync => "Syncing from GSV...".to_string(),
        Command::App => "Opening Mini App...".to_string(),
        Command::Probe => "Opening WebGPU probe...".to_string(),
        Command::Tensor => "Opening tensor worker...".to_string(),
        Command::Tunnel => "Tunnel".to_string(),
        Command::Reconnect => "Reconnect".to_string(),
        Command::Unknown(cmd) => format!("Unknown command: /{cmd}"),
    }
}

pub async fn handle_command(cmd: &Command, state: &AppState) -> String {
    handle_command_from(cmd, state, None).await
}

/// Sender-aware entry: `sender_id` is the Telegram numeric user id for
/// commands that act as the sender (`/chat`); `None` elsewhere.
pub async fn handle_command_from(
    cmd: &Command,
    state: &AppState,
    sender_id: Option<&str>,
) -> String {
    if let Command::Chat(prompt) = cmd {
        return handle_chat(prompt, sender_id, state).await;
    }
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
        Command::Worker(id) => handle_worker(id.as_deref(), state).await,
        Command::Vm(id) => handle_vm(id.as_deref(), state).await,
        Command::Chat(_) => command_response(cmd),
        Command::Sync => handle_sync(state).await,
        Command::App => command_response(cmd),
        Command::Probe => command_response(cmd),
        Command::Tensor => command_response(cmd),
        Command::Tunnel => handle_tunnel(state).await,
        Command::Reconnect => handle_reconnect(state).await,
        Command::Unknown(_) => command_response(cmd),
    }
}

/// Refresh the public tunnel URL (ensure ngrok is up) and report it.
async fn handle_tunnel(state: &AppState) -> String {
    let config = state.config().clone();
    match crate::tunnel::ensure_public_url(&config).await {
        Ok(url) => {
            state.set_tunnel_url(url.clone()).await;
            let bot = crate::bot::telegram::TelegramBot::new(&config);
            if let Err(e) = bot.set_chat_menu_button(&url).await {
                tracing::warn!("Failed to refresh Telegram menu button: {}", e);
            }
            format!(
                "🕳️ *Tunnel* is live.\nPublic URL: `{}`\n\nThis is the address the `/app` Mini App button opens from phones.",
                url
            )
        }
        Err(e) => {
            let lan = crate::edge::local_lan_ip()
                .map(|ip| format!("http://{ip}:{}", state.config().port))
                .unwrap_or_default();
            format!(
                "⚠️ *Tunnel unavailable*: `{e}`\n\n\
                 On the same Wi-Fi use the LAN URL: `{lan}`\n\
                 Outside the house: install ngrok (`winget install Ngrok.Ngrok` + authtoken) \
                 or set `TELENETIS_PUBLIC_URL` in `.env` to pin a fixed host."
            )
        }
    }
}

/// Reconnect the bot flow: re-register self presence and force a GSV re-sync.
async fn handle_reconnect(state: &AppState) -> String {
    crate::state::register_self_presence(state);
    let sync = handle_sync(state).await;
    format!(
        "🔄 *Reconnected.*\n\nWorker presence re-registered for `{}`.\n\n{}",
        state.jail_id(),
        sync
    )
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

    let lan = crate::edge::local_lan_ip().unwrap_or_else(|| "?".to_string());
    let public = state.tunnel_url().await;
    format!(
        "*Telenetis Status*\n\n\
         Online: {} | Jail: `{}`\n\
         Tickets: {} open / {} in-progress / {} done / {} blocked\n\
         Bus envelopes: {}\n\
         Workers: {}\n{}\n\
         Reach: LAN `http://{}:{}`{}",
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
        lan,
        state.config().port,
        match public {
            Some(u) => format!(" · public `{u}`"),
            None => String::new(),
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

    // T2.2: hint only — suggest the next move, never auto-claim (ownership
    // stays with the owner ticket t-1789606667836911200).
    if let Some((action, id)) =
        crate::actions::next_hint(&filtered.iter().map(|t| (*t).clone()).collect::<Vec<_>>())
    {
        lines.push(String::new());
        lines.push(format!("Next: {} `{}`.", action.as_str(), id));
    }

    format!("{}\n\nUse /ticket <id> for details.", lines.join("\n"))
}

/// Edge workers: poolAI telegram bindings + seats + per-peer task status.
/// `/worker` lists, `/worker <telegram_user_id|peer_id>` shows one.
async fn handle_worker(id: Option<&str>, state: &AppState) -> String {
    use crate::edge::{apply_task_status, assemble_views, find_worker, render_workers, PoolClient};

    let pool = PoolClient::new(state.config());
    let bindings = match pool.bindings().await {
        Ok(b) => b,
        Err(e) => return format!("⚠️ *Edge workers unavailable:* `{e}`"),
    };
    let mut views = assemble_views(&bindings);
    for v in &mut views {
        if let Ok(st) = pool.task_status(&v.peer_id).await {
            apply_task_status(v, &st);
        }
    }
    let seats = pool.seats().await.ok();
    match id {
        None => render_workers(&views, seats.as_ref()),
        Some(want) => match find_worker(&views, want) {
            Some(v) => render_workers(std::slice::from_ref(v), seats.as_ref()),
            None => format!("No edge worker `{want}`. Use /worker to list."),
        },
    }
}

/// Virtual workers: poolAI VM instances behind edge peers (`{peer}-vm`).
/// `/vm` lists, `/vm <telegram_user_id|peer_id>` shows one.
async fn handle_vm(id: Option<&str>, state: &AppState) -> String {
    use crate::edge::{render_vms, vm_views, PoolClient};

    let pool = PoolClient::new(state.config());
    let rows = match vm_views(&pool, id).await {
        Ok(r) => r,
        Err(e) => return format!("⚠️ *Virtual workers unavailable:* `{e}`"),
    };
    if let Some(want) = id {
        if rows.is_empty() {
            return format!("No virtual worker `{want}`. Use /vm to list.");
        }
    }
    render_vms(&rows)
}

/// Ask llama through poolAI services only (no Termux, no direct calls).
/// Enqueues a `llama_chat` task for the sender's bound peer; the answer
/// lands in the Mini App chat screen (polled by task id).
async fn handle_chat(prompt: &str, sender_id: Option<&str>, state: &AppState) -> String {
    use crate::edge::PoolClient;

    let prompt = prompt.trim();
    if prompt.is_empty() {
        return "Usage: /chat <text> — asks llama, answer in Mini App chat.".to_string();
    }
    let Some(sender) = sender_id.map(str::trim).filter(|s| !s.is_empty()) else {
        return "Send /chat from your Telegram account so I know whose peer to ask.".to_string();
    };
    let pool = PoolClient::new(state.config());
    let peer = match pool.peer_for_user(sender).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            return "No edge peer bound to you yet — /start on your phone, then bind.".to_string()
        }
        Err(e) => return format!("⚠️ *Chat unavailable:* `{e}`"),
    };
    // Optional leading tier: `/chat fast ...` (else deep).
    let (tier, text) = match prompt
        .split_once(char::is_whitespace)
        .map(|(a, b)| (a.trim().to_ascii_lowercase(), b.trim()))
    {
        Some((t, rest)) if t == "fast" || t == "deep" => (t, rest.to_string()),
        _ => ("deep".to_string(), prompt.to_string()),
    };
    if text.is_empty() {
        return "Usage: /chat [fast|deep] <text> — asks llama, answer in Mini App chat."
            .to_string();
    }
    match pool.enqueue_chat(&peer, &text, 64, &tier).await {
        Ok(id) => {
            format!("Queued for `{peer}` (task `{id}`).\nThe answer appears in Mini App chat.")
        }
        Err(e) => format!("⚠️ *Chat unavailable:* `{e}`"),
    }
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
    sorted.sort_by_key(|entry| std::cmp::Reverse(entry.1.len()));
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
    let assigned = state.list_roles().await;

    let mut lines: Vec<String> = Vec::new();
    let mut sections: Vec<String> = Vec::new();

    if !assigned.is_empty() {
        sections.push("*Assigned Roles*".to_string());
        sections.push("".to_string());
        for r in &assigned {
            sections.push(format!("`{}` — {}", r.jail_id, r.role.as_str()));
        }
        sections.push("".to_string());
    } else {
        sections.push("No roles assigned yet.".to_string());
        sections.push("".to_string());
    }

    sections.push("*Workers (Roles)*".to_string());
    if presence.is_empty() {
        sections.push("".to_string());
        sections.push("No workers online.".to_string());
    } else {
        sections.push("".to_string());
        for w in presence.values() {
            let status_str = match w.status {
                crate::state::WorkerStatus::Ready => "Ready",
                crate::state::WorkerStatus::Busy => "Busy",
                crate::state::WorkerStatus::Offline => "Offline",
            };
            sections.push(format!(
                "*{}* — {} | {} | rank={} | {}",
                w.jail_id, w.agent, w.ide, w.rank, status_str
            ));
        }
    }

    lines.push("*Roles*".to_string());
    lines.push("".to_string());
    lines.extend(sections);
    lines.retain(|l| !l.is_empty());
    lines.join("\n")
}

async fn handle_ranks(state: &AppState) -> String {
    let presence = state.presence_map().await;
    if presence.is_empty() {
        return "No workers online.".to_string();
    }

    let mut workers: Vec<_> = presence.values().collect();
    workers.sort_by_key(|w| std::cmp::Reverse(w.rank));

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
            poolai_url: "http://127.0.0.1:8091".to_string(),
            port: 9800,
            jail_id: "test-jail".to_string(),
            godfather_channel_id: 0,
            webhook_url: None,
            webhook_secret: None,
            public_url: None,
            tunnel_enabled: false,
            ngrok_bin: None,
        }
    }

    #[test]
    fn parse_start_command() {
        assert_eq!(parse_command("/start"), Some("start".to_string()));
        assert_eq!(parse_command("/status@gsv_bot"), Some("status".to_string()));
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
        assert!(matches!(Command::from_str("probe"), Command::Probe));
        assert!(matches!(Command::from_text("/probe"), Command::Probe));
        assert!(matches!(Command::from_str("tensor"), Command::Tensor));
        assert!(matches!(Command::from_text("/tensor"), Command::Tensor));
        assert!(matches!(Command::from_str("tunnel"), Command::Tunnel));
        assert!(matches!(Command::from_str("reconnect"), Command::Reconnect));
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
        assert!(r.contains("/probe"));
        assert!(r.contains("/tensor"));
    }

    #[test]
    fn command_response_ticket_shows_id() {
        let r = command_response(&Command::Ticket("T-5".to_string()));
        assert!(r.contains("T-5"));
    }

    #[test]
    fn command_worker_parses() {
        assert!(matches!(Command::from_str("worker"), Command::Worker(None)));
        assert!(matches!(
            Command::from_text("/worker"),
            Command::Worker(None)
        ));
        match Command::from_text("/worker 999001") {
            Command::Worker(Some(id)) => assert_eq!(id, "999001"),
            _ => panic!("expected worker with id"),
        }
    }

    #[test]
    fn command_response_help_lists_worker() {
        let r = command_response(&Command::Help);
        assert!(r.contains("/worker"));
    }

    #[test]
    fn command_chat_parses() {
        assert!(matches!(
            Command::from_text("/chat hello there"),
            Command::Chat(p) if p == "hello there"
        ));
        let r = command_response(&Command::Help);
        assert!(r.contains("/chat"));
    }

    #[tokio::test]
    async fn handle_chat_needs_prompt_and_sender() {
        let state = crate::state::AppState::new(test_config());
        let r = handle_command_from(&Command::Chat("   ".to_string()), &state, Some("1")).await;
        assert!(r.contains("Usage"));
        let r = handle_command_from(&Command::Chat("hi".to_string()), &state, None).await;
        assert!(r.contains("Telegram account"));
    }

    #[tokio::test]
    async fn handle_chat_unreachable_pool() {
        let mut cfg = test_config();
        cfg.poolai_url = "http://127.0.0.1:9".to_string();
        let state = crate::state::AppState::new(cfg);
        let r = handle_command_from(&Command::Chat("hi".to_string()), &state, Some("1")).await;
        assert!(r.contains("unavailable") || r.contains("No edge peer"));
    }

    #[test]
    fn command_vm_parses() {
        assert!(matches!(Command::from_str("vm"), Command::Vm(None)));
        assert!(matches!(Command::from_text("/vm"), Command::Vm(None)));
        match Command::from_text("/vm a54-01") {
            Command::Vm(Some(id)) => assert_eq!(id, "a54-01"),
            _ => panic!("expected vm with id"),
        }
        let r = command_response(&Command::Help);
        assert!(r.contains("/vm"));
    }

    #[tokio::test]
    async fn handle_vm_unreachable_pool() {
        let mut cfg = test_config();
        cfg.poolai_url = "http://127.0.0.1:9".to_string();
        let state = crate::state::AppState::new(cfg);
        let resp = handle_command(&Command::Vm(None), &state).await;
        assert!(resp.contains("unavailable"));
    }

    #[tokio::test]
    async fn handle_worker_unreachable_pool() {
        // Nothing listens on :9 — fail-fast reply, no hang.
        let mut cfg = test_config();
        cfg.poolai_url = "http://127.0.0.1:9".to_string();
        let state = crate::state::AppState::new(cfg);
        let resp = handle_command(&Command::Worker(None), &state).await;
        assert!(resp.contains("unavailable"));
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
    async fn handle_board_appends_next_hint() {
        // T2.2: the bot suggests the next move, never auto-claims.
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
        assert!(resp.contains("Next:"));
        assert!(resp.contains("claim"));
        assert!(resp.contains("T-1"));
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
