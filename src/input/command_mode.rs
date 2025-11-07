use chrono::NaiveDate;

#[derive(Debug, PartialEq)]
pub enum Command {
    Quit,
    Sync,
    Goto(NaiveDate),
    NewEvent(Option<String>),
    SwitchCalendar(String),
    Theme(String),
    Help,
    Error(String),
}

pub fn parse_command(input: &str) -> Command {
    let trimmed = input.trim();

    if !trimmed.starts_with(':') {
        return Command::Error("Commands must start with ':'".to_string());
    }

    let command_text = &trimmed[1..];
    let parts: Vec<&str> = command_text.split_whitespace().collect();

    if parts.is_empty() {
        return Command::Error("Empty command".to_string());
    }

    match parts[0] {
        "q" | "quit" => Command::Quit,
        "w" | "write" => Command::Sync,
        "help" => Command::Help,
        "goto" => {
            if parts.len() < 2 {
                Command::Error("goto requires a date argument".to_string())
            } else if let Ok(date) = NaiveDate::parse_from_str(parts[1], "%Y-%m-%d") {
                Command::Goto(date)
            } else {
                Command::Error(format!("Invalid date format: {}", parts[1]))
            }
        }
        "new" => {
            if parts.len() < 2 {
                Command::NewEvent(None)
            } else {
                let title = parts[1..].join(" ");
                Command::NewEvent(Some(title))
            }
        }
        "cal" | "calendar" => {
            if parts.len() < 2 {
                Command::Error("cal requires a calendar name".to_string())
            } else {
                Command::SwitchCalendar(parts[1].to_string())
            }
        }
        "theme" => {
            if parts.len() < 2 {
                Command::Error("theme requires a theme name".to_string())
            } else {
                Command::Theme(parts[1].to_string())
            }
        }
        _ => Command::Error(format!("Unknown command: {}", parts[0])),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_quit_command() {
        let cmd = parse_command(":q");
        assert_eq!(cmd, Command::Quit);
    }

    #[test]
    fn parse_quit_long_form() {
        let cmd = parse_command(":quit");
        assert_eq!(cmd, Command::Quit);
    }

    #[test]
    fn parse_write_command_triggers_sync() {
        let cmd = parse_command(":w");
        assert_eq!(cmd, Command::Sync);
    }

    #[test]
    fn parse_write_long_form() {
        let cmd = parse_command(":write");
        assert_eq!(cmd, Command::Sync);
    }

    #[test]
    fn parse_goto_command_with_date() {
        let cmd = parse_command(":goto 2025-01-15");
        let expected_date = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();
        assert_eq!(cmd, Command::Goto(expected_date));
    }

    #[test]
    fn parse_goto_command_with_invalid_date_returns_error() {
        let cmd = parse_command(":goto invalid");
        assert!(matches!(cmd, Command::Error(_)));
    }

    #[test]
    fn parse_goto_without_date_returns_error() {
        let cmd = parse_command(":goto");
        assert!(matches!(cmd, Command::Error(_)));
    }

    #[test]
    fn parse_new_event_command() {
        let cmd = parse_command(":new Team meeting");
        assert_eq!(cmd, Command::NewEvent(Some("Team meeting".to_string())));
    }

    #[test]
    fn parse_new_event_with_multiple_words() {
        let cmd = parse_command(":new Sprint planning session tomorrow");
        assert_eq!(cmd, Command::NewEvent(Some("Sprint planning session tomorrow".to_string())));
    }

    #[test]
    fn parse_new_without_title_returns_blank_title() {
        let cmd = parse_command(":new");
        assert_eq!(cmd, Command::NewEvent(None));
    }

    #[test]
    fn parse_calendar_switch_command() {
        let cmd = parse_command(":cal work");
        assert_eq!(cmd, Command::SwitchCalendar("work".to_string()));
    }

    #[test]
    fn parse_calendar_long_form() {
        let cmd = parse_command(":calendar personal");
        assert_eq!(cmd, Command::SwitchCalendar("personal".to_string()));
    }

    #[test]
    fn parse_help_command() {
        let cmd = parse_command(":help");
        assert_eq!(cmd, Command::Help);
    }

    #[test]
    fn parse_unknown_command_returns_error() {
        let cmd = parse_command(":unknown");
        assert!(matches!(cmd, Command::Error(_)));
    }

    #[test]
    fn parse_command_without_colon_returns_error() {
        let cmd = parse_command("quit");
        assert!(matches!(cmd, Command::Error(_)));
    }

    #[test]
    fn parse_empty_command_returns_error() {
        let cmd = parse_command(":");
        assert!(matches!(cmd, Command::Error(_)));
    }
}
