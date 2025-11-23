use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Modifier, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use gcal_imp::app::{AppState, ViewType, Mode};
use crate::tui::{calendar_views, dialogs, event_detail};

pub fn ui(f: &mut Frame, app: &AppState) {
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
        ViewType::Month => calendar_views::month::render(f, app, chunks[1]),
        ViewType::Week => calendar_views::week::render(f, app, chunks[1]),
        ViewType::Day => calendar_views::day::render(f, app, chunks[1]),
        ViewType::Year => calendar_views::year::render(f, app, chunks[1]),
    }

    calendar_views::event_list::render(f, app, chunks[2]);

    let status_text = if matches!(app.mode, Mode::Command) {
        app.command_buffer.to_string()
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
        dialogs::help::render(f, app);
    }

    if app.event_form.is_some() {
        dialogs::event_form::render(f, app);
    }

    if app.delete_confirmation_event_id.is_some() {
        dialogs::delete_confirmation::render(f, app);
    }

    if app.detail_view_event_id.is_some() {
        event_detail::presentation::render(f, app);
    }
}
