use ratatui::{
    layout::Alignment,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use gcal_imp::app::AppState;

pub fn render(f: &mut Frame, app: &AppState) {
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
