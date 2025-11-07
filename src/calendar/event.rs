use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Event {
    pub id: String,
    pub calendar_id: String,
    pub title: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub all_day: bool,
    pub attendees: Vec<String>,
    pub reminders: Vec<Reminder>,
    pub status: EventStatus,
    pub last_modified: DateTime<Utc>,
    pub html_link: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EventStatus {
    Confirmed,
    Tentative,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Reminder {
    pub method: ReminderMethod,
    pub minutes_before: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ReminderMethod {
    Email,
    Popup,
}

impl Event {
    pub fn duration_minutes(&self) -> i64 {
        (self.end - self.start).num_minutes()
    }

    pub fn overlaps(&self, other: &Event) -> bool {
        self.start < other.end && other.start < self.end
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_event(id: &str, title: &str, start: DateTime<Utc>, end: DateTime<Utc>) -> Event {
        Event {
            id: id.to_string(),
            calendar_id: "test_cal".to_string(),
            title: title.to_string(),
            description: None,
            location: None,
            start,
            end,
            all_day: false,
            attendees: vec![],
            reminders: vec![],
            status: EventStatus::Confirmed,
            last_modified: Utc::now(),
            html_link: None,
        }
    }

    #[test]
    fn new_event_has_confirmed_status() {
        let start = Utc::now();
        let end = start + chrono::Duration::hours(1);
        let event = create_test_event("test_id", "Test Event", start, end);

        assert_eq!(event.status, EventStatus::Confirmed);
    }

    #[test]
    fn event_duration_calculated_correctly() {
        let start = Utc::now();
        let end = start + chrono::Duration::minutes(90);
        let event = create_test_event("test_id", "Meeting", start, end);

        assert_eq!(event.duration_minutes(), 90);
    }

    #[test]
    fn event_overlaps_with_another_event() {
        let start1 = Utc::now();
        let end1 = start1 + chrono::Duration::hours(2);
        let event1 = create_test_event("event1", "Event 1", start1, end1);

        let start2 = start1 + chrono::Duration::hours(1);
        let end2 = start2 + chrono::Duration::hours(1);
        let event2 = create_test_event("event2", "Event 2", start2, end2);

        assert!(event1.overlaps(&event2));
    }

    #[test]
    fn event_does_not_overlap_when_adjacent() {
        let start1 = Utc::now();
        let end1 = start1 + chrono::Duration::hours(1);
        let event1 = create_test_event("event1", "Event 1", start1, end1);

        let event2 = create_test_event("event2", "Event 2", end1, end1 + chrono::Duration::hours(1));

        assert!(!event1.overlaps(&event2));
    }
}
