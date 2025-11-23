use chrono::{Local, TimeZone, Utc};
use gcal_imp::{
    app::AppState,
    calendar::{Event as CalendarEvent, EventStatus},
};

pub fn add_sample_events(app: &mut AppState) {
    let today = Local::now().date_naive();

    let Some(tomorrow) = today.succ_opt() else { return };
    let Some(yesterday) = today.pred_opt() else { return };

    let events = vec![
        ("Morning Standup", today, 9, 0, 9, 30, None),
        ("Team Sync", today, 14, 0, 15, 0, Some("Conference Room A")),
        ("Code Review", tomorrow, 10, 0, 11, 0, None),
        ("Sprint Planning", tomorrow, 15, 0, 16, 30, Some("Zoom")),
        ("1-on-1 with Manager", yesterday, 11, 0, 11, 30, None),
        ("Lunch with Team", yesterday, 12, 30, 13, 30, Some("Downtown Cafe")),
    ];

    for (i, (title, date, start_h, start_m, end_h, end_m, location)) in events.into_iter().enumerate() {
        let Some(start_time) = date.and_hms_opt(start_h, start_m, 0) else { continue };
        let Some(end_time) = date.and_hms_opt(end_h, end_m, 0) else { continue };

        let start = match Utc.from_local_datetime(&start_time) {
            chrono::LocalResult::Single(dt) => dt,
            _ => continue,
        };
        let end = match Utc.from_local_datetime(&end_time) {
            chrono::LocalResult::Single(dt) => dt,
            _ => continue,
        };

        let event = CalendarEvent {
            id: format!("sample_{}", i),
            calendar_id: "primary".to_string(),
            title: title.to_string(),
            description: Some("Sample event for testing".to_string()),
            location: location.map(String::from),
            start,
            end,
            all_day: false,
            attendees: vec![],
            reminders: vec![],
            status: EventStatus::Confirmed,
            last_modified: Utc::now(),
            html_link: None,
        };

        app.add_event(event);
    }
}
