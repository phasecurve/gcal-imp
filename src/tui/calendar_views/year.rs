use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use gcal_imp::{
    app::AppState,
    ui::year_view,
};

pub fn render(f: &mut Frame, app: &AppState, area: ratatui::layout::Rect) {
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
                let weeks = (month.days.len() + month.first_weekday as usize).div_ceil(7);
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
