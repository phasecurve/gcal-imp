use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use chrono::{Datelike, NaiveDate};
use gcal_imp::{
    app::{AppState, Mode},
    ui::month_view,
};

pub fn render(f: &mut Frame, app: &AppState, area: ratatui::layout::Rect) {
    let layout = month_view::calculate_layout(app);

    let month_name = NaiveDate::from_ymd_opt(layout.year, layout.month, 1)
        .map(|d| d.format("%B %Y").to_string())
        .unwrap_or_else(|| format!("{}-{:02}", layout.year, layout.month));

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
