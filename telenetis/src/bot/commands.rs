pub fn parse_command(text: &str) -> Option<String> {
    let cmd = text.split_whitespace().next()?;
    if cmd.starts_with('/') {
        Some(cmd[1..].to_string())
    } else {
        None
    }
}

pub enum Command {
    Start,
    Status,
    Board,
    Flows,
    Roles,
    Help,
    Unknown(String),
}

impl Command {
    pub fn from_str(s: &str) -> Self {
        match s {
            "start" => Self::Start,
            "status" => Self::Status,
            "board" => Self::Board,
            "flows" => Self::Flows,
            "roles" => Self::Roles,
            "help" => Self::Help,
            other => Self::Unknown(other.to_string()),
        }
    }
}

pub fn command_response(cmd: &Command) -> String {
    match cmd {
        Command::Start => "Welcome to Telenetis!\n\nYour Telegram dashboard for GSV bot coordination.\n\nCommands:\n/status - Bot status\n/board - Ticket board\n/flows - Live bot flows\n/roles - Role management\n/help - This message"
            .to_string(),
        Command::Status => "Fetching status...".to_string(),
        Command::Board => "Fetching ticket board...".to_string(),
        Command::Flows => "Opening live flows...".to_string(),
        Command::Roles => "Opening role manager...".to_string(),
        Command::Help => command_response(&Command::Start),
        Command::Unknown(cmd) => format!("Unknown command: /{}", cmd),
    }
}
