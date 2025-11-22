use chrono::{Datelike, NaiveDate};
use crate::app::AppState;

#[derive(Debug, Clone, PartialEq)]
pub struct YearLayout {
    pub year: i32,
    pub months: Vec<MonthGrid>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MonthGrid {
    pub month: u32,
    pub days: Vec<DayCell>,
    pub is_current_month: bool,
    pub first_weekday: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DayCell {
    pub day: u32,
    pub is_today: bool,
    pub is_selected: bool,
    pub has_events: bool,
}

pub fn calculate_layout(state: &AppState) -> YearLayout {
    let year = state.selected_date.year();
    let today = chrono::Local::now().date_naive();
    let current_month = today.month();
    let current_year = today.year();

    let mut months = Vec::new();

    for month in 1..=12 {
        let Some(first_day) = NaiveDate::from_ymd_opt(year, month, 1) else {
            continue;
        };

        let next_month_first = if month == 12 {
            NaiveDate::from_ymd_opt(year + 1, 1, 1)
        } else {
            NaiveDate::from_ymd_opt(year, month + 1, 1)
        };

        let Some(next_first) = next_month_first else { continue };
        let days_in_month = next_first.signed_duration_since(first_day).num_days() as u32;

        let mut days = Vec::new();

        for day in 1..=days_in_month {
            let Some(date) = NaiveDate::from_ymd_opt(year, month, day) else {
                continue;
            };
            let has_events = !state.get_events_for_date(date).is_empty();

            days.push(DayCell {
                day,
                is_today: date == today,
                is_selected: date == state.selected_date,
                has_events,
            });
        }

        let first_weekday = first_day.weekday().num_days_from_monday();

        months.push(MonthGrid {
            month,
            days,
            is_current_month: year == current_year && month == current_month,
            first_weekday,
        });
    }

    YearLayout { year, months }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    #[test]
    fn year_layout_has_twelve_months() {
        let mut state = AppState::new();
        state.selected_date = date(2025, 6, 15);

        let layout = calculate_layout(&state);

        assert_eq!(layout.months.len(), 12);
        assert_eq!(layout.year, 2025);
    }

    #[test]
    fn january_has_31_days() {
        let mut state = AppState::new();
        state.selected_date = date(2025, 1, 1);

        let layout = calculate_layout(&state);

        assert_eq!(layout.months[0].days.len(), 31);
    }

    #[test]
    fn february_2024_has_29_days() {
        let mut state = AppState::new();
        state.selected_date = date(2024, 2, 1);

        let layout = calculate_layout(&state);

        assert_eq!(layout.months[1].days.len(), 29);
    }

    #[test]
    fn selected_date_is_marked() {
        let mut state = AppState::new();
        state.selected_date = date(2025, 6, 15);

        let layout = calculate_layout(&state);

        let june = &layout.months[5];
        let selected_days: Vec<_> = june.days.iter()
            .filter(|d| d.is_selected)
            .collect();

        assert_eq!(selected_days.len(), 1);
        assert_eq!(selected_days[0].day, 15);
    }
}
