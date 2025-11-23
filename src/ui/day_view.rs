use chrono::{NaiveDate, Timelike};
use crate::app::AppState;
use crate::calendar::Event;

#[derive(Debug, Clone, PartialEq)]
pub struct DayLayout {
    pub date: NaiveDate,
    pub is_today: bool,
    pub hours: Vec<HourBlock>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HourBlock {
    pub hour: u32,
    pub events: Vec<EventEntry>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EventEntry {
    pub event_id: String,
    pub title: String,
    pub start_minute: u32,
    pub duration_minutes: i64,
    pub location: Option<String>,
    pub description: Option<String>,
}

pub fn calculate_layout(state: &AppState) -> DayLayout {
    let date = state.selected_date;
    let today = chrono::Local::now().date_naive();
    let events = state.get_events_for_date(date);

    let hours = build_hour_blocks(&events);

    DayLayout {
        date,
        is_today: date == today,
        hours,
    }
}

fn build_hour_blocks(events: &[&Event]) -> Vec<HourBlock> {
    let mut blocks = Vec::new();

    for hour in 0..24 {
        let hour_events: Vec<EventEntry> = events
            .iter()
            .filter(|e| e.start.hour() == hour)
            .map(|e| EventEntry {
                event_id: e.id.clone(),
                title: e.title.clone(),
                start_minute: e.start.minute(),
                duration_minutes: e.duration_minutes(),
                location: e.location.clone(),
                description: e.description.clone(),
            })
            .collect();

        blocks.push(HourBlock {
            hour,
            events: hour_events,
        });
    }

    blocks
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use crate::calendar::{Event, EventStatus, DEFAULT_CALENDAR_ID};

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    fn create_event(
        id: &str,
        title: &str,
        date: NaiveDate,
        hour: u32,
        minute: u32,
        duration_minutes: i64,
    ) -> Event {
        use chrono::TimeZone;
        let start = Utc
            .from_local_datetime(&date.and_hms_opt(hour, minute, 0).unwrap())
            .unwrap();
        Event {
            id: id.to_string(),
            calendar_id: DEFAULT_CALENDAR_ID.to_string(),
            title: title.to_string(),
            description: None,
            location: None,
            start,
            end: start + chrono::Duration::minutes(duration_minutes),
            all_day: false,
            attendees: vec![],
            reminders: vec![],
            status: EventStatus::Confirmed,
            last_modified: Utc::now(),
            html_link: None,
        }
    }

    #[test]
    fn day_layout_has_date() {
        let mut state = AppState::new();
        state.selected_date = date(2025, 1, 15);

        let layout = calculate_layout(&state);

        assert_eq!(layout.date, date(2025, 1, 15));
    }

    #[test]
    fn day_layout_has_24_hours() {
        let mut state = AppState::new();
        state.selected_date = date(2025, 1, 15);

        let layout = calculate_layout(&state);

        assert_eq!(layout.hours.len(), 24);
    }

    #[test]
    fn hours_are_in_order() {
        let mut state = AppState::new();
        state.selected_date = date(2025, 1, 15);

        let layout = calculate_layout(&state);

        for (i, hour_block) in layout.hours.iter().enumerate() {
            assert_eq!(hour_block.hour, i as u32);
        }
    }

    #[test]
    fn events_are_placed_in_correct_hour() {
        let mut state = AppState::new();
        state.selected_date = date(2025, 1, 15);

        let event = create_event("e1", "Morning Meeting", date(2025, 1, 15), 9, 30, 60);
        state.add_event(event);

        let layout = calculate_layout(&state);

        let hour_9 = &layout.hours[9];
        assert_eq!(hour_9.events.len(), 1);
        assert_eq!(hour_9.events[0].title, "Morning Meeting");
    }

    #[test]
    fn event_entry_includes_start_minute() {
        let mut state = AppState::new();
        state.selected_date = date(2025, 1, 15);

        let event = create_event("e1", "Meeting", date(2025, 1, 15), 10, 45, 60);
        state.add_event(event);

        let layout = calculate_layout(&state);

        let hour_10 = &layout.hours[10];
        assert_eq!(hour_10.events[0].start_minute, 45);
    }

    #[test]
    fn event_entry_includes_duration() {
        let mut state = AppState::new();
        state.selected_date = date(2025, 1, 15);

        let event = create_event("e1", "Long Meeting", date(2025, 1, 15), 14, 0, 120);
        state.add_event(event);

        let layout = calculate_layout(&state);

        let hour_14 = &layout.hours[14];
        assert_eq!(hour_14.events[0].duration_minutes, 120);
    }

    #[test]
    fn multiple_events_in_same_hour() {
        let mut state = AppState::new();
        state.selected_date = date(2025, 1, 15);

        let event1 = create_event("e1", "Meeting A", date(2025, 1, 15), 10, 0, 30);
        let event2 = create_event("e2", "Meeting B", date(2025, 1, 15), 10, 30, 30);

        state.add_event(event1);
        state.add_event(event2);

        let layout = calculate_layout(&state);

        let hour_10 = &layout.hours[10];
        assert_eq!(hour_10.events.len(), 2);
    }

    #[test]
    fn empty_hours_have_no_events() {
        let mut state = AppState::new();
        state.selected_date = date(2025, 1, 15);

        let layout = calculate_layout(&state);

        let hour_0 = &layout.hours[0];
        assert_eq!(hour_0.events.len(), 0);
    }

    #[test]
    fn is_today_flag_set_correctly() {
        let mut state = AppState::new();
        state.selected_date = chrono::Local::now().date_naive();

        let layout = calculate_layout(&state);

        assert!(layout.is_today);
    }

    #[test]
    fn is_today_flag_false_for_other_days() {
        let mut state = AppState::new();
        state.selected_date = date(2025, 1, 1);

        let layout = calculate_layout(&state);

        assert!(!layout.is_today);
    }
}
