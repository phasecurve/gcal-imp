use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as TermEvent, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal,
};
use std::{env, io::{self, Write}, process::{Command, Stdio}, sync::OnceLock};

use gcal_imp::{
    app::{AppState, ViewType, Mode, SyncStatus, EventForm},
    input::{normal_mode, command_mode, insert_mode, visual_mode},
    ui::{month_view, week_view, day_view, year_view, theme::Theme},
    storage::config::Config,
    sync::{google_auth::GoogleAuthenticator, sync_engine::SyncEngine},
    calendar::{Event as CalendarEvent, EventStatus},
};
use chrono::{Datelike, Local, NaiveDate};

#[tokio::main]
async fn main() -> Result<(), io::Error> {
    setup_logging();

    let cli_mode = match parse_cli_mode() {
        Ok(mode) => mode,
        Err(err) => {
            eprintln!("Error: {}", err);
            println!("Usage: gcal-imp [--agenda [YYYY/MM/DD]]");
            return Ok(());
        }
    };

    if let CliMode::AgendaDate(_) = cli_mode {
        if let Err(e) = check_or_setup_auth().await {
            eprintln!("Authentication error: {}", e);
            tracing::error!("Authentication failed: {}", e);
            return Ok(());
        }
        let date = match cli_mode {
            CliMode::AgendaDate(d) => d,
            _ => unreachable!(),
        };
        return run_agenda_mode(date).await;
    }

    if let Err(e) = check_or_setup_auth().await {
        eprintln!("Authentication error: {}", e);
        tracing::error!("Authentication failed: {}", e);
        return Ok(());
    }

    run_tui().await
}

fn setup_logging() {
    let log_dir = dirs::config_dir()
        .map(|d| d.join("gcal-imp"))
        .unwrap_or_else(|| std::path::PathBuf::from("."));

    std::fs::create_dir_all(&log_dir).ok();

    let file_appender = tracing_appender::rolling::daily(log_dir, "gcal-imp.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_target(false)
        .init();

    std::mem::forget(_guard);

    tracing::info!("gcal-imp started");
}

async fn check_or_setup_auth() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::load_or_create()?;

    if config.google.client_id.is_empty() || config.google.client_secret.is_empty() {
        println!("Configuration incomplete. Please edit the config file at:");
        println!("{}", Config::config_path().display());
        println!("\nYou need to set:");
        println!("  - google.client_id: Your Google OAuth2 client ID");
        println!("  - google.client_secret: Your Google OAuth2 client secret");
        println!("\nGet these from: https://console.cloud.google.com/apis/credentials");
        return Err("Missing Google OAuth credentials in config".into());
    }

    let mut auth = GoogleAuthenticator::new(config);

    match auth.get_valid_token().await {
        Ok(_) => {
            println!("Authentication successful! Starting calendar...\n");
            Ok(())
        },
        Err(_) => {
            println!("No valid authentication found. Setting up Google Calendar access...\n");
            auth.print_auth_instructions();

            println!("Enter the authorization code: ");
            let mut code = String::new();
            std::io::stdin().read_line(&mut code)?;
            let code = code.trim();

            auth.exchange_code_for_token(code).await?;
            println!("\nAuthentication successful! You can now use gcal-imp.\n");

            Ok(())
        }
    }
}

async fn run_tui() -> Result<(), io::Error> {
    let config = Config::load_or_create()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let theme = Theme::get_by_name(&config.ui.theme);
    let mut app = AppState::new().with_theme(theme);

    let mut sync_engine = SyncEngine::new(config);

    app.sync_status = SyncStatus::Syncing;
    terminal.draw(|f| ui(f, &app)).ok();

    match sync_engine.fetch_events_around_date(app.selected_date).await {
        Ok(events) => {
            for event in events {
                app.add_event(event);
            }
            app.sync_status = SyncStatus::Synced;
        }
        Err(e) => {
            app.sync_status = SyncStatus::Error(format!("Sync failed: {}", e));
            add_sample_events(&mut app);
        }
    }

    let res = run_app(&mut terminal, &mut app, sync_engine).await;

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("Error: {:?}", err);
    }

    Ok(())
}

async fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut AppState,
    mut sync_engine: SyncEngine,
) -> io::Result<()> {
    loop {
        if app.detail_view_event_id.is_some() {
            refresh_detail_view_lines(app);
        } else if !app.detail_view_line_text.is_empty() {
            app.detail_view_line_text.clear();
        }

        terminal.draw(|f| ui(f, app))?;

        while event::poll(std::time::Duration::from_millis(0))? {
            let _ = event::read()?;
        }

        if let TermEvent::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match app.mode {
                    Mode::Normal => {
                        if app.show_help {
                            match key.code {
                                KeyCode::Char('j') => {
                                    app.help_scroll = app.help_scroll.saturating_add(1);
                                }
                                KeyCode::Char('k') => {
                                    app.help_scroll = app.help_scroll.saturating_sub(1);
                                }
                                KeyCode::Char('q') | KeyCode::Esc => {
                                    app.show_help = false;
                                    app.help_scroll = 0;
                                }
                                _ => {}
                            }
                        } else if app.detail_view_event_id.is_some() {
                            match key.code {
                                KeyCode::Esc => {
                                    app.detail_view_event_id = None;
                                    app.detail_view_scroll = 0;
                                    app.detail_view_cursor_line = 0;
                                    app.detail_view_cursor_col = 0;
                                }
                                KeyCode::Char('j') => {
                                    app.detail_view_cursor_line = app.detail_view_cursor_line.saturating_add(1);
                                }
                                KeyCode::Char('k') => {
                                    app.detail_view_cursor_line = app.detail_view_cursor_line.saturating_sub(1);
                                }
                                KeyCode::Char('h') => {
                                    app.detail_view_cursor_col = app.detail_view_cursor_col.saturating_sub(1);
                                }
                                KeyCode::Char('l') => {
                                    app.detail_view_cursor_col = app.detail_view_cursor_col.saturating_add(1);
                                }
                                KeyCode::Char('0') => {
                                    app.detail_view_cursor_col = 0;
                                }
                                KeyCode::Char('$') => {
                                    if let Some(line_text) = app.detail_view_line_text.get(app.detail_view_cursor_line) {
                                        app.detail_view_cursor_col = last_char_index(line_text);
                                    } else {
                                        app.detail_view_cursor_col = 0;
                                    }
                                }
                                KeyCode::Char('^') => {
                                    if let Some(line_text) = app.detail_view_line_text.get(app.detail_view_cursor_line) {
                                        app.detail_view_cursor_col = find_first_non_whitespace(line_text);
                                    }
                                }
                                KeyCode::Char('w') => {
                                    if !app.detail_view_line_text.is_empty() {
                                        let (line, col) = next_word_position(
                                            &app.detail_view_line_text,
                                            app.detail_view_cursor_line,
                                            app.detail_view_cursor_col,
                                        );
                                        app.detail_view_cursor_line = line;
                                        app.detail_view_cursor_col = col;
                                    }
                                }
                                KeyCode::Char('b') => {
                                    if !app.detail_view_line_text.is_empty() {
                                        let (line, col) = prev_word_position(
                                            &app.detail_view_line_text,
                                            app.detail_view_cursor_line,
                                            app.detail_view_cursor_col,
                                        );
                                        app.detail_view_cursor_line = line;
                                        app.detail_view_cursor_col = col;
                                    }
                                }
                                KeyCode::Char('e') => {
                                    if !app.detail_view_line_text.is_empty() {
                                        let (line, col) = word_end_position(
                                            &app.detail_view_line_text,
                                            app.detail_view_cursor_line,
                                            app.detail_view_cursor_col,
                                        );
                                        app.detail_view_cursor_line = line;
                                        app.detail_view_cursor_col = col;
                                    }
                                }
                                KeyCode::Char('g') => {
                                    app.detail_view_cursor_line = 0;
                                    app.detail_view_scroll = 0;
                                }
                                KeyCode::Char('G') => {
                                    app.detail_view_cursor_line = 999;
                                }
                                KeyCode::Char('E') => {
                                    if let Some(event_id) = &app.detail_view_event_id {
                                        if let Some(event) = app.events.get(event_id).cloned() {
                                            app.event_form = Some(EventForm::for_event(&event));
                                            app.mode = Mode::Insert;
                                            app.detail_view_event_id = None;
                                            app.detail_view_scroll = 0;
                                            app.detail_view_cursor_line = 0;
                                            app.detail_view_cursor_col = 0;
                                        }
                                    }
                                }
                                KeyCode::Char('a') => {
                                    app.event_form = Some(EventForm::new(app.selected_date, String::new()));
                                    app.mode = Mode::Insert;
                                    app.detail_view_event_id = None;
                                    app.detail_view_scroll = 0;
                                    app.detail_view_cursor_line = 0;
                                    app.detail_view_cursor_col = 0;
                                    app.detail_view_visual_start = None;
                                }
                                KeyCode::Char('o') => {
                                    tracing::info!("Attempting to open URL at cursor position");
                                    if let Some(event_id) = &app.detail_view_event_id {
                                        if let Some(event) = app.events.get(event_id) {
                                            let mut all_lines = Vec::new();

                                            all_lines.push(event.title.clone());
                                            all_lines.push(String::new());

                                            if event.all_day {
                                                all_lines.push(format!("📅 {}", event.start.format("%A, %B %d, %Y")));
                                            } else {
                                                all_lines.push(format!("📅 {} at {}", event.start.format("%A, %B %d, %Y"), event.start.format("%H:%M")));
                                            }

                                            if event.all_day {
                                                let duration_days = (event.end - event.start).num_days();
                                                if duration_days > 1 {
                                                    all_lines.push(format!("⏱  {} days", duration_days));
                                                }
                                            } else {
                                                let duration = event.duration_minutes();
                                                if duration >= 60 {
                                                    all_lines.push(format!("⏱  {} hour{} {} min", duration / 60, if duration / 60 > 1 { "s" } else { "" }, duration % 60));
                                                } else {
                                                    all_lines.push(format!("⏱  {} minutes", duration));
                                                }
                                            }

                                            if let Some(location) = &event.location {
                                                all_lines.push(String::new());
                                                all_lines.push("📍 Location:".to_string());
                                                all_lines.push(format!("   {}", location));
                                            }

                                            if let Some(desc) = &event.description {
                                                all_lines.push(String::new());
                                                all_lines.push("📝 Description:".to_string());
                                                all_lines.push(String::new());
                                                let clean_desc = strip_html(desc);
                                                for line in clean_desc.lines() {
                                                    all_lines.push(line.to_string());
                                                }
                                            }

                                            if !event.attendees.is_empty() {
                                                all_lines.push(String::new());
                                                all_lines.push("👥 Attendees:".to_string());
                                                for attendee in &event.attendees {
                                                    all_lines.push(format!("   • {}", attendee));
                                                }
                                            }

                                            if app.detail_view_cursor_line < all_lines.len() {
                                                let line_text = &all_lines[app.detail_view_cursor_line];

                                                let markdown_link_pattern = regex::Regex::new(r"\[([^\]]+)\]\((https?://[^\)]+)\)").unwrap();
                                                let plain_url_pattern = regex::Regex::new(r"(https?://[^\s\)]+)").unwrap();

                                                let url_to_open = if let Some(cap) = markdown_link_pattern.captures(line_text) {
                                                    cap.get(2).map(|m| m.as_str())
                                                } else if let Some(cap) = plain_url_pattern.captures(line_text) {
                                                    cap.get(1).map(|m| m.as_str())
                                                } else {
                                                    None
                                                };

                                                if let Some(url) = url_to_open {
                                                    tracing::info!("Opening URL: {}", url);
                                                    match std::process::Command::new("xdg-open").arg(url).spawn() {
                                                        Ok(_) => tracing::info!("Successfully launched xdg-open"),
                                                        Err(e) => tracing::error!("Failed to open URL: {}", e),
                                                    }
                                                } else {
                                                    tracing::info!("No URL found on current line");
                                                }
                                            }
                                        }
                                    }
                                }
                                KeyCode::Char('y') => {
                                    if !app.detail_view_line_text.is_empty() {
                                        let lines = &app.detail_view_line_text;

                                        let text_to_yank = if let Some((start_line, start_col)) = app.detail_view_visual_start {
                                            let end_line = app.detail_view_cursor_line;
                                            let end_col = app.detail_view_cursor_col;

                                            let (start_line, start_col, end_line, end_col) = if start_line < end_line || (start_line == end_line && start_col <= end_col) {
                                                (start_line, start_col, end_line, end_col)
                                            } else {
                                                (end_line, end_col, start_line, start_col)
                                            };

                                            let mut selected_text = String::new();
                                            for line_idx in start_line..=end_line.min(lines.len().saturating_sub(1)) {
                                                if let Some(line) = lines.get(line_idx) {
                                                    let chars: Vec<char> = line.chars().collect();

                                                    if line_idx == start_line && line_idx == end_line {
                                                        let start = start_col.min(chars.len());
                                                        let end = (end_col + 1).min(chars.len());
                                                        selected_text.push_str(&chars[start..end].iter().collect::<String>());
                                                    } else if line_idx == start_line {
                                                        let start = start_col.min(chars.len());
                                                        selected_text.push_str(&chars[start..].iter().collect::<String>());
                                                        selected_text.push('\n');
                                                    } else if line_idx == end_line {
                                                        let end = (end_col + 1).min(chars.len());
                                                        selected_text.push_str(&chars[..end].iter().collect::<String>());
                                                    } else {
                                                        selected_text.push_str(line);
                                                        selected_text.push('\n');
                                                    }
                                                }
                                            }

                                            app.detail_view_visual_start = None;
                                            tracing::info!("Yanking visual selection: {}", selected_text);
                                            selected_text
                                        } else if let Some(line) = lines.get(app.detail_view_cursor_line) {
                                            tracing::info!("Yanking line: {}", line);
                                            line.clone()
                                        } else {
                                            String::new()
                                        };

                                        if !text_to_yank.is_empty() {
                                            tracing::info!("About to yank {} bytes: '{}'", text_to_yank.len(), text_to_yank);
                                            let _ = copy_to_clipboard(&text_to_yank);
                                        }
                                    }
                                }
                                KeyCode::Char('q') => return Ok(()),
                                KeyCode::Char('v') => {
                                    if app.detail_view_visual_start.is_some() {
                                        app.detail_view_visual_start = None;
                                    } else {
                                        app.detail_view_visual_start = Some((app.detail_view_cursor_line, app.detail_view_cursor_col));
                                    }
                                }
                                KeyCode::Char('B') => {
                                    tracing::info!("Opening event in browser");
                                    if let Some(event_id) = &app.detail_view_event_id {
                                        if let Some(event) = app.events.get(event_id) {
                                            let url = event.html_link.clone()
                                                .unwrap_or_else(|| format!("https://calendar.google.com/calendar/u/0/r/eventedit/{}", event.id));
                                            tracing::info!("Opening Google Calendar URL: {}", url);
                                            match std::process::Command::new("xdg-open").arg(&url).spawn() {
                                                Ok(_) => tracing::info!("Successfully launched browser"),
                                                Err(e) => tracing::error!("Failed to open browser: {}", e),
                                            }
                                        }
                                    }
                                }
                                _ => {}
                            }
                        } else {
                            match key.code {
                                KeyCode::Char('q') => return Ok(()),
                                _ => normal_mode::handle_key(key.code, app),
                            }
                        }
                    }
                    Mode::Command => {
                        match key.code {
                            KeyCode::Enter => {
                                let command_text = app.command_buffer.clone();
                                let cmd = command_mode::parse_command(&command_text);

                                match cmd {
                                    command_mode::Command::Quit => return Ok(()),
                                    command_mode::Command::Sync => {
                                        app.command_buffer.clear();
                                        app.mode = Mode::Normal;
                                        app.sync_status = SyncStatus::Syncing;
                                        terminal.draw(|f| ui(f, app))?;

                                        match sync_engine.fetch_events_around_date(app.selected_date).await {
                                            Ok(events) => {
                                                app.events.clear();
                                                for event in events {
                                                    app.add_event(event);
                                                }
                                                app.sync_status = SyncStatus::Synced;
                                            }
                                            Err(e) => {
                                                app.sync_status = SyncStatus::Error(format!("Sync failed: {}", e));
                                            }
                                        }
                                    }
                                    command_mode::Command::Goto(date) => {
                                        app.selected_date = date;
                                        app.command_buffer.clear();
                                        app.mode = Mode::Normal;
                                    }
                                    command_mode::Command::Help => {
                                        app.show_help = !app.show_help;
                                        app.command_buffer.clear();
                                        app.mode = Mode::Normal;
                                    }
                                    command_mode::Command::Theme(theme_name) => {
                                        app.theme = Theme::get_by_name(&theme_name);
                                        app.command_buffer.clear();
                                        app.mode = Mode::Normal;
                                    }
                                    command_mode::Command::NewEvent(title) => {
                                        let title = title.unwrap_or_default();
                                        let form = EventForm::new(app.selected_date, title);
                                        app.event_form = Some(form);
                                        app.command_buffer.clear();
                                        app.mode = Mode::Insert;
                                    }
                                    _ => {
                                        app.command_buffer.clear();
                                        app.mode = Mode::Normal;
                                    }
                                }
                            }
                            KeyCode::Esc => {
                                app.command_buffer.clear();
                                app.mode = Mode::Normal;
                            }
                            KeyCode::Backspace => {
                                app.command_buffer.pop();
                            }
                            KeyCode::Char(c) => {
                                app.command_buffer.push(c);
                            }
                            _ => {}
                        }
                    }
                    Mode::Insert => {
                        match key.code {
                            KeyCode::Esc => {
                                app.event_form = None;
                                app.mode = Mode::Normal;
                            }
                            KeyCode::Enter => {
                                if let Some(mut form) = app.event_form.take() {
                                    form.parse_time_input();
                                    form.parse_duration_input();

                                    let (start_datetime, end_datetime, all_day) = if form.all_day {
                                        let days = form.duration_minutes / (24 * 60);
                                        let start = form.date.and_hms_opt(0, 0, 0)
                                            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid date"))?
                                            .and_utc();
                                        let end = start + chrono::Duration::days(days as i64);
                                        (start, end, true)
                                    } else {
                                        let start = form.date
                                            .and_hms_opt(form.start_hour, form.start_minute, 0)
                                            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid time"))?
                                            .and_utc();
                                        let end = start + chrono::Duration::minutes(form.duration_minutes as i64);
                                        (start, end, false)
                                    };

                                    if let Some(event_id) = form.event_id {
                                        let existing_link = app.events.get(&event_id).and_then(|e| e.html_link.clone());
                                        let event = CalendarEvent {
                                            id: event_id.clone(),
                                            calendar_id: "primary".to_string(),
                                            title: form.title.clone(),
                                            description: if form.description.is_empty() { None } else { Some(form.description.clone()) },
                                            location: if form.location.is_empty() { None } else { Some(form.location.clone()) },
                                            start: start_datetime,
                                            end: end_datetime,
                                            all_day,
                                            attendees: vec![],
                                            reminders: vec![],
                                            status: EventStatus::Confirmed,
                                            last_modified: chrono::Utc::now(),
                                            html_link: existing_link,
                                        };

                                        tracing::info!("Updating event: {} (id: {})", event.title, event.id);
                                        app.sync_status = SyncStatus::Syncing;
                                        terminal.draw(|f| ui(f, app))?;

                                        match sync_engine.update_event(&event).await {
                                            Ok(()) => {
                                                tracing::info!("Event updated successfully");
                                                app.add_event(event);
                                                app.sync_status = SyncStatus::Synced;
                                            }
                                            Err(e) => {
                                                tracing::error!("Failed to update event: {}", e);
                                                app.sync_status = SyncStatus::Error(format!("Failed to update event: {}", e));
                                            }
                                        }
                                    } else {
                                        let event = CalendarEvent {
                                            id: uuid::Uuid::new_v4().to_string(),
                                            calendar_id: "primary".to_string(),
                                            title: form.title.clone(),
                                            description: if form.description.is_empty() { None } else { Some(form.description.clone()) },
                                            location: if form.location.is_empty() { None } else { Some(form.location.clone()) },
                                            start: start_datetime,
                                            end: end_datetime,
                                            all_day,
                                            attendees: vec![],
                                            reminders: vec![],
                                            status: EventStatus::Confirmed,
                                            last_modified: chrono::Utc::now(),
                                            html_link: None,
                                        };

                                        tracing::info!("Creating new event: {}", event.title);
                                        app.sync_status = SyncStatus::Syncing;
                                        terminal.draw(|f| ui(f, app))?;

                                        match sync_engine.create_event(&event).await {
                                            Ok(created_info) => {
                                                tracing::info!("Event created successfully with id: {}", created_info.id);
                                                let mut created_event = event;
                                                created_event.id = created_info.id;
                                                created_event.html_link = created_info.html_link;
                                                app.add_event(created_event);
                                                app.sync_status = SyncStatus::Synced;
                                            }
                                            Err(e) => {
                                                tracing::error!("Failed to create event: {}", e);
                                                app.sync_status = SyncStatus::Error(format!("Failed to create event: {}", e));
                                            }
                                        }
                                    }

                                    app.mode = Mode::Normal;
                                }
                            }
                            _ => {
                                insert_mode::handle_key(key.code, app);
                            }
                        }
                    }
                    Mode::Visual => {
                        if app.delete_confirmation_event_id.is_some() {
                            match key.code {
                                KeyCode::Char('y') | KeyCode::Char('Y') => {
                                    if let Some(event_id) = app.delete_confirmation_event_id.take() {
                                        tracing::info!("Deleting event: {}", event_id);
                                        app.sync_status = SyncStatus::Syncing;
                                        terminal.draw(|f| ui(f, app))?;

                                        match sync_engine.delete_event(&event_id).await {
                                            Ok(()) => {
                                                tracing::info!("Event deleted successfully");
                                                app.remove_event(&event_id);
                                                app.sync_status = SyncStatus::Synced;
                                                if app.selected_event_index > 0 {
                                                    app.selected_event_index -= 1;
                                                }
                                            }
                                            Err(e) => {
                                                tracing::error!("Failed to delete event: {}", e);
                                                app.sync_status = SyncStatus::Error(format!("Failed to delete event: {}", e));
                                            }
                                        }
                                    }
                                    app.mode = Mode::Normal;
                                }
                                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                                    app.delete_confirmation_event_id = None;
                                    app.mode = Mode::Normal;
                                }
                                _ => {}
                            }
                        } else {
                            visual_mode::handle_key(key.code, app);
                        }
                    }
                }
            }
        }
    }
}

enum CliMode {
    Default,
    AgendaDate(NaiveDate),
}

fn parse_cli_mode() -> Result<CliMode, String> {
    let mut mode = CliMode::Default;
    let mut args = env::args().skip(1).peekable();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--agenda" => {
                let target_date = match args.peek() {
                    Some(next) if !next.starts_with("--") => {
                        let date_str = args.next().unwrap();
                        NaiveDate::parse_from_str(&date_str, "%Y/%m/%d")
                            .map_err(|_| format!("Invalid date '{}'. Use YYYY/MM/DD.", date_str))?
                    }
                    _ => Local::now().date_naive(),
                };
                mode = CliMode::AgendaDate(target_date);
            }
            "--help" => {
                println!("Usage: gcal-imp [--agenda [YYYY/MM/DD]]");
                std::process::exit(0);
            }
            _ => return Err(format!("Unknown argument: {}", arg)),
        }
    }
    Ok(mode)
}

async fn run_agenda_mode(date: NaiveDate) -> Result<(), io::Error> {
    let config = Config::load_or_create()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
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

    lines.join("
")
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
    if let Some(location) = &event.location {
        if !location.is_empty() {
            line.push_str(&format!(" @ {}", location));
        }
    }
    truncate_to_width(&line, width)
}

fn truncate_to_width(line: &str, width: usize) -> String {
    if width > 0 && line.len() > width {
        let mut truncated = line.chars().take(width.saturating_sub(1)).collect::<String>();
        truncated.push('.');
        truncated
    } else {
        line.to_string()
    }
}

fn refresh_detail_view_lines(app: &mut AppState) {
    if let Some(event_id) = &app.detail_view_event_id {
        if let Some(event) = app.events.get(event_id) {
            app.detail_view_line_text = build_event_detail_lines(event);
        } else {
            app.detail_view_line_text.clear();
        }
    }
}

fn ui(f: &mut Frame, app: &AppState) {
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(f.size());

    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(60),
            Constraint::Percentage(40),
        ])
        .split(main_chunks[1]);

    let chunks = [main_chunks[0], content_chunks[0], content_chunks[1], main_chunks[2]];

    let title_text = format!("gcal-imp - {} View - {:?} Mode",
        match app.view {
            ViewType::Month => "Month",
            ViewType::Week => "Week",
            ViewType::Day => "Day",
            ViewType::Year => "Year",
        },
        app.mode
    );

    let title = Paragraph::new(title_text)
        .style(Style::default().fg(app.theme.title).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, chunks[0]);

    match app.view {
        ViewType::Month => render_month_view(f, app, chunks[1]),
        ViewType::Week => render_week_view(f, app, chunks[1]),
        ViewType::Day => render_day_view(f, app, chunks[1]),
        ViewType::Year => render_year_view(f, app, chunks[1]),
    }

    render_event_list(f, app, chunks[2]);

    let status_text = if matches!(app.mode, Mode::Command) {
        format!("{}", app.command_buffer)
    } else {
        format!("Events: {} | Sync: {:?} | Press 'q' to quit, '?' for help",
            app.events.len(), app.sync_status)
    };

    let status_color = if matches!(app.mode, Mode::Command) {
        app.theme.command_mode
    } else {
        app.theme.status_bar
    };

    let status = Paragraph::new(status_text)
        .style(Style::default().fg(status_color))
        .alignment(if matches!(app.mode, Mode::Command) { Alignment::Left } else { Alignment::Center })
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(status, chunks[3]);

    if app.show_help {
        render_help_overlay(f, app);
    }

    if app.event_form.is_some() {
        render_event_form(f, app);
    }

    if app.delete_confirmation_event_id.is_some() {
        render_delete_confirmation(f, app);
    }

    if app.detail_view_event_id.is_some() {
        render_event_detail(f, app);
    }
}

fn render_help_overlay(f: &mut Frame, app: &AppState) {
    use ratatui::widgets::Clear;

    let area = f.size();
    let help_width = 60;
    let help_height = 23;
    let x = (area.width.saturating_sub(help_width)) / 2;
    let y = (area.height.saturating_sub(help_height)) / 2;

    let help_area = ratatui::layout::Rect {
        x,
        y,
        width: help_width,
        height: help_height,
    };

    f.render_widget(Clear, help_area);

    let help_text = vec![
        Line::from(vec![Span::styled("gcal-imp Help", Style::default().fg(app.theme.help_title).add_modifier(Modifier::BOLD))]),
        Line::from(""),
        Line::from(vec![Span::styled("Navigation:", Style::default().fg(app.theme.help_section))]),
        Line::from("  h/l      - Previous/next day"),
        Line::from("  j/k      - Navigate events (or week if no events)"),
        Line::from("  t        - Jump to today"),
        Line::from("  g/G      - First/last day of month"),
        Line::from("  { / }    - Previous/next month"),
        Line::from(""),
        Line::from(vec![Span::styled("Views:", Style::default().fg(app.theme.help_section))]),
        Line::from("  m/w/d/y  - Month/Week/Day/Year view"),
        Line::from(""),
        Line::from(vec![Span::styled("Event Management:", Style::default().fg(app.theme.help_section))]),
        Line::from("  a        - Add new event (insert mode)"),
        Line::from("  :new     - Create event (:new [Meeting title])"),
        Line::from("  Enter    - Day view (Month) / Edit (Day)"),
        Line::from("  i        - View event details (scrollable)"),
        Line::from("  E        - Edit selected event"),
        Line::from("  x        - Delete selected event"),
        Line::from("  v        - Visual mode (select date range)"),
        Line::from(""),
        Line::from(vec![Span::styled("Detail View:", Style::default().fg(app.theme.help_section))]),
        Line::from("  hjkl     - Navigate cursor"),
        Line::from("  wbe      - Word motions"),
        Line::from("  0^$      - Line start/first-non-ws/end"),
        Line::from("  gG       - Top/bottom"),
        Line::from("  a        - Add new event"),
        Line::from("  o        - Open URL at cursor"),
        Line::from("  y        - Yank line to clipboard"),
        Line::from("  B        - Open event in browser"),
        Line::from("  E        - Edit event"),
        Line::from("  q/Esc    - Close detail view"),
        Line::from(""),
        Line::from(vec![Span::styled("Commands:", Style::default().fg(app.theme.help_section))]),
        Line::from("  :q       - Quit"),
        Line::from("  :w       - Sync with Google Calendar"),
        Line::from("  :goto    - Jump to date (:goto 2025-12-25)"),
        Line::from("  :theme   - Change theme (:theme gruvbox)"),
        Line::from("  :help    - Show this help"),
        Line::from(""),
    ];

    let visible_lines = help_height.saturating_sub(3) as usize;
    let total_lines = help_text.len();
    let max_scroll = total_lines.saturating_sub(visible_lines);
    let scroll = app.help_scroll.min(max_scroll);

    let scrolled_text: Vec<Line> = help_text
        .into_iter()
        .skip(scroll)
        .take(visible_lines)
        .collect();

    let help_paragraph = Paragraph::new(scrolled_text)
        .block(Block::default()
            .borders(Borders::ALL)
            .title(format!(" Help (j/k to scroll, q to close) [{}/{}] ", scroll + 1, total_lines))
            .style(Style::default().bg(Color::Black)))
        .alignment(Alignment::Left);

    f.render_widget(help_paragraph, help_area);
}

fn render_month_view(f: &mut Frame, app: &AppState, area: ratatui::layout::Rect) {
    let layout = month_view::calculate_layout(app);

    let month_name = chrono::NaiveDate::from_ymd_opt(layout.year, layout.month, 1)
        .unwrap()
        .format("%B %Y")
        .to_string();

    let mut lines = vec![
        Line::from(vec![
            Span::styled(month_name, Style::default().fg(app.theme.title).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(" Mon ", Style::default().fg(app.theme.weekday_header)),
            Span::styled(" Tue ", Style::default().fg(app.theme.weekday_header)),
            Span::styled(" Wed ", Style::default().fg(app.theme.weekday_header)),
            Span::styled(" Thu ", Style::default().fg(app.theme.weekday_header)),
            Span::styled(" Fri ", Style::default().fg(app.theme.weekday_header)),
            Span::styled(" Sat ", Style::default().fg(app.theme.weekday_header)),
            Span::styled(" Sun ", Style::default().fg(app.theme.weekday_header)),
        ]),
    ];

    for week in &layout.weeks {
        let mut day_spans = Vec::new();

        for day_cell in &week.days {
            let day_text = if let Some(date) = day_cell.date {
                format!(" {:>2}  ", date.day())
            } else {
                "     ".to_string()
            };

            let mut style = Style::default();

            let is_in_visual_selection = day_cell.date
                .map(|d| app.is_date_in_visual_selection(d))
                .unwrap_or(false);

            if !day_cell.is_current_month {
                style = style.fg(app.theme.inactive_day);
            } else if is_in_visual_selection {
                style = style.bg(Color::DarkGray).fg(Color::White).add_modifier(Modifier::BOLD);
            } else if day_cell.is_selected {
                style = style.bg(Color::Blue).fg(Color::White).add_modifier(Modifier::BOLD);
            } else if day_cell.is_today {
                style = style.fg(Color::Green).add_modifier(Modifier::BOLD);
            }

            if day_cell.has_events {
                style = style.add_modifier(Modifier::UNDERLINED);
            }

            day_spans.push(Span::styled(day_text, style));
        }

        lines.push(Line::from(day_spans));
    }

    lines.push(Line::from(""));

    if app.mode == Mode::Visual && app.visual_selection_start.is_some() {
        if let Some((start, end)) = app.get_visual_selection_range() {
            let days = (end - start).num_days() + 1;
            lines.push(Line::from(vec![
                Span::styled("VISUAL ", Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
                Span::styled(format!("({} day{})", days, if days == 1 { "" } else { "s" }), Style::default().fg(Color::Yellow)),
                Span::raw(" | "),
                Span::styled("Enter", Style::default().fg(Color::Green)),
                Span::raw(" = Create event | "),
                Span::styled("Esc", Style::default().fg(Color::Red)),
                Span::raw(" = Cancel"),
            ]));
        }
    } else {
        lines.push(Line::from(vec![
            Span::styled("hjkl", Style::default().fg(Color::Cyan)),
            Span::raw(" = Navigate | "),
            Span::styled("a", Style::default().fg(Color::Green)),
            Span::raw(" = Add event | "),
            Span::styled("Enter", Style::default().fg(Color::Green)),
            Span::raw(" = Day view | "),
            Span::styled("v", Style::default().fg(Color::Magenta)),
            Span::raw(" = Visual | "),
            Span::styled("m/w/d", Style::default().fg(Color::Cyan)),
            Span::raw(" = Views"),
        ]));
    }

    let content = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(content, area);
}

fn render_event_list(f: &mut Frame, app: &AppState, area: ratatui::layout::Rect) {
    let events = app.get_events_for_date(app.selected_date);

    let title = format!("Events on {}", app.selected_date.format("%B %d, %Y"));

    let mut lines = vec![
        Line::from(vec![
            Span::styled(title, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
    ];

    if events.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("No events", Style::default().fg(Color::DarkGray)),
        ]));
    } else {
        for (idx, event) in events.iter().enumerate() {
            let time_str = event.start.format("%H:%M").to_string();
            let is_selected = idx == app.selected_event_index;

            let time_style = if is_selected {
                Style::default().bg(app.theme.selected_bg).fg(Color::Black).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Green)
            };

            let title_style = if is_selected {
                Style::default().bg(app.theme.selected_bg).fg(Color::Black).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let cursor = if is_selected { ">" } else { " " };

            lines.push(Line::from(vec![
                Span::styled(cursor, Style::default().fg(app.theme.selected_bg)),
                Span::styled(time_str, time_style),
                Span::raw(" "),
                Span::styled(&event.title, title_style),
            ]));

            if let Some(location) = &event.location {
                let loc_style = if is_selected {
                    Style::default().bg(app.theme.selected_bg).fg(Color::DarkGray)
                } else {
                    Style::default().fg(Color::DarkGray)
                };

                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled("📍 ", Style::default()),
                    Span::styled(location, loc_style),
                ]));
            }

            lines.push(Line::from(""));
        }

        if !events.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("j/k", Style::default().fg(Color::Cyan)),
                Span::raw(" = Navigate | "),
                Span::styled("E", Style::default().fg(Color::Green)),
                Span::raw(" = Edit | "),
                Span::styled("x", Style::default().fg(Color::Red)),
                Span::raw(" = Delete"),
            ]));
        }
    }

    let content = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(content, area);
}

fn render_week_view(f: &mut Frame, app: &AppState, area: ratatui::layout::Rect) {
    let layout = week_view::calculate_layout(app);

    let week_range = format!("{} - {}",
        layout.week_start.format("%b %d"),
        layout.days.last().unwrap().date.format("%b %d, %Y")
    );

    let mut lines = vec![
        Line::from(vec![
            Span::styled(week_range, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
    ];

    let mut header_spans = vec![Span::styled("      ", Style::default())];

    for day in &layout.days {
        let day_str = format!(" {:>3} {:<2} ",
            day.date.format("%a"),
            day.date.day()
        );

        let is_in_visual_selection = app.is_date_in_visual_selection(day.date);

        let style = if is_in_visual_selection {
            Style::default().bg(Color::DarkGray).fg(Color::White).add_modifier(Modifier::BOLD)
        } else if day.is_selected {
            Style::default().bg(Color::Blue).fg(Color::White)
        } else if day.is_today {
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Yellow)
        };

        header_spans.push(Span::styled(day_str, style));
    }

    lines.push(Line::from(header_spans));
    lines.push(Line::from(""));

    for hour in 6..22 {
        let time_str = format!("{:02}:00 ", hour);
        let mut line_spans = vec![Span::styled(time_str, Style::default().fg(Color::Gray))];

        for day in &layout.days {
            let hour_events: Vec<_> = day.events.iter()
                .filter(|slot| slot.hour == hour)
                .flat_map(|slot| &slot.events)
                .collect();

            let cell_text = if !hour_events.is_empty() {
                format!("{:>8}", hour_events.len())
            } else {
                "        ".to_string()
            };

            line_spans.push(Span::raw(cell_text));
        }

        lines.push(Line::from(line_spans));
    }

    let content = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(content, area);
}

fn render_day_view(f: &mut Frame, app: &AppState, area: ratatui::layout::Rect) {
    let layout = day_view::calculate_layout(app);

    let day_title = format!("{}", layout.date.format("%A, %B %d, %Y"));

    let mut lines = vec![
        Line::from(vec![
            Span::styled(day_title, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
    ];

    for hour_block in &layout.hours {
        if !hour_block.events.is_empty() {
            let time_label = format!("{:02}:00", hour_block.hour);
            lines.push(Line::from(vec![
                Span::styled(time_label, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            ]));

            for event in &hour_block.events {
                let time_str = format!("  {:02}:{:02}", hour_block.hour, event.start_minute);
                lines.push(Line::from(vec![
                    Span::styled(time_str, Style::default().fg(Color::Green)),
                    Span::raw(" "),
                    Span::styled(&event.title, Style::default().fg(Color::White)),
                    Span::styled(format!(" ({}m)", event.duration_minutes), Style::default().fg(Color::DarkGray)),
                ]));

                if let Some(location) = &event.location {
                    lines.push(Line::from(vec![
                        Span::raw("      📍 "),
                        Span::styled(location, Style::default().fg(Color::DarkGray)),
                    ]));
                }
            }

            lines.push(Line::from(""));
        }
    }

    if lines.len() == 2 {
        lines.push(Line::from(vec![
            Span::styled("No events scheduled", Style::default().fg(Color::DarkGray)),
        ]));
    }

    let content = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(content, area);
}

fn render_year_view(f: &mut Frame, app: &AppState, area: ratatui::layout::Rect) {
    let layout = year_view::calculate_layout(app);

    let mut lines = vec![
        Line::from(vec![
            Span::styled(format!("{}", layout.year), Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
    ];

    for row in 0..4 {
        let mut month_headers = Vec::new();
        for col in 0..3 {
            let month_idx = row * 3 + col;
            if month_idx < layout.months.len() {
                let month = &layout.months[month_idx];
                let month_name = match month.month {
                    1 => "Jan", 2 => "Feb", 3 => "Mar", 4 => "Apr",
                    5 => "May", 6 => "Jun", 7 => "Jul", 8 => "Aug",
                    9 => "Sep", 10 => "Oct", 11 => "Nov", 12 => "Dec",
                    _ => "???",
                };

                let style = if month.is_current_month {
                    Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Yellow)
                };

                month_headers.push(Span::styled(format!("{:^21}", month_name), style));
                if col < 2 {
                    month_headers.push(Span::raw(" "));
                }
            }
        }
        lines.push(Line::from(month_headers));

        let mut dow_headers = Vec::new();
        for col in 0..3 {
            let month_idx = row * 3 + col;
            if month_idx < layout.months.len() {
                let mut header_spans = Vec::new();
                for (i, dow) in ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"].iter().enumerate() {
                    if i > 0 {
                        header_spans.push(Span::styled(" ", Style::default().fg(Color::DarkGray)));
                    }
                    header_spans.push(Span::styled(format!("{:2}", dow), Style::default().fg(Color::DarkGray)));
                }
                dow_headers.extend(header_spans);
                if col < 2 {
                    dow_headers.push(Span::styled(" │ ", Style::default().fg(Color::DarkGray)));
                }
            }
        }
        lines.push(Line::from(dow_headers));

        let mut max_weeks = 0;
        for col in 0..3 {
            let month_idx = row * 3 + col;
            if month_idx < layout.months.len() {
                let month = &layout.months[month_idx];
                let weeks = (month.days.len() + month.first_weekday as usize + 6) / 7;
                max_weeks = max_weeks.max(weeks);
            }
        }

        for week in 0..max_weeks {
            let mut week_spans = Vec::new();

            for col in 0..3 {
                let month_idx = row * 3 + col;
                if month_idx < layout.months.len() {
                    let month = &layout.months[month_idx];

                    for weekday in 0..7 {
                        if weekday > 0 {
                            week_spans.push(Span::raw(" "));
                        }

                        let absolute_day = week * 7 + weekday;

                        if absolute_day < month.first_weekday as usize || absolute_day - month.first_weekday as usize >= month.days.len() {
                            week_spans.push(Span::raw("  "));
                        } else {
                            let day_idx = absolute_day - month.first_weekday as usize;
                            let day = &month.days[day_idx];

                            let style = if day.is_selected {
                                Style::default().bg(Color::Blue).fg(Color::White).add_modifier(Modifier::BOLD)
                            } else if day.is_today {
                                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
                            } else if day.has_events {
                                Style::default().fg(Color::Yellow)
                            } else {
                                Style::default()
                            };

                            week_spans.push(Span::styled(format!("{:2}", day.day), style));
                        }
                    }

                    if col < 2 {
                        week_spans.push(Span::styled(" │ ", Style::default().fg(Color::DarkGray)));
                    }
                } else {
                    week_spans.push(Span::raw(format!("{:<20}", "")));
                    if col < 2 {
                        week_spans.push(Span::styled(" │ ", Style::default().fg(Color::DarkGray)));
                    }
                }
            }

            lines.push(Line::from(week_spans));
        }

        lines.push(Line::from(""));
    }

    let content = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(content, area);
}

fn add_sample_events(app: &mut AppState) {
    use chrono::{Local, TimeZone, Utc};
    use gcal_imp::calendar::{Event as CalendarEvent, EventStatus};

    let today = Local::now().date_naive();

    let events = vec![
        ("Morning Standup", today, 9, 0, 9, 30, None),
        ("Team Sync", today, 14, 0, 15, 0, Some("Conference Room A")),
        ("Code Review", today.succ_opt().unwrap(), 10, 0, 11, 0, None),
        ("Sprint Planning", today.succ_opt().unwrap(), 15, 0, 16, 30, Some("Zoom")),
        ("1-on-1 with Manager", today.pred_opt().unwrap(), 11, 0, 11, 30, None),
        ("Lunch with Team", today.pred_opt().unwrap(), 12, 30, 13, 30, Some("Downtown Cafe")),
    ];

    for (i, (title, date, start_h, start_m, end_h, end_m, location)) in events.into_iter().enumerate() {
        let start = Utc.from_local_datetime(&date.and_hms_opt(start_h, start_m, 0).unwrap()).unwrap();
        let end = Utc.from_local_datetime(&date.and_hms_opt(end_h, end_m, 0).unwrap()).unwrap();

        let event = CalendarEvent {
            id: format!("sample_{}", i),
            calendar_id: "primary".to_string(),
            title: title.to_string(),
            description: Some("Sample event for testing".to_string()),
            location: location.map(String::from),
            start,
            end,
            all_day: false,
            attendees: vec![],
            reminders: vec![],
            status: EventStatus::Confirmed,
            last_modified: Utc::now(),
            html_link: None,
        };

        app.add_event(event);
    }
}

fn render_event_form(f: &mut Frame, app: &AppState) {
    use gcal_imp::app::FormField;
    use ratatui::widgets::Clear;

    let Some(form) = &app.event_form else {
        return;
    };

    let area = f.size();
    let form_width = 70;
    let form_height = if form.all_day { 14 } else { 18 };
    let x = (area.width.saturating_sub(form_width)) / 2;
    let y = (area.height.saturating_sub(form_height)) / 2;

    let form_area = ratatui::layout::Rect {
        x,
        y,
        width: form_width,
        height: form_height,
    };

    f.render_widget(Clear, form_area);

    let active_color = app.theme.selected_bg;
    let inactive_color = Color::DarkGray;

    let form_title = if form.is_editing() { "Edit Event" } else { "Create New Event" };

    let mut form_text = vec![
        Line::from(vec![Span::styled(form_title, Style::default().fg(app.theme.title).add_modifier(Modifier::BOLD))]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Title: ", Style::default().fg(if form.active_field == FormField::Title { active_color } else { inactive_color })),
            Span::raw(&form.title),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Date: ", Style::default().fg(inactive_color)),
            Span::raw(form.date.format("%Y-%m-%d").to_string()),
        ]),
        Line::from(""),
    ];

    if !form.all_day {
        form_text.extend(vec![
            Line::from(vec![
                Span::styled("Start Time: ", Style::default().fg(if form.active_field == FormField::StartTime { active_color } else { inactive_color })),
                Span::raw(&form.time_input_buffer),
                Span::styled(if form.active_field == FormField::StartTime {
                    if form.time_buffer_touched {
                        " (HH:MM or HHMM)"
                    } else {
                        " [type to replace]"
                    }
                } else { "" }, Style::default().fg(Color::DarkGray)),
            ]),
            Line::from(""),
        ]);
    }

    let duration_label = if form.all_day { "Duration (days): " } else { "Duration (min): " };
    form_text.extend(vec![
        Line::from(vec![
            Span::styled(duration_label, Style::default().fg(if form.active_field == FormField::Duration { active_color } else { inactive_color })),
            Span::raw(&form.duration_input_buffer),
            Span::styled(if form.active_field == FormField::Duration {
                if form.duration_buffer_touched {
                    ""
                } else {
                    " [type to replace]"
                }
            } else { "" }, Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Location: ", Style::default().fg(if form.active_field == FormField::Location { active_color } else { inactive_color })),
            Span::raw(&form.location),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Description: ", Style::default().fg(if form.active_field == FormField::Description { active_color } else { inactive_color })),
            Span::raw(&form.description),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Tab", Style::default().fg(Color::Cyan)),
            Span::raw(" = Next field | "),
            Span::styled("Enter", Style::default().fg(Color::Green)),
            Span::raw(" = Save | "),
            Span::styled("Esc", Style::default().fg(Color::Red)),
            Span::raw(" = Cancel"),
        ]),
    ]);

    let block_title = if form.is_editing() { " Edit Event " } else { " New Event " };

    let form_paragraph = Paragraph::new(form_text)
        .block(Block::default()
            .borders(Borders::ALL)
            .title(block_title)
            .style(Style::default().bg(Color::Black)))
        .alignment(Alignment::Left);

    f.render_widget(form_paragraph, form_area);
}

fn render_delete_confirmation(f: &mut Frame, app: &AppState) {
    use ratatui::widgets::Clear;

    let Some(event_id) = &app.delete_confirmation_event_id else {
        return;
    };

    let event = app.events.get(event_id);
    let event_title = event.map(|e| e.title.as_str()).unwrap_or("this event");

    let area = f.size();
    let dialog_width = 60;
    let dialog_height = 10;
    let x = (area.width.saturating_sub(dialog_width)) / 2;
    let y = (area.height.saturating_sub(dialog_height)) / 2;

    let dialog_area = ratatui::layout::Rect {
        x,
        y,
        width: dialog_width,
        height: dialog_height,
    };

    f.render_widget(Clear, dialog_area);

    let dialog_text = vec![
        Line::from(vec![Span::styled("Delete Event?", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))]),
        Line::from(""),
        Line::from(vec![
            Span::raw("Are you sure you want to delete "),
            Span::styled(event_title, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw("?"),
        ]),
        Line::from(""),
        Line::from("This action cannot be undone."),
        Line::from(""),
        Line::from(vec![
            Span::styled("Y", Style::default().fg(Color::Green)),
            Span::raw(" = Yes, delete | "),
            Span::styled("N", Style::default().fg(Color::Red)),
            Span::raw(" = No, cancel"),
        ]),
    ];

    let dialog_paragraph = Paragraph::new(dialog_text)
        .block(Block::default()
            .borders(Borders::ALL)
            .title(" Confirm Delete ")
            .style(Style::default().bg(Color::Black)))
        .alignment(Alignment::Center);

    f.render_widget(dialog_paragraph, dialog_area);
}

fn strip_html(html: &str) -> String {
    let normalized = expand_anchor_tags(html);
    html2text::from_read(normalized.as_bytes(), 1000)
}

fn expand_anchor_tags(html: &str) -> String {
    static LINK_RE: OnceLock<regex::Regex> = OnceLock::new();
    let regex = LINK_RE.get_or_init(|| {
        regex::Regex::new(r#"(?is)<a\s+[^>]*?href=["']([^"']+)["'][^>]*>(.*?)</a>"#)
            .expect("invalid anchor regex")
    });

    regex
        .replace_all(html, |caps: &regex::Captures| {
            let url = caps.get(1).map(|m| m.as_str()).unwrap_or_default().trim();
            let text = caps.get(2).map(|m| m.as_str()).unwrap_or_default().trim();

            if text.is_empty() {
                url.to_string()
            } else if url.eq_ignore_ascii_case(text) {
                url.to_string()
            } else {
                format!("{text} ({url})")
            }
        })
        .into_owned()
}

fn copy_to_clipboard(text: &str) -> Result<(), String> {
    match arboard::Clipboard::new() {
        Ok(mut clipboard) => {
            match clipboard.set_text(text) {
                Ok(_) => {
                    tracing::info!("Successfully copied to clipboard (cross-platform)");
                    Ok(())
                }
                Err(e) => {
                    let err = format!("Failed to set clipboard text: {}", e);
                    tracing::error!("{}", err);
                    Err(err)
                }
            }
        }
        Err(e) => {
            let err = format!("Failed to access clipboard: {}", e);
            tracing::error!("{}", err);
            Err(err)
        }
    }
}

fn build_event_detail_lines(event: &CalendarEvent) -> Vec<String> {
    let mut lines = Vec::new();

    lines.push(event.title.clone());
    lines.push(String::new());

    if event.all_day {
        lines.push(format!("📅 {}", event.start.format("%A, %B %d, %Y")));
    } else {
        lines.push(format!("📅 {} at {}", event.start.format("%A, %B %d, %Y"), event.start.format("%H:%M")));
    }

    if event.all_day {
        let duration_days = (event.end - event.start).num_days();
        if duration_days > 1 {
            lines.push(format!("⏱  {} days", duration_days));
        }
    } else {
        let duration = event.duration_minutes();
        if duration >= 60 {
            lines.push(format!("⏱  {} hour{} {} min", duration / 60, if duration / 60 > 1 { "s" } else { "" }, duration % 60));
        } else {
            lines.push(format!("⏱  {} minutes", duration));
        }
    }

    if let Some(location) = &event.location {
        lines.push(String::new());
        lines.push("📍 Location:".to_string());
        lines.push(format!("   {}", location));
    }

    if let Some(description) = &event.description {
        lines.push(String::new());
        lines.push("📝 Description:".to_string());
        lines.push(String::new());
        let clean_description = strip_html(description);
        for line in clean_description.lines() {
            lines.push(line.to_string());
        }
    }

    if !event.attendees.is_empty() {
        lines.push(String::new());
        lines.push("👥 Attendees:".to_string());
        for attendee in &event.attendees {
            lines.push(format!("   • {}", attendee));
        }
    }

    lines.push(String::new());
    lines.push("hjkl = Move | wbe = Word | 0^$ = Line | gG = Top/Bottom".to_string());
    lines.push("o = Open URL | y = Yank line | B = Browser | E = Edit | q/Esc = Close".to_string());

    lines
}

fn next_word_position(lines: &[String], line_idx: usize, col: usize) -> (usize, usize) {
    if lines.is_empty() {
        return (0, 0);
    }

    let mut line_idx = line_idx.min(lines.len() - 1);
    let mut col = col;

    loop {
        if line_idx >= lines.len() {
            let last_idx = lines.len() - 1;
            return (last_idx, last_char_index(&lines[last_idx]));
        }

        let line_text = &lines[line_idx];
        let len = line_text.chars().count();

        if len == 0 || col >= len {
            if line_idx + 1 >= lines.len() {
                return (line_idx, 0);
            }
            line_idx += 1;
            let next_line = &lines[line_idx];
            if let Some(first_non_ws) = next_line.chars().position(|c| !c.is_whitespace()) {
                return (line_idx, first_non_ws);
            } else {
                col = 0;
                continue;
            }
        }

        if let Some(next_col) = find_next_word_start(line_text, col) {
            return (line_idx, next_col);
        }

        if line_idx + 1 >= lines.len() {
            return (line_idx, last_char_index(line_text));
        }

        line_idx += 1;
        let next_line = &lines[line_idx];
        if let Some(first_non_ws) = next_line.chars().position(|c| !c.is_whitespace()) {
            return (line_idx, first_non_ws);
        } else {
            col = 0;
        }
    }
}

fn word_end_position(lines: &[String], line_idx: usize, col: usize) -> (usize, usize) {
    if lines.is_empty() {
        return (0, 0);
    }

    let mut line_idx = line_idx.min(lines.len() - 1);
    let mut col = col;

    loop {
        if line_idx >= lines.len() {
            let last_idx = lines.len() - 1;
            return (last_idx, last_char_index(&lines[last_idx]));
        }

        let line_text = &lines[line_idx];
        let len = line_text.chars().count();

        if len == 0 || col >= len {
            if line_idx + 1 >= lines.len() {
                return (line_idx, 0);
            }
            line_idx += 1;
            col = 0;
            continue;
        }

        if let Some(end_col) = find_word_end(line_text, col) {
            return (line_idx, end_col);
        }

        if line_idx + 1 >= lines.len() {
            return (line_idx, last_char_index(line_text));
        }

        line_idx += 1;
        col = 0;
    }
}

fn prev_word_position(lines: &[String], line_idx: usize, col: usize) -> (usize, usize) {
    if lines.is_empty() {
        return (0, 0);
    }

    let mut line_idx = line_idx.min(lines.len() - 1);
    let mut col = col;

    loop {
        if line_idx >= lines.len() {
            line_idx = lines.len() - 1;
            col = lines[line_idx].chars().count();
        }

        let line_text = &lines[line_idx];
        let len = line_text.chars().count();

        let safe_col = if len == 0 { 0 } else { col.min(len) };

        if let Some(prev_col) = find_prev_word_start(line_text, safe_col) {
            return (line_idx, prev_col);
        }

        if line_idx == 0 {
            return (0, 0);
        }

        line_idx -= 1;
        col = lines[line_idx].chars().count();
    }
}

fn last_char_index(text: &str) -> usize {
    text.chars().count().saturating_sub(1)
}

fn is_word_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

fn find_next_word_start(text: &str, current_col: usize) -> Option<usize> {
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() {
        return None;
    }

    let len = chars.len();
    let mut pos = current_col.min(len.saturating_sub(1));

    if chars[pos].is_whitespace() {
        while pos < len && chars[pos].is_whitespace() {
            pos += 1;
        }
    } else if is_word_char(chars[pos]) {
        while pos < len && is_word_char(chars[pos]) {
            pos += 1;
        }
    } else {
        while pos < len && !chars[pos].is_whitespace() && !is_word_char(chars[pos]) {
            pos += 1;
        }
    }

    while pos < len && chars[pos].is_whitespace() {
        pos += 1;
    }

    if pos < len {
        Some(pos)
    } else {
        None
    }
}

fn find_word_end(text: &str, current_col: usize) -> Option<usize> {
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() {
        return None;
    }

    let len = chars.len();
    let mut pos = current_col.saturating_add(1);

    while pos < len && chars[pos].is_whitespace() {
        pos += 1;
    }

    if pos >= len {
        return None;
    }

    let is_word = is_word_char(chars[pos]);

    while pos < len && !chars[pos].is_whitespace() {
        if is_word_char(chars[pos]) != is_word {
            break;
        }
        pos += 1;
    }

    Some(pos.saturating_sub(1).min(len.saturating_sub(1)))
}

fn find_prev_word_start(text: &str, current_col: usize) -> Option<usize> {
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() {
        return None;
    }

    let len = chars.len();
    let mut pos = current_col.min(len);

    if pos == 0 {
        return None;
    }

    pos -= 1;

    while pos > 0 && chars[pos].is_whitespace() {
        pos -= 1;
    }

    if chars[pos].is_whitespace() {
        return None;
    }

    let is_word = is_word_char(chars[pos]);

    while pos > 0 {
        let prev = chars[pos - 1];
        if prev.is_whitespace() {
            break;
        }
        let prev_is_word = is_word_char(prev);
        if prev_is_word != is_word {
            break;
        }
        pos -= 1;
    }

    Some(pos)
}

fn find_first_non_whitespace(text: &str) -> usize {
    let chars: Vec<char> = text.chars().collect();
    for (i, ch) in chars.iter().enumerate() {
        if !ch.is_whitespace() {
            return i;
        }
    }
    0
}

#[cfg(test)]
mod detail_cursor_tests {
    use super::*;

    fn sample_lines() -> Vec<String> {
        vec![
            "alpha beta".to_string(),
            "gamma delta".to_string(),
        ]
    }

    #[test]
    fn next_word_wraps_to_next_line() {
        let lines = sample_lines();
        let (line, col) = next_word_position(
            &lines,
            0,
            lines[0].chars().count().saturating_sub(1),
        );

        assert_eq!(line, 1);
        assert_eq!(col, 0);
    }

    #[test]
    fn prev_word_moves_to_previous_line() {
        let lines = sample_lines();
        let (line, col) = prev_word_position(&lines, 1, 0);

        assert_eq!(line, 0);
        assert_eq!(col, 6);
    }

    #[test]
    fn word_end_wraps_to_next_line() {
        let lines = sample_lines();
        let (line, col) = word_end_position(
            &lines,
            0,
            lines[0].chars().count().saturating_sub(1),
        );

        assert_eq!(line, 1);
        assert_eq!(col, 4);
    }
}

#[cfg(test)]
mod html_link_tests {
    use super::*;

    #[test]
    fn anchor_tags_include_url_after_strip() {
        let html = r#"<p>Visit <a href="https://example.com">Example</a> now.</p>"#;
        let text = strip_html(html);
        assert!(text.contains("Example (https://example.com)"));
    }

    #[test]
    fn anchor_without_text_falls_back_to_url() {
        let html = r#"<a href="https://example.com"></a>"#;
        let text = strip_html(html);
        assert!(text.contains("https://example.com"));
    }
}

fn render_event_detail(f: &mut Frame, app: &AppState) {
    use ratatui::widgets::Clear;

    let Some(event_id) = &app.detail_view_event_id else {
        return;
    };

    tracing::info!("render_event_detail: event_id={}", event_id);

    let Some(event) = app.events.get(event_id) else {
        tracing::error!("render_event_detail: event not found for id={}", event_id);
        return;
    };

    tracing::info!("render_event_detail: title={}, has_description={}", event.title, event.description.is_some());

    let area = f.size();
    let panel_width = (area.width as f32 * 0.7) as u16;
    let panel_height = (area.height as f32 * 0.8) as u16;
    let x = (area.width.saturating_sub(panel_width)) / 2;
    let y = (area.height.saturating_sub(panel_height)) / 2;

    let panel_area = ratatui::layout::Rect {
        x,
        y,
        width: panel_width,
        height: panel_height,
    };

    f.render_widget(Clear, panel_area);

    let mut lines = vec![
        Line::from(vec![Span::styled(&event.title, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))]),
        Line::from(""),
    ];

    let start_str = if event.all_day {
        format!("📅 {}", event.start.format("%A, %B %d, %Y"))
    } else {
        format!("📅 {} at {}", event.start.format("%A, %B %d, %Y"), event.start.format("%H:%M"))
    };
    lines.push(Line::from(vec![Span::styled(start_str, Style::default().fg(Color::Green))]));

    if event.all_day {
        let duration_days = (event.end - event.start).num_days();
        if duration_days > 1 {
            lines.push(Line::from(vec![
                Span::styled(format!("⏱  {} days", duration_days), Style::default().fg(Color::Yellow))
            ]));
        }
    } else {
        let duration = event.duration_minutes();
        let duration_str = if duration >= 60 {
            format!("⏱  {} hour{} {} min", duration / 60, if duration / 60 > 1 { "s" } else { "" }, duration % 60)
        } else {
            format!("⏱  {} minutes", duration)
        };
        lines.push(Line::from(vec![Span::styled(duration_str, Style::default().fg(Color::Yellow))]));
    }

    if let Some(location) = &event.location {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("📍 Location:", Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD))
        ]));
        lines.push(Line::from(vec![Span::raw(format!("   {}", location))]));
    }

    if let Some(description) = &event.description {
        tracing::info!("render_event_detail: processing description, length={}", description.len());
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("📝 Description:", Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD))
        ]));
        lines.push(Line::from(""));

        let clean_description = strip_html(description);
        tracing::info!("render_event_detail: cleaned description, length={}", clean_description.len());
        tracing::info!("render_event_detail: about to iterate through description lines");

        for (line_num, line) in clean_description.lines().enumerate() {
            tracing::info!("render_event_detail: START processing line {}", line_num);
            let line_owned = line.to_string();
            tracing::info!("render_event_detail: line {} length={}, content_preview={}", line_num, line_owned.len(), &line_owned.chars().take(50).collect::<String>());
            if line_owned.trim().is_empty() {
                lines.push(Line::from(""));
            } else {
                let markdown_link_pattern = regex::Regex::new(r"\[([^\]]+)\]\((https?://[^\)]+)\)").unwrap();
                let plain_url_pattern = regex::Regex::new(r"(https?://[^\s\)]+)").unwrap();
                let mut spans = Vec::new();
                let mut last_end = 0;

                tracing::info!("render_event_detail: searching for markdown links in line {}", line_num);
                let markdown_captures: Vec<_> = markdown_link_pattern.captures_iter(&line_owned).collect();
                tracing::info!("render_event_detail: found {} markdown link matches", markdown_captures.len());
                for cap in markdown_captures {
                    let m = cap.get(0).unwrap();
                    tracing::info!("render_event_detail: markdown link match at {}..{}", m.start(), m.end());
                    if m.start() > last_end {
                        tracing::info!("render_event_detail: adding text span from {}..{}", last_end, m.start());
                        if m.start() <= line_owned.len() && last_end <= m.start() {
                            spans.push(Span::raw(line_owned[last_end..m.start()].to_string()));
                        } else {
                            tracing::error!("render_event_detail: invalid slice range {}..{} for line length {}", last_end, m.start(), line_owned.len());
                        }
                    }
                    spans.push(Span::styled(m.as_str().to_string(), Style::default().fg(Color::Blue).add_modifier(Modifier::UNDERLINED)));
                    last_end = m.end();
                }

                tracing::info!("render_event_detail: searching for plain URLs from offset {}, line length={}", last_end, line_owned.len());
                if last_end > line_owned.len() {
                    tracing::error!("render_event_detail: last_end {} exceeds line length {}", last_end, line_owned.len());
                    last_end = line_owned.len();
                }
                tracing::info!("render_event_detail: about to iterate plain URL captures");
                let plain_captures: Vec<_> = plain_url_pattern.captures_iter(&line_owned[last_end..]).collect();
                tracing::info!("render_event_detail: found {} plain URL matches", plain_captures.len());
                for (idx, cap) in plain_captures.iter().enumerate() {
                    tracing::info!("render_event_detail: processing plain URL match {}", idx);
                    let m = cap.get(0).unwrap();
                    let abs_start = last_end + m.start();
                    let abs_end = last_end + m.end();
                    tracing::info!("render_event_detail: plain URL match at {}..{} (abs {}..{})", m.start(), m.end(), abs_start, abs_end);
                    if abs_start > last_end {
                        tracing::info!("render_event_detail: adding text before URL from {}..{}", last_end, abs_start);
                        if abs_start <= line_owned.len() && last_end <= abs_start {
                            spans.push(Span::raw(line_owned[last_end..abs_start].to_string()));
                        } else {
                            tracing::error!("render_event_detail: invalid slice {}..{} for line length {}", last_end, abs_start, line_owned.len());
                        }
                    }
                    tracing::info!("render_event_detail: adding URL span");
                    spans.push(Span::styled(m.as_str().to_string(), Style::default().fg(Color::Blue).add_modifier(Modifier::UNDERLINED)));
                    last_end = abs_end;
                    tracing::info!("render_event_detail: completed processing URL match {}", idx);
                }

                if last_end < line_owned.len() {
                    tracing::debug!("render_event_detail: adding remaining text from {}..{}", last_end, line_owned.len());
                    spans.push(Span::raw(line_owned[last_end..].to_string()));
                }

                if spans.is_empty() {
                    spans.push(Span::raw(line_owned));
                }

                lines.push(Line::from(spans));
            }
        }
    }

    if !event.attendees.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("👥 Attendees:", Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD))
        ]));
        for attendee in &event.attendees {
            lines.push(Line::from(vec![Span::raw(format!("   • {}", attendee))]));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("hjkl", Style::default().fg(Color::Cyan)),
        Span::raw(" = Move | "),
        Span::styled("wbe", Style::default().fg(Color::Cyan)),
        Span::raw(" = Word | "),
        Span::styled("0^$", Style::default().fg(Color::Cyan)),
        Span::raw(" = Line | "),
        Span::styled("gG", Style::default().fg(Color::Cyan)),
        Span::raw(" = Top/Bottom"),
    ]));
    lines.push(Line::from(vec![
        Span::styled("o", Style::default().fg(Color::Magenta)),
        Span::raw(" = Open URL | "),
        Span::styled("y", Style::default().fg(Color::Yellow)),
        Span::raw(" = Yank line | "),
        Span::styled("B", Style::default().fg(Color::Blue)),
        Span::raw(" = Browser | "),
        Span::styled("E", Style::default().fg(Color::Green)),
        Span::raw(" = Edit | "),
        Span::styled("q", Style::default().fg(Color::Red)),
        Span::raw("/"),
        Span::styled("Esc", Style::default().fg(Color::Red)),
        Span::raw(" = Close"),
    ]));

    let visible_lines_count = panel_height.saturating_sub(2) as usize;
    let total_lines = lines.len();

    tracing::info!("render_event_detail: panel_height={}, visible_lines_count={}, total_lines={}", panel_height, visible_lines_count, total_lines);

    let cursor_line = app.detail_view_cursor_line.min(total_lines.saturating_sub(1));
    let cursor_col = app.detail_view_cursor_col;

    tracing::info!("render_event_detail: cursor_line={}, cursor_col={}", cursor_line, cursor_col);

    let scroll_start = if visible_lines_count == 0 {
        0
    } else if cursor_line >= app.detail_view_scroll + visible_lines_count {
        cursor_line.saturating_sub(visible_lines_count - 1)
    } else if cursor_line < app.detail_view_scroll {
        cursor_line
    } else {
        app.detail_view_scroll
    }.min(total_lines.saturating_sub(visible_lines_count));

    tracing::info!("render_event_detail: scroll_start={}", scroll_start);

    let (visual_start, visual_end) = if let Some((start_line, start_col)) = app.detail_view_visual_start {
        let end_line = cursor_line;
        let end_col = cursor_col;
        if start_line < end_line || (start_line == end_line && start_col <= end_col) {
            (Some((start_line, start_col)), Some((end_line, end_col)))
        } else {
            (Some((end_line, end_col)), Some((start_line, start_col)))
        }
    } else {
        (None, None)
    };

    let lines_with_cursor: Vec<Line> = lines.into_iter()
        .enumerate()
        .skip(scroll_start)
        .take(visible_lines_count)
        .map(|(line_idx, line)| {
            let mut new_spans = Vec::new();
            let mut char_count = 0;

            for span in line.spans {
                let text = span.content.to_string();
                let chars: Vec<char> = text.chars().collect();

                for (char_idx, ch) in chars.iter().enumerate() {
                    let global_char_idx = char_count + char_idx;
                    let is_cursor = line_idx == cursor_line && global_char_idx == cursor_col;
                    let is_in_visual = if let (Some((vs_line, vs_col)), Some((ve_line, ve_col))) = (visual_start, visual_end) {
                        if line_idx > vs_line && line_idx < ve_line {
                            true
                        } else if line_idx == vs_line && line_idx == ve_line {
                            global_char_idx >= vs_col && global_char_idx <= ve_col
                        } else if line_idx == vs_line {
                            global_char_idx >= vs_col
                        } else if line_idx == ve_line {
                            global_char_idx <= ve_col
                        } else {
                            false
                        }
                    } else {
                        false
                    };

                    let style = if is_cursor {
                        span.style.bg(Color::White).fg(Color::Black)
                    } else if is_in_visual {
                        span.style.bg(Color::DarkGray).fg(Color::White)
                    } else {
                        span.style
                    };

                    new_spans.push(Span::styled(ch.to_string(), style));
                }

                char_count += chars.len();
            }

            if line_idx == cursor_line && (new_spans.is_empty() || char_count <= cursor_col) {
                new_spans.push(Span::styled(" ", Style::default().bg(Color::White).fg(Color::Black)));
            }

            Line::from(new_spans)
        })
        .collect();

    let detail_paragraph = Paragraph::new(lines_with_cursor)
        .block(Block::default()
            .borders(Borders::ALL)
            .title(" Event Details ")
            .style(Style::default().bg(Color::Black)))
        .alignment(Alignment::Left)
        .wrap(ratatui::widgets::Wrap { trim: false });

    f.render_widget(detail_paragraph, panel_area);
}
