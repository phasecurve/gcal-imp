use chrono::{Datelike, NaiveDate, Weekday};
use crate::app::AppState;

#[derive(Debug, Clone, PartialEq)]
pub struct MonthLayout {
    pub year: i32,
    pub month: u32,
    pub weeks: Vec<Week>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Week {
    pub days: Vec<DayCell>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DayCell {
    pub date: Option<NaiveDate>,
    pub is_selected: bool,
    pub is_today: bool,
    pub has_events: bool,
    pub is_current_month: bool,
}

impl DayCell {
    pub fn new(date: Option<NaiveDate>) -> Self {
        Self {
            date,
            is_selected: false,
            is_today: false,
            has_events: false,
            is_current_month: true,
        }
    }

    pub fn with_selected(mut self, selected: bool) -> Self {
        self.is_selected = selected;
        self
    }

    pub fn with_today(mut self, today: bool) -> Self {
        self.is_today = today;
        self
    }

    pub fn with_events(mut self, has_events: bool) -> Self {
        self.has_events = has_events;
        self
    }

    pub fn with_current_month(mut self, current_month: bool) -> Self {
        self.is_current_month = current_month;
        self
    }
}

pub fn calculate_layout(state: &AppState) -> MonthLayout {
    let year = state.selected_date.year();
    let month = state.selected_date.month();
    let today = chrono::Local::now().date_naive();

    let Some(first_day) = NaiveDate::from_ymd_opt(year, month, 1) else {
        return MonthLayout { year, month, weeks: Vec::new() };
    };

    let next_month_first = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
    };

    let Some(last_day) = next_month_first.and_then(|d| d.pred_opt()) else {
        return MonthLayout { year, month, weeks: Vec::new() };
    };

    let mut weeks = Vec::new();
    let mut current_week = Week { days: Vec::new() };

    let start_weekday = first_day.weekday();
    let days_before = start_weekday.num_days_from_monday() as i64;

    for i in 0..days_before {
        let prev_date = first_day.pred_opt()
            .and_then(|d| d.checked_sub_days(chrono::Days::new((days_before - i - 1) as u64)));

        current_week.days.push(
            DayCell::new(prev_date)
                .with_current_month(false)
        );
    }

    let mut current_date = first_day;
    while current_date <= last_day {
        let has_events = !state.get_events_for_date(current_date).is_empty();

        let cell = DayCell::new(Some(current_date))
            .with_selected(current_date == state.selected_date)
            .with_today(current_date == today)
            .with_events(has_events)
            .with_current_month(true);

        current_week.days.push(cell);

        if current_date.weekday() == Weekday::Sun {
            weeks.push(current_week);
            current_week = Week { days: Vec::new() };
        }

        let Some(next) = current_date.succ_opt() else { break };
        current_date = next;
    }

    if !current_week.days.is_empty() {
        while current_week.days.len() < 7 {
            let next_date = current_date;
            current_week.days.push(
                DayCell::new(Some(next_date))
                    .with_current_month(false)
            );
            let Some(next) = current_date.succ_opt() else { break };
            current_date = next;
        }
        weeks.push(current_week);
    }

    MonthLayout { year, month, weeks }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use crate::calendar::{Event, EventStatus};

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    fn create_event_on_date(id: &str, event_date: NaiveDate) -> Event {
        let start = event_date.and_hms_opt(10, 0, 0).unwrap().and_utc();
        Event {
            id: id.to_string(),
            calendar_id: "primary".to_string(),
            title: "Event".to_string(),
            description: None,
            location: None,
            start,
            end: start + chrono::Duration::hours(1),
            all_day: false,
            attendees: vec![],
            reminders: vec![],
            status: EventStatus::Confirmed,
            last_modified: Utc::now(),
            html_link: None,
        }
    }

    #[test]
    fn month_layout_has_correct_year_and_month() {
        let mut state = AppState::new();
        state.selected_date = date(2025, 1, 15);

        let layout = calculate_layout(&state);

        assert_eq!(layout.year, 2025);
        assert_eq!(layout.month, 1);
    }

    #[test]
    fn month_layout_has_weeks() {
        let mut state = AppState::new();
        state.selected_date = date(2025, 1, 15);

        let layout = calculate_layout(&state);

        assert!(!layout.weeks.is_empty());
    }

    #[test]
    fn selected_date_is_marked_in_layout() {
        let mut state = AppState::new();
        state.selected_date = date(2025, 1, 15);

        let layout = calculate_layout(&state);

        let selected_cells: Vec<_> = layout.weeks.iter()
            .flat_map(|w| &w.days)
            .filter(|c| c.is_selected)
            .collect();

        assert_eq!(selected_cells.len(), 1);
        assert_eq!(selected_cells[0].date, Some(date(2025, 1, 15)));
    }

    #[test]
    fn cells_with_events_are_marked() {
        let mut state = AppState::new();
        state.selected_date = date(2025, 1, 15);
        let event_date = date(2025, 1, 10);
        state.add_event(create_event_on_date("event1", event_date));

        let layout = calculate_layout(&state);

        let event_cells: Vec<_> = layout.weeks.iter()
            .flat_map(|w| &w.days)
            .filter(|c| c.has_events && c.date == Some(event_date))
            .collect();

        assert_eq!(event_cells.len(), 1);
    }

    #[test]
    fn each_week_has_seven_days() {
        let mut state = AppState::new();
        state.selected_date = date(2025, 1, 15);

        let layout = calculate_layout(&state);

        for week in &layout.weeks {
            assert_eq!(week.days.len(), 7);
        }
    }

    #[test]
    fn previous_month_days_marked_as_not_current() {
        let mut state = AppState::new();
        state.selected_date = date(2025, 1, 15);

        let layout = calculate_layout(&state);

        let first_week = &layout.weeks[0];
        let prev_month_days: Vec<_> = first_week.days.iter()
            .filter(|c| !c.is_current_month)
            .collect();

        assert!(!prev_month_days.is_empty());
    }
}
