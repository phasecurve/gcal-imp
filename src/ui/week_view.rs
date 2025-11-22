use chrono::{Datelike, NaiveDate, Timelike};
use crate::app::AppState;
use crate::calendar::Event;

#[derive(Debug, Clone, PartialEq)]
pub struct WeekLayout {
    pub week_start: NaiveDate,
    pub days: Vec<DayColumn>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DayColumn {
    pub date: NaiveDate,
    pub is_selected: bool,
    pub is_today: bool,
    pub events: Vec<TimeSlot>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TimeSlot {
    pub hour: u32,
    pub events: Vec<EventBlock>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EventBlock {
    pub event_id: String,
    pub title: String,
    pub start_hour: u32,
    pub start_minute: u32,
    pub duration_minutes: i64,
}

impl WeekLayout {
    pub fn week_of_date(date: NaiveDate) -> NaiveDate {
        let weekday = date.weekday();
        let days_from_monday = weekday.num_days_from_monday() as u64;
        date.checked_sub_days(chrono::Days::new(days_from_monday))
            .unwrap_or(date)
    }
}

pub fn calculate_layout(state: &AppState) -> WeekLayout {
    let week_start = WeekLayout::week_of_date(state.selected_date);
    let today = chrono::Local::now().date_naive();

    let mut days = Vec::new();

    for day_offset in 0..7u64 {
        let Some(date) = week_start.checked_add_days(chrono::Days::new(day_offset)) else {
            continue;
        };
        let events = state.get_events_for_date(date);

        let time_slots = build_time_slots(&events);

        days.push(DayColumn {
            date,
            is_selected: date == state.selected_date,
            is_today: date == today,
            events: time_slots,
        });
    }

    WeekLayout { week_start, days }
}

fn build_time_slots(events: &[&Event]) -> Vec<TimeSlot> {
    let mut slots = Vec::new();

    for hour in 0..24 {
        let hour_events: Vec<EventBlock> = events
            .iter()
            .filter(|e| e.start.hour() == hour)
            .map(|e| EventBlock {
                event_id: e.id.clone(),
                title: e.title.clone(),
                start_hour: e.start.hour(),
                start_minute: e.start.minute(),
                duration_minutes: e.duration_minutes(),
            })
            .collect();

        if !hour_events.is_empty() {
            slots.push(TimeSlot {
                hour,
                events: hour_events,
            });
        }
    }

    slots
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Utc, Weekday};
    use crate::calendar::{Event, EventStatus};

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    fn create_event(id: &str, title: &str, date: NaiveDate, hour: u32, duration_hours: i64) -> Event {
        use chrono::TimeZone;
        let start = Utc.from_local_datetime(&date.and_hms_opt(hour, 0, 0).unwrap()).unwrap();
        Event {
            id: id.to_string(),
            calendar_id: "primary".to_string(),
            title: title.to_string(),
            description: None,
            location: None,
            start,
            end: start + chrono::Duration::hours(duration_hours),
            all_day: false,
            attendees: vec![],
            reminders: vec![],
            status: EventStatus::Confirmed,
            last_modified: Utc::now(),
            html_link: None,
        }
    }

    #[test]
    fn week_of_date_returns_monday() {
        let wednesday = date(2025, 1, 15);
        let monday = WeekLayout::week_of_date(wednesday);
        assert_eq!(monday, date(2025, 1, 13));
        assert_eq!(monday.weekday(), Weekday::Mon);
    }

    #[test]
    fn week_of_date_for_monday_returns_same_date() {
        let monday = date(2025, 1, 13);
        let week_start = WeekLayout::week_of_date(monday);
        assert_eq!(week_start, monday);
    }

    #[test]
    fn week_of_date_for_sunday_returns_previous_monday() {
        let sunday = date(2025, 1, 19);
        let monday = WeekLayout::week_of_date(sunday);
        assert_eq!(monday, date(2025, 1, 13));
    }

    #[test]
    fn week_layout_has_seven_days() {
        let mut state = AppState::new();
        state.selected_date = date(2025, 1, 15);

        let layout = calculate_layout(&state);

        assert_eq!(layout.days.len(), 7);
    }

    #[test]
    fn week_layout_starts_on_monday() {
        let mut state = AppState::new();
        state.selected_date = date(2025, 1, 15);

        let layout = calculate_layout(&state);

        assert_eq!(layout.days[0].date.weekday(), Weekday::Mon);
    }

    #[test]
    fn week_layout_ends_on_sunday() {
        let mut state = AppState::new();
        state.selected_date = date(2025, 1, 15);

        let layout = calculate_layout(&state);

        assert_eq!(layout.days[6].date.weekday(), Weekday::Sun);
    }

    #[test]
    fn selected_date_is_marked() {
        let mut state = AppState::new();
        state.selected_date = date(2025, 1, 15);

        let layout = calculate_layout(&state);

        let selected_days: Vec<_> = layout.days.iter()
            .filter(|d| d.is_selected)
            .collect();

        assert_eq!(selected_days.len(), 1);
        assert_eq!(selected_days[0].date, date(2025, 1, 15));
    }

    #[test]
    fn events_are_organized_by_hour() {
        let mut state = AppState::new();
        state.selected_date = date(2025, 1, 15);

        let event1 = create_event("e1", "Morning", date(2025, 1, 15), 9, 1);
        let event2 = create_event("e2", "Afternoon", date(2025, 1, 15), 14, 2);

        state.add_event(event1);
        state.add_event(event2);

        let layout = calculate_layout(&state);

        let wednesday = &layout.days[2];
        let time_slots = &wednesday.events;

        assert_eq!(time_slots.len(), 2);
        assert_eq!(time_slots[0].hour, 9);
        assert_eq!(time_slots[1].hour, 14);
    }

    #[test]
    fn event_block_includes_duration() {
        let mut state = AppState::new();
        state.selected_date = date(2025, 1, 15);

        let event = create_event("e1", "Long Meeting", date(2025, 1, 15), 10, 2);
        state.add_event(event);

        let layout = calculate_layout(&state);

        let wednesday = &layout.days[2];
        let time_slot = &wednesday.events[0];
        let event_block = &time_slot.events[0];

        assert_eq!(event_block.duration_minutes, 120);
    }

    #[test]
    fn multiple_events_in_same_hour() {
        let mut state = AppState::new();
        state.selected_date = date(2025, 1, 15);

        let event1 = create_event("e1", "Meeting A", date(2025, 1, 15), 10, 1);
        let event2 = create_event("e2", "Meeting B", date(2025, 1, 15), 10, 1);

        state.add_event(event1);
        state.add_event(event2);

        let layout = calculate_layout(&state);

        let wednesday = &layout.days[2];
        let time_slot = &wednesday.events[0];

        assert_eq!(time_slot.events.len(), 2);
    }
}
