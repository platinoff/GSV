pub fn parse_command(text: &str) -> Option<String> {
    let cmd = text.split_whitespace().next()?;
    cmd.strip_prefix('/').map(|s| s.to_string())
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
    #[allow(clippy::should_implement_trait)]
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

#[cfg(test)]
mod tests {
    use super::*;

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
    fn command_from_str_known() {
        assert!(matches!(Command::from_str("start"), Command::Start));
        assert!(matches!(Command::from_str("status"), Command::Status));
        assert!(matches!(Command::from_str("help"), Command::Help));
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
}
