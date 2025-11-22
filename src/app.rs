use chrono::{Local, NaiveDate, Timelike};
use std::collections::HashMap;

use crate::calendar::Event;
use crate::ui::theme::Theme;

#[derive(Debug, Clone, PartialEq)]
pub enum Mode {
    Normal,
    Insert,
    Visual,
    Command,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ViewType {
    Month,
    Week,
    Day,
    Year,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SyncStatus {
    Synced,
    Syncing,
    Offline,
    Error(String),
}

#[derive(Debug, Clone)]
pub struct Position {
    pub x: usize,
    pub y: usize,
}

pub struct AppState {
    pub mode: Mode,
    pub view: ViewType,
    pub selected_date: NaiveDate,
    pub events: HashMap<String, Event>,
    pub cursor_position: Position,
    pub sync_status: SyncStatus,
    pub command_buffer: String,
    pub search_query: Option<String>,
    pub show_help: bool,
    pub help_scroll: usize,
    pub theme: Theme,
    pub event_form: Option<EventForm>,
    pub selected_event_index: usize,
    pub delete_confirmation_event_id: Option<String>,
    pub visual_selection_start: Option<NaiveDate>,
    pub detail_view_event_id: Option<String>,
    pub detail_view_scroll: usize,
    pub detail_view_cursor_line: usize,
    pub detail_view_cursor_col: usize,
    pub detail_view_line_text: Vec<String>,
    pub detail_view_visual_start: Option<(usize, usize)>,
}

#[derive(Debug, Clone)]
pub struct EventForm {
    pub title: String,
    pub date: NaiveDate,
    pub start_hour: u32,
    pub start_minute: u32,
    pub duration_minutes: u32,
    pub location: String,
    pub description: String,
    pub active_field: FormField,
    pub event_id: Option<String>,
    pub time_input_buffer: String,
    pub duration_input_buffer: String,
    pub time_buffer_touched: bool,
    pub duration_buffer_touched: bool,
    pub all_day: bool,
}

impl EventForm {
    pub fn new(date: NaiveDate, title: String) -> Self {
        let now = chrono::Local::now();
        Self {
            title,
            date,
            start_hour: now.hour(),
            start_minute: 0,
            duration_minutes: 60,
            location: String::new(),
            description: String::new(),
            active_field: FormField::Title,
            event_id: None,
            time_input_buffer: format!("{:02}:{:02}", now.hour(), 0),
            duration_input_buffer: "60".to_string(),
            time_buffer_touched: false,
            duration_buffer_touched: false,
            all_day: false,
        }
    }

    pub fn for_event(event: &Event) -> Self {
        let start_hour = event.start.time().hour();
        let start_minute = event.start.time().minute();
        let duration_minutes = event.duration_minutes() as u32;
        Self {
            title: event.title.clone(),
            date: event.start.date_naive(),
            start_hour,
            start_minute,
            duration_minutes,
            location: event.location.clone().unwrap_or_default(),
            description: event.description.clone().unwrap_or_default(),
            active_field: FormField::Title,
            event_id: Some(event.id.clone()),
            time_input_buffer: format!("{:02}:{:02}", start_hour, start_minute),
            duration_input_buffer: duration_minutes.to_string(),
            time_buffer_touched: false,
            duration_buffer_touched: false,
            all_day: event.all_day,
        }
    }

    pub fn new_all_day(date: NaiveDate, title: String, duration_days: u32) -> Self {
        Self {
            title,
            date,
            start_hour: 0,
            start_minute: 0,
            duration_minutes: duration_days * 24 * 60,
            location: String::new(),
            description: String::new(),
            active_field: FormField::Title,
            event_id: None,
            time_input_buffer: String::new(),
            duration_input_buffer: duration_days.to_string(),
            time_buffer_touched: false,
            duration_buffer_touched: false,
            all_day: true,
        }
    }

    pub fn is_editing(&self) -> bool {
        self.event_id.is_some()
    }

    pub fn next_field(&mut self) {
        if self.all_day {
            self.active_field = match self.active_field {
                FormField::Title => FormField::Duration,
                FormField::Duration => FormField::Location,
                FormField::Location => FormField::Description,
                FormField::Description => FormField::Title,
                FormField::StartTime => FormField::Duration,
            };
        } else {
            self.active_field = match self.active_field {
                FormField::Title => FormField::StartTime,
                FormField::StartTime => FormField::Duration,
                FormField::Duration => FormField::Location,
                FormField::Location => FormField::Description,
                FormField::Description => FormField::Title,
            };
        }
    }

    pub fn prev_field(&mut self) {
        if self.all_day {
            self.active_field = match self.active_field {
                FormField::Title => FormField::Description,
                FormField::Duration => FormField::Title,
                FormField::Location => FormField::Duration,
                FormField::Description => FormField::Location,
                FormField::StartTime => FormField::Title,
            };
        } else {
            self.active_field = match self.active_field {
                FormField::Title => FormField::Description,
                FormField::StartTime => FormField::Title,
                FormField::Duration => FormField::StartTime,
                FormField::Location => FormField::Duration,
                FormField::Description => FormField::Location,
            };
        }
    }

    pub fn parse_time_input(&mut self) {
        let input = self.time_input_buffer.replace(':', "");
        if let Ok(num) = input.parse::<u32>() {
            if input.len() == 3 || input.len() == 4 {
                self.start_hour = num / 100;
                self.start_minute = num % 100;
                if self.start_hour >= 24 {
                    self.start_hour = 23;
                }
                if self.start_minute >= 60 {
                    self.start_minute = 59;
                }
                self.time_input_buffer = format!("{:02}:{:02}", self.start_hour, self.start_minute);
            } else if input.len() <= 2 {
                self.start_hour = num.min(23);
                self.start_minute = 0;
                self.time_input_buffer = format!("{:02}:{:02}", self.start_hour, self.start_minute);
            }
        }
    }

    pub fn parse_duration_input(&mut self) {
        if let Ok(value) = self.duration_input_buffer.parse::<u32>() {
            if self.all_day {
                let days = value.clamp(1, 365);
                self.duration_minutes = days * 24 * 60;
            } else {
                self.duration_minutes = value.clamp(1, 10080);
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum FormField {
    Title,
    StartTime,
    Duration,
    Location,
    Description,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            mode: Mode::Normal,
            view: ViewType::Month,
            selected_date: Local::now().date_naive(),
            events: HashMap::new(),
            cursor_position: Position { x: 0, y: 0 },
            sync_status: SyncStatus::Synced,
            command_buffer: String::new(),
            search_query: None,
            show_help: false,
            help_scroll: 0,
            theme: Theme::default(),
            event_form: None,
            selected_event_index: 0,
            delete_confirmation_event_id: None,
            visual_selection_start: None,
            detail_view_event_id: None,
            detail_view_scroll: 0,
            detail_view_cursor_line: 0,
            detail_view_cursor_col: 0,
            detail_view_line_text: Vec::new(),
            detail_view_visual_start: None,
        }
    }

    pub fn with_theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    pub fn add_event(&mut self, event: Event) {
        self.events.insert(event.id.clone(), event);
    }

    pub fn get_events_for_date(&self, date: NaiveDate) -> Vec<&Event> {
        let mut events: Vec<&Event> = self.events
            .values()
            .filter(|event| event.start.date_naive() == date)
            .collect();
        events.sort_by_key(|e| e.start);
        events
    }

    pub fn get_selected_event(&self) -> Option<&Event> {
        let events = self.get_events_for_date(self.selected_date);
        events.get(self.selected_event_index).copied()
    }

    pub fn move_event_selection_down(&mut self) {
        let event_count = self.get_events_for_date(self.selected_date).len();
        if event_count > 0 && self.selected_event_index < event_count - 1 {
            self.selected_event_index += 1;
        }
    }

    pub fn move_event_selection_up(&mut self) {
        if self.selected_event_index > 0 {
            self.selected_event_index -= 1;
        }
    }

    pub fn reset_event_selection(&mut self) {
        self.selected_event_index = 0;
    }

    pub fn remove_event(&mut self, event_id: &str) {
        self.events.remove(event_id);
    }

    pub fn get_visual_selection_range(&self) -> Option<(NaiveDate, NaiveDate)> {
        self.visual_selection_start.map(|start| {
            let end = self.selected_date;
            if start <= end {
                (start, end)
            } else {
                (end, start)
            }
        })
    }

    pub fn is_date_in_visual_selection(&self, date: NaiveDate) -> bool {
        if let Some((start, end)) = self.get_visual_selection_range() {
            date >= start && date <= end
        } else {
            false
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn create_event_at(id: &str, date: NaiveDate, hour: u32) -> Event {
        let start = Utc.from_local_datetime(&date.and_hms_opt(hour, 0, 0).unwrap()).unwrap();
        Event {
            id: id.to_string(),
            calendar_id: "primary".to_string(),
            title: format!("Event {}", id),
            description: None,
            location: None,
            start,
            end: start + chrono::Duration::hours(1),
            all_day: false,
            attendees: vec![],
            reminders: vec![],
            status: crate::calendar::EventStatus::Confirmed,
            last_modified: Utc::now(),
            html_link: None,
        }
    }

    #[test]
    fn new_app_starts_in_normal_mode() {
        let app = AppState::new();
        assert_eq!(app.mode, Mode::Normal);
    }

    #[test]
    fn new_app_shows_month_view() {
        let app = AppState::new();
        assert_eq!(app.view, ViewType::Month);
    }

    #[test]
    fn new_app_selects_today() {
        let app = AppState::new();
        assert_eq!(app.selected_date, Local::now().date_naive());
    }

    #[test]
    fn new_app_has_no_events() {
        let app = AppState::new();
        assert_eq!(app.events.len(), 0);
    }

    #[test]
    fn add_event_to_state() {
        use chrono::Utc;
        let mut app = AppState::new();
        let start = Utc::now();
        let end = start + chrono::Duration::hours(1);

        let event = Event {
            id: "event1".to_string(),
            calendar_id: "primary".to_string(),
            title: "Meeting".to_string(),
            description: None,
            location: None,
            start,
            end,
            all_day: false,
            attendees: vec![],
            reminders: vec![],
            status: crate::calendar::EventStatus::Confirmed,
            last_modified: Utc::now(),
            html_link: None,
        };

        app.add_event(event.clone());

        assert_eq!(app.events.len(), 1);
        assert_eq!(app.events.get(&event.id), Some(&event));
    }

    #[test]
    fn get_events_for_date_returns_matching_events() {
        let mut app = AppState::new();
        let date = chrono::NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();
        let other_date = chrono::NaiveDate::from_ymd_opt(2025, 1, 16).unwrap();

        app.add_event(create_event_at("event1", date, 9));
        app.add_event(create_event_at("event2", date, 14));
        app.add_event(create_event_at("event3", other_date, 10));

        let events = app.get_events_for_date(date);

        assert_eq!(events.len(), 2);
    }
}
