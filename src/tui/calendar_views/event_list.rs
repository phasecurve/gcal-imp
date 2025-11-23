use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use gcal_imp::app::AppState;

pub fn render(f: &mut Frame, app: &AppState, area: ratatui::layout::Rect) {
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
        let selected_base = Style::default().bg(app.theme.selected_bg).add_modifier(Modifier::BOLD);

        for (idx, event) in events.iter().enumerate() {
            let time_str = event.start.format("%H:%M").to_string();
            let is_selected = idx == app.selected_event_index;

            let (time_style, title_style) = if is_selected {
                (selected_base.fg(Color::Black), selected_base.fg(Color::Black))
            } else {
                (Style::default().fg(Color::Green), Style::default().fg(Color::White))
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
