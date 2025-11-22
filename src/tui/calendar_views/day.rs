use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use gcal_imp::{
    app::AppState,
    ui::day_view,
};

pub fn render(f: &mut Frame, app: &AppState, area: ratatui::layout::Rect) {
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
