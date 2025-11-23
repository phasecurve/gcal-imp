use std::{
    env,
    io::{self, Write},
    process::{Command, Stdio},
};

use chrono::{Local, NaiveDate};

use gcal_imp::{
    calendar::Event as CalendarEvent,
    storage::config::Config,
    sync::sync_engine::SyncEngine,
};

#[derive(Clone, Copy)]
pub enum CliMode {
    Default { sample: bool },
    AgendaDate(NaiveDate),
}

pub fn parse_cli_mode() -> Result<CliMode, String> {
    let mut sample = false;
    let mut agenda_date = None;
    let mut args = env::args().skip(1).peekable();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--sample" => {
                sample = true;
            }
            "--agenda" => {
                let target_date = if let Some(next) = args.peek() {
                    if !next.starts_with("--") {
                        let date_str = args.next().expect("peeked value must exist");
                        NaiveDate::parse_from_str(&date_str, "%Y/%m/%d")
                            .map_err(|_| format!("Invalid date '{}'. Use YYYY/MM/DD.", date_str))?
                    } else {
                        Local::now().date_naive()
                    }
                } else {
                    Local::now().date_naive()
                };
                agenda_date = Some(target_date);
            }
            "--help" => {
                println!("Usage: gcal-imp [--agenda [YYYY/MM/DD]] [--sample]");
                std::process::exit(0);
            }
            _ => return Err(format!("Unknown argument: {}", arg)),
        }
    }

    if let Some(date) = agenda_date {
        Ok(CliMode::AgendaDate(date))
    } else {
        Ok(CliMode::Default { sample })
    }
}

pub async fn run_agenda_mode(date: NaiveDate) -> Result<(), io::Error> {
    let config = Config::load_or_create()
        .map_err(|e| io::Error::other(e.to_string()))?;
    let mut sync_engine = SyncEngine::new(config);

    let mut events = match sync_engine.fetch_events(date, date).await {
        Ok(list) => list,
        Err(e) => {
            eprintln!("Failed to fetch events: {}", e);
            Vec::new()
        }
    };

    events.sort_by_key(|event| event.start);
    let agenda = format_agenda_text(date, &events);
    display_with_pager(&agenda)
}

fn format_agenda_text(date: NaiveDate, events: &[CalendarEvent]) -> String {
    let mut lines = Vec::new();
    lines.push(format!("Agenda – {}", date.format("%A, %B %d, %Y")));
    lines.push(String::new());

    if events.is_empty() {
        lines.push("No events scheduled.".to_string());
    } else {
        for event in events {
            lines.push(format!("- {}", build_agenda_line(event, usize::MAX)));
        }
    }

    lines.join("\n")
}

fn build_agenda_line(event: &CalendarEvent, width: usize) -> String {
    let start_local = event.start.with_timezone(&Local);
    let end_local = event.end.with_timezone(&Local);
    let time_label = if event.all_day {
        "All Day".to_string()
    } else {
        format!(
            "{}-{}",
            start_local.format("%H:%M"),
            end_local.format("%H:%M")
        )
    };

    let mut line = format!("{:<13} {}", time_label, event.title);
    if let Some(location) = &event.location
        && !location.is_empty()
    {
        line.push_str(&format!(" @ {}", location));
    }
    truncate_to_width(&line, width)
}

fn truncate_to_width(line: &str, width: usize) -> String {
    if width > 0 && line.len() > width {
        let mut truncated = line.chars().take(width.saturating_sub(1)).collect::<String>();
        truncated.push('…');
        truncated
    } else {
        line.to_string()
    }
}

fn display_with_pager(text: &str) -> Result<(), io::Error> {
    let pager_value = env::var("PAGER").unwrap_or_else(|_| "less".to_string());
    let mut parts = pager_value.split_whitespace();
    let cmd = match parts.next() {
        Some(c) => c,
        None => {
            print!("{text}");
            return Ok(());
        }
    };
    let args: Vec<&str> = parts.collect();

    match Command::new(cmd)
        .args(&args)
        .stdin(Stdio::piped())
        .spawn()
    {
        Ok(mut child) => {
            if let Some(stdin) = child.stdin.as_mut() {
                stdin.write_all(text.as_bytes())?;
            }
            let _ = child.wait();
        }
        Err(_) => {
            print!("{text}");
        }
    }

    Ok(())
}
