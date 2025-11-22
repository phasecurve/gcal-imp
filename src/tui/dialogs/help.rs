use ratatui::{
    layout::Alignment,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use gcal_imp::app::AppState;

pub fn render(f: &mut Frame, app: &AppState) {
    let area = f.size();
    let help_width = 60;
    let help_height = 23;
    let x = (area.width.saturating_sub(help_width)) / 2;
    let y = (area.height.saturating_sub(help_height)) / 2;

    let help_area = ratatui::layout::Rect {
        x,
        y,
        width: help_width,
        height: help_height,
    };

    f.render_widget(Clear, help_area);

    let help_text = vec![
        Line::from(vec![Span::styled("gcal-imp Help", Style::default().fg(app.theme.help_title).add_modifier(Modifier::BOLD))]),
        Line::from(""),
        Line::from(vec![Span::styled("Navigation:", Style::default().fg(app.theme.help_section))]),
        Line::from("  h/l      - Previous/next day"),
        Line::from("  j/k      - Navigate events (or week if no events)"),
        Line::from("  t        - Jump to today"),
        Line::from("  g/G      - First/last day of month"),
        Line::from("  { / }    - Previous/next month"),
        Line::from(""),
        Line::from(vec![Span::styled("Views:", Style::default().fg(app.theme.help_section))]),
        Line::from("  m/w/d/y  - Month/Week/Day/Year view"),
        Line::from(""),
        Line::from(vec![Span::styled("Event Management:", Style::default().fg(app.theme.help_section))]),
        Line::from("  a        - Add new event (insert mode)"),
        Line::from("  :new     - Create event (:new [Meeting title])"),
        Line::from("  Enter    - Day view (Month) / Edit (Day)"),
        Line::from("  i        - View event details (scrollable)"),
        Line::from("  E        - Edit selected event"),
        Line::from("  x        - Delete selected event"),
        Line::from("  v        - Visual mode (select date range)"),
        Line::from(""),
        Line::from(vec![Span::styled("Detail View:", Style::default().fg(app.theme.help_section))]),
        Line::from("  hjkl     - Navigate cursor"),
        Line::from("  wbe      - Word motions"),
        Line::from("  0^$      - Line start/first-non-ws/end"),
        Line::from("  gG       - Top/bottom"),
        Line::from("  a        - Add new event"),
        Line::from("  o        - Open URL at cursor"),
        Line::from("  y        - Yank line to clipboard"),
        Line::from("  B        - Open event in browser"),
        Line::from("  E        - Edit event"),
        Line::from("  q/Esc    - Close detail view"),
        Line::from(""),
        Line::from(vec![Span::styled("Commands:", Style::default().fg(app.theme.help_section))]),
        Line::from("  :q       - Quit"),
        Line::from("  :w       - Sync with Google Calendar"),
        Line::from("  :goto    - Jump to date (:goto 2025-12-25)"),
        Line::from("  :theme   - Change theme (:theme gruvbox)"),
        Line::from("  :help    - Show this help"),
        Line::from(""),
    ];

    let visible_lines = help_height.saturating_sub(3) as usize;
    let total_lines = help_text.len();
    let max_scroll = total_lines.saturating_sub(visible_lines);
    let scroll = app.help_scroll.min(max_scroll);

    let scrolled_text: Vec<Line> = help_text
        .into_iter()
        .skip(scroll)
        .take(visible_lines)
        .collect();

    let help_paragraph = Paragraph::new(scrolled_text)
        .block(Block::default()
            .borders(Borders::ALL)
            .title(format!(" Help (j/k to scroll, q to close) [{}/{}] ", scroll + 1, total_lines))
            .style(Style::default().bg(Color::Black)))
        .alignment(Alignment::Left);

    f.render_widget(help_paragraph, help_area);
}
