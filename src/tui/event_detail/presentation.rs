use std::sync::OnceLock;
use ratatui::{
    layout::Alignment,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use regex::Regex;
use gcal_imp::{app::AppState, calendar::Event as CalendarEvent};
use super::content_formatting::strip_html;

static MARKDOWN_LINK_RE: OnceLock<Regex> = OnceLock::new();
static PLAIN_URL_RE: OnceLock<Regex> = OnceLock::new();

fn markdown_link_pattern() -> &'static Regex {
    MARKDOWN_LINK_RE.get_or_init(|| {
        Regex::new(r"\[([^\]]+)\]\((https?://[^\)]+)\)")
            .expect("invalid markdown link regex")
    })
}

fn plain_url_pattern() -> &'static Regex {
    PLAIN_URL_RE.get_or_init(|| {
        Regex::new(r"(https?://[^\s\)]+)")
            .expect("invalid plain url regex")
    })
}

pub fn refresh_detail_view_lines(app: &mut AppState) {
    if let Some(event_id) = &app.detail_view_event_id {
        if let Some(event) = app.events.get(event_id) {
            app.detail_view_line_text = build_event_detail_lines(event);
        } else {
            app.detail_view_line_text.clear();
        }
    }
}

pub fn build_event_detail_lines(event: &CalendarEvent) -> Vec<String> {
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

pub fn render(f: &mut Frame, app: &AppState) {
    let Some(event_id) = &app.detail_view_event_id else {
        return;
    };

    let Some(event) = app.events.get(event_id) else {
        return;
    };

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
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("📝 Description:", Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD))
        ]));
        lines.push(Line::from(""));

        let clean_description = strip_html(description);

        for line in clean_description.lines() {
            let line_owned = line.to_string();
            if line_owned.trim().is_empty() {
                lines.push(Line::from(""));
            } else {
                let md_pattern = markdown_link_pattern();
                let url_pattern = plain_url_pattern();
                let mut spans = Vec::new();
                let mut last_end = 0;

                let markdown_captures: Vec<_> = md_pattern.captures_iter(&line_owned).collect();
                for cap in markdown_captures {
                    let Some(m) = cap.get(0) else { continue };
                    if m.start() > last_end && m.start() <= line_owned.len() {
                        spans.push(Span::raw(line_owned[last_end..m.start()].to_string()));
                    }
                    spans.push(Span::styled(m.as_str().to_string(), Style::default().fg(Color::Blue).add_modifier(Modifier::UNDERLINED)));
                    last_end = m.end();
                }

                if last_end > line_owned.len() {
                    last_end = line_owned.len();
                }
                let plain_captures: Vec<_> = url_pattern.captures_iter(&line_owned[last_end..]).collect();
                for cap in plain_captures.iter() {
                    let Some(m) = cap.get(0) else { continue };
                    let abs_start = last_end + m.start();
                    let abs_end = last_end + m.end();
                    if abs_start > last_end && abs_start <= line_owned.len() {
                        spans.push(Span::raw(line_owned[last_end..abs_start].to_string()));
                    }
                    spans.push(Span::styled(m.as_str().to_string(), Style::default().fg(Color::Blue).add_modifier(Modifier::UNDERLINED)));
                    last_end = abs_end;
                }

                if last_end < line_owned.len() {
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

    let cursor_line = app.detail_view_cursor_line.min(total_lines.saturating_sub(1));
    let cursor_col = app.detail_view_cursor_col;

    let scroll_start = if visible_lines_count == 0 {
        0
    } else if cursor_line >= app.detail_view_scroll + visible_lines_count {
        cursor_line.saturating_sub(visible_lines_count - 1)
    } else if cursor_line < app.detail_view_scroll {
        cursor_line
    } else {
        app.detail_view_scroll
    }.min(total_lines.saturating_sub(visible_lines_count));

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
