use std::io;
use std::sync::OnceLock;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as TermEvent, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};
use regex::Regex;
use uuid::Uuid;
use gcal_imp::{
    app::{AppState, Mode, SyncStatus, EventForm},
    storage::config::Config,
    sync::sync_engine::SyncEngine,
    ui::theme::Theme,
    input::{normal_mode, command_mode, insert_mode, visual_mode},
    calendar::{Event as CalendarEvent, EventStatus, DEFAULT_CALENDAR_ID},
};
use crate::tui::{
    presentation::ui,
    sample_events::add_sample_events,
    event_detail::{
        presentation::refresh_detail_view_lines,
        navigation::{next_word_position, prev_word_position, word_end_position, last_char_index, find_first_non_whitespace},
        text_selection::{copy_to_clipboard, paste_from_clipboard},
        content_formatting::strip_html,
    },
};

fn build_event_from_form(
    id: String,
    form: &EventForm,
    start: chrono::DateTime<chrono::Utc>,
    end: chrono::DateTime<chrono::Utc>,
    all_day: bool,
    html_link: Option<String>,
) -> CalendarEvent {
    CalendarEvent {
        id,
        calendar_id: DEFAULT_CALENDAR_ID.to_string(),
        title: form.title.clone(),
        description: (!form.description.is_empty()).then(|| form.description.clone()),
        location: (!form.location.is_empty()).then(|| form.location.clone()),
        start,
        end,
        all_day,
        attendees: vec![],
        reminders: vec![],
        status: EventStatus::Confirmed,
        last_modified: chrono::Utc::now(),
        html_link,
    }
}

pub async fn run_tui(sample: bool) -> Result<(), io::Error> {
    let config = Config::load_or_create()
        .map_err(|e| io::Error::other(e.to_string()))?;

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

    if sample {
        add_sample_events(&mut app);
    }

    match sync_engine.fetch_events_around_date(app.selected_date).await {
        Ok(events) => {
            for event in events {
                app.add_event(event);
            }
            app.sync_status = SyncStatus::Synced;
        }
        Err(e) => {
            app.sync_status = SyncStatus::Error(format!("Sync failed: {}", e));
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

        if let TermEvent::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            match app.mode {
                Mode::Normal => {
                    if app.show_help {
                        handle_help_keys(key.code, app);
                    } else if app.detail_view_event_id.is_some() {
                        if handle_detail_view_keys(key.code, app)? {
                            return Ok(());
                        }
                    } else {
                        match key.code {
                            KeyCode::Char('q') => return Ok(()),
                            _ => normal_mode::handle_key(key.code, app),
                        }
                    }
                }
                Mode::Command => {
                    if handle_command_mode(key.code, app, terminal, &mut sync_engine).await? {
                        return Ok(());
                    }
                }
                Mode::Insert => {
                    if handle_insert_mode(key.code, app, terminal, &mut sync_engine).await? {
                        return Ok(());
                    }
                }
                Mode::Visual => {
                    if app.delete_confirmation_event_id.is_some() {
                        handle_delete_confirmation(key.code, app, terminal, &mut sync_engine).await?;
                    } else {
                        visual_mode::handle_key(key.code, app);
                    }
                }
            }
        }
    }
}

fn handle_help_keys(code: KeyCode, app: &mut AppState) {
    match code {
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
}

fn handle_detail_view_keys(code: KeyCode, app: &mut AppState) -> io::Result<bool> {
    match code {
        KeyCode::Esc => {
            app.detail_view_event_id = None;
            app.detail_view_scroll = 0;
            app.detail_view_cursor_line = 0;
            app.detail_view_cursor_col = 0;
            Ok(false)
        }
        KeyCode::Char('j') => {
            app.detail_view_cursor_line = app.detail_view_cursor_line.saturating_add(1);
            Ok(false)
        }
        KeyCode::Char('k') => {
            app.detail_view_cursor_line = app.detail_view_cursor_line.saturating_sub(1);
            Ok(false)
        }
        KeyCode::Char('h') => {
            app.detail_view_cursor_col = app.detail_view_cursor_col.saturating_sub(1);
            Ok(false)
        }
        KeyCode::Char('l') => {
            app.detail_view_cursor_col = app.detail_view_cursor_col.saturating_add(1);
            Ok(false)
        }
        KeyCode::Char('0') => {
            app.detail_view_cursor_col = 0;
            Ok(false)
        }
        KeyCode::Char('$') => {
            if let Some(line_text) = app.detail_view_line_text.get(app.detail_view_cursor_line) {
                app.detail_view_cursor_col = last_char_index(line_text);
            } else {
                app.detail_view_cursor_col = 0;
            }
            Ok(false)
        }
        KeyCode::Char('^') => {
            if let Some(line_text) = app.detail_view_line_text.get(app.detail_view_cursor_line) {
                app.detail_view_cursor_col = find_first_non_whitespace(line_text);
            }
            Ok(false)
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
            Ok(false)
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
            Ok(false)
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
            Ok(false)
        }
        KeyCode::Char('g') => {
            app.detail_view_cursor_line = 0;
            app.detail_view_scroll = 0;
            Ok(false)
        }
        KeyCode::Char('G') => {
            app.detail_view_cursor_line = 999;
            Ok(false)
        }
        KeyCode::Char('E') => {
            if let Some(event_id) = &app.detail_view_event_id
                && let Some(event) = app.events.get(event_id).cloned()
            {
                app.event_form = Some(EventForm::for_event(&event));
                app.mode = Mode::Insert;
                app.detail_view_event_id = None;
                app.detail_view_scroll = 0;
                app.detail_view_cursor_line = 0;
                app.detail_view_cursor_col = 0;
            }
            Ok(false)
        }
        KeyCode::Char('a') => {
            app.event_form = Some(EventForm::new(app.selected_date, String::new()));
            app.mode = Mode::Insert;
            app.detail_view_event_id = None;
            app.detail_view_scroll = 0;
            app.detail_view_cursor_line = 0;
            app.detail_view_cursor_col = 0;
            app.detail_view_visual_start = None;
            Ok(false)
        }
        KeyCode::Char('o') => {
            handle_open_url(app);
            Ok(false)
        }
        KeyCode::Char('y') => {
            handle_yank(app);
            Ok(false)
        }
        KeyCode::Char('p') => {
            handle_paste(app);
            Ok(false)
        }
        KeyCode::Char('q') => Ok(true),
        KeyCode::Char('v') => {
            if app.detail_view_visual_start.is_some() {
                app.detail_view_visual_start = None;
            } else {
                app.detail_view_visual_start = Some((app.detail_view_cursor_line, app.detail_view_cursor_col));
            }
            Ok(false)
        }
        KeyCode::Char('B') => {
            handle_open_browser(app);
            Ok(false)
        }
        _ => Ok(false)
    }
}

fn handle_open_url(app: &AppState) {
    tracing::info!("Attempting to open URL at cursor position");
    if let Some(event_id) = &app.detail_view_event_id
        && let Some(event) = app.events.get(event_id)
    {
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
            all_lines.extend([String::new(), "📍 Location:".to_string(), format!("   {}", location)]);
        }

        if let Some(desc) = &event.description {
            all_lines.extend([String::new(), "📝 Description:".to_string(), String::new()]);
            all_lines.extend(strip_html(desc).lines().map(String::from));
        }

        if !event.attendees.is_empty() {
            all_lines.extend([String::new(), "👥 Attendees:".to_string()]);
            all_lines.extend(event.attendees.iter().map(|a| format!("   • {}", a)));
        }

        if app.detail_view_cursor_line < all_lines.len() {
            let line_text = &all_lines[app.detail_view_cursor_line];

            static MARKDOWN_LINK_RE: OnceLock<Regex> = OnceLock::new();
            static PLAIN_URL_RE: OnceLock<Regex> = OnceLock::new();

            let markdown_link_pattern = MARKDOWN_LINK_RE.get_or_init(|| {
                Regex::new(r"\[([^\]]+)\]\((https?://[^\)]+)\)")
                    .expect("invalid markdown link regex")
            });
            let plain_url_pattern = PLAIN_URL_RE.get_or_init(|| {
                Regex::new(r"(https?://[^\s\)]+)")
                    .expect("invalid plain url regex")
            });

            let url_to_open = markdown_link_pattern.captures(line_text)
                .and_then(|cap| cap.get(2))
                .or_else(|| plain_url_pattern.captures(line_text).and_then(|cap| cap.get(1)))
                .map(|m| m.as_str());

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

fn handle_yank(app: &mut AppState) {
    if !app.detail_view_line_text.is_empty() {
        let lines = &app.detail_view_line_text;

        let text_to_yank = if let Some((start_line, start_col)) = app.detail_view_visual_start {
            let end_line = app.detail_view_cursor_line;
            let end_col = app.detail_view_cursor_col;

            tracing::info!("Visual selection: start=({}, {}), end=({}, {})", start_line, start_col, end_line, end_col);

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
            tracing::info!("Yanking {} bytes: '{}'", text_to_yank.len(), text_to_yank);
            if let Err(e) = copy_to_clipboard(&text_to_yank) {
                tracing::error!("Clipboard yank failed: {}", e);
            }
        }
    }
}

fn handle_paste(_app: &AppState) {
    match paste_from_clipboard() {
        Ok(text) => {
            tracing::info!("Clipboard contains {} bytes: '{}'", text.len(), text);
        }
        Err(e) => {
            tracing::error!("Clipboard paste failed: {}", e);
        }
    }
}

fn handle_open_browser(app: &AppState) {
    tracing::info!("Opening event in browser");
    if let Some(event_id) = &app.detail_view_event_id
        && let Some(event) = app.events.get(event_id)
    {
        let url = event.html_link.clone()
            .unwrap_or_else(|| format!("https://calendar.google.com/calendar/u/0/r/eventedit/{}", event.id));
        tracing::info!("Opening Google Calendar URL: {}", url);
        match std::process::Command::new("xdg-open").arg(&url).spawn() {
            Ok(_) => tracing::info!("Successfully launched browser"),
            Err(e) => tracing::error!("Failed to open browser: {}", e),
        }
    }
}

async fn handle_command_mode<B: ratatui::backend::Backend>(
    code: KeyCode,
    app: &mut AppState,
    terminal: &mut Terminal<B>,
    sync_engine: &mut SyncEngine,
) -> io::Result<bool> {
    match code {
        KeyCode::Enter => {
            let command_text = app.command_buffer.clone();
            let cmd = command_mode::parse_command(&command_text);

            match cmd {
                command_mode::Command::Quit => return Ok(true),
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
            Ok(false)
        }
        KeyCode::Esc => {
            app.command_buffer.clear();
            app.mode = Mode::Normal;
            Ok(false)
        }
        KeyCode::Backspace => {
            app.command_buffer.pop();
            Ok(false)
        }
        KeyCode::Char(c) => {
            app.command_buffer.push(c);
            Ok(false)
        }
        _ => Ok(false)
    }
}

async fn handle_insert_mode<B: ratatui::backend::Backend>(
    code: KeyCode,
    app: &mut AppState,
    terminal: &mut Terminal<B>,
    sync_engine: &mut SyncEngine,
) -> io::Result<bool> {
    match code {
        KeyCode::Esc => {
            app.event_form = None;
            app.mode = Mode::Normal;
            Ok(false)
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

                let is_update = form.event_id.is_some();
                let (event_id, html_link) = if let Some(id) = form.event_id.clone() {
                    let link = app.events.get(&id).and_then(|e| e.html_link.clone());
                    (id, link)
                } else {
                    (Uuid::new_v4().to_string(), None)
                };

                let event = build_event_from_form(
                    event_id,
                    &form,
                    start_datetime,
                    end_datetime,
                    all_day,
                    html_link,
                );

                app.sync_status = SyncStatus::Syncing;
                terminal.draw(|f| ui(f, app))?;

                if is_update {
                    match sync_engine.update_event(&event).await {
                        Ok(()) => {
                            app.add_event(event);
                            app.sync_status = SyncStatus::Synced;
                        }
                        Err(e) => {
                            app.sync_status = SyncStatus::Error(format!("Failed to update: {}", e));
                        }
                    }
                } else {
                    match sync_engine.create_event(&event).await {
                        Ok(created_info) => {
                            let mut created_event = event;
                            created_event.id = created_info.id;
                            created_event.html_link = created_info.html_link;
                            app.add_event(created_event);
                            app.sync_status = SyncStatus::Synced;
                        }
                        Err(e) => {
                            app.sync_status = SyncStatus::Error(format!("Failed to create: {}", e));
                        }
                    }
                }

                app.mode = Mode::Normal;
            }
            Ok(false)
        }
        _ => {
            insert_mode::handle_key(code, app);
            Ok(false)
        }
    }
}

async fn handle_delete_confirmation<B: ratatui::backend::Backend>(
    code: KeyCode,
    app: &mut AppState,
    terminal: &mut Terminal<B>,
    sync_engine: &mut SyncEngine,
) -> io::Result<()> {
    match code {
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
    Ok(())
}
