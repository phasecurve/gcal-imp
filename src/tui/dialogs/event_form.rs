use ratatui::{
    layout::Alignment,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use gcal_imp::app::{AppState, FormField};

pub fn render(f: &mut Frame, app: &AppState) {
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
