use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use chrono::Datelike;
use gcal_imp::{
    app::AppState,
    ui::week_view,
};

pub fn render(f: &mut Frame, app: &AppState, area: ratatui::layout::Rect) {
    let layout = week_view::calculate_layout(app);

    let week_range = if let Some(last_day) = layout.days.last() {
        format!("{} - {}",
            layout.week_start.format("%b %d"),
            last_day.date.format("%b %d, %Y"))
    } else {
        layout.week_start.format("%b %d, %Y").to_string()
    };

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
