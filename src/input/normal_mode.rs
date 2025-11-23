use chrono::{Datelike, Days, NaiveDate};
use crossterm::event::KeyCode;

use crate::app::{AppState, Mode, ViewType};

pub fn handle_key(key: KeyCode, state: &mut AppState) {
    match key {
        KeyCode::Char('h') => move_previous_day(state),
        KeyCode::Char('j') => {
            if state.view == ViewType::Day || has_events_on_selected_date(state) {
                state.move_event_selection_down();
            } else {
                move_down_week(state);
            }
        }
        KeyCode::Char('k') => {
            if state.view == ViewType::Day || has_events_on_selected_date(state) {
                state.move_event_selection_up();
            } else {
                move_up_week(state);
            }
        }
        KeyCode::Char('l') => move_next_day(state),
        KeyCode::Char('t') => jump_to_today(state),
        KeyCode::Char('m') => switch_to_month_view(state),
        KeyCode::Char('w') => switch_to_week_view(state),
        KeyCode::Char('d') => switch_to_day_view(state),
        KeyCode::Char('y') => switch_to_year_view(state),
        KeyCode::Char('a') => enter_insert_mode(state),
        KeyCode::Char('E') => enter_edit_mode(state),
        KeyCode::Char('x') => delete_selected_event(state),
        KeyCode::Char('v') => enter_visual_mode(state),
        KeyCode::Char('i') => open_event_detail_view(state),
        KeyCode::Enter => handle_enter_key(state),
        KeyCode::Char(':') => enter_command_mode(state),
        KeyCode::Char('?') => show_help(state),
        KeyCode::Char('g') => handle_gg_motion(state),
        KeyCode::Char('G') => move_to_end_of_month(state),
        KeyCode::Char('{') => move_previous_month(state),
        KeyCode::Char('}') => move_next_month(state),
        _ => {}
    }
}

fn has_events_on_selected_date(state: &AppState) -> bool {
    !state.get_events_for_date(state.selected_date).is_empty()
}

fn move_previous_day(state: &mut AppState) {
    if let Some(new_date) = state.selected_date.checked_sub_days(Days::new(1)) {
        state.selected_date = new_date;
        state.reset_event_selection();
    }
}

fn move_next_day(state: &mut AppState) {
    if let Some(new_date) = state.selected_date.checked_add_days(Days::new(1)) {
        state.selected_date = new_date;
        state.reset_event_selection();
    }
}

fn move_down_week(state: &mut AppState) {
    if let Some(new_date) = state.selected_date.checked_add_days(Days::new(7)) {
        state.selected_date = new_date;
    }
}

fn move_up_week(state: &mut AppState) {
    if let Some(new_date) = state.selected_date.checked_sub_days(Days::new(7)) {
        state.selected_date = new_date;
    }
}

fn jump_to_today(state: &mut AppState) {
    state.selected_date = chrono::Local::now().date_naive();
    state.reset_event_selection();
}

fn enter_edit_mode(state: &mut AppState) {
    if let Some(event) = state.get_selected_event() {
        state.event_form = Some(crate::app::EventForm::for_event(event));
        state.mode = Mode::Insert;
    }
}

fn delete_selected_event(state: &mut AppState) {
    if let Some(event) = state.get_selected_event() {
        state.delete_confirmation_event_id = Some(event.id.clone());
        state.mode = Mode::Visual;
    }
}

fn enter_visual_mode(state: &mut AppState) {
    state.visual_selection_start = Some(state.selected_date);
    state.mode = Mode::Visual;
}

fn open_event_detail_view(state: &mut AppState) {
    if let Some(event) = state.get_selected_event() {
        state.detail_view_event_id = Some(event.id.clone());
        state.detail_view_scroll = 0;
        state.detail_view_cursor_line = 0;
        state.detail_view_cursor_col = 0;
    }
}

fn handle_enter_key(state: &mut AppState) {
    match state.view {
        ViewType::Month | ViewType::Week => {
            state.view = ViewType::Day;
        }
        ViewType::Day => {
            if state.get_selected_event().is_some() {
                enter_edit_mode(state);
            }
        }
        _ => {}
    }
}

fn switch_to_month_view(state: &mut AppState) {
    state.view = ViewType::Month;
}

fn switch_to_week_view(state: &mut AppState) {
    state.view = ViewType::Week;
}

fn switch_to_day_view(state: &mut AppState) {
    state.view = ViewType::Day;
}

fn switch_to_year_view(state: &mut AppState) {
    state.view = ViewType::Year;
}

fn enter_insert_mode(state: &mut AppState) {
    state.event_form = Some(crate::app::EventForm::new(state.selected_date, String::new()));
    state.mode = Mode::Insert;
}

fn enter_command_mode(state: &mut AppState) {
    state.mode = Mode::Command;
    state.command_buffer = ":".to_string();
}

fn show_help(state: &mut AppState) {
    state.mode = Mode::Command;
    state.command_buffer = ":help".to_string();
}

fn handle_gg_motion(state: &mut AppState) {
    let year = state.selected_date.year();
    let month = state.selected_date.month();
    if let Some(first) = NaiveDate::from_ymd_opt(year, month, 1) {
        state.selected_date = first;
    }
}

fn move_to_end_of_month(state: &mut AppState) {
    let year = state.selected_date.year();
    let month = state.selected_date.month();

    let next_month_first = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
    };

    if let Some(first) = next_month_first
        && let Some(last_day) = first.checked_sub_days(Days::new(1))
    {
        state.selected_date = last_day;
    }
}

fn move_previous_month(state: &mut AppState) {
    let year = state.selected_date.year();
    let month = state.selected_date.month();
    let day = state.selected_date.day();

    let (new_year, new_month) = if month == 1 {
        (year - 1, 12)
    } else {
        (year, month - 1)
    };

    let next_month_first = NaiveDate::from_ymd_opt(new_year, new_month + 1, 1)
        .or_else(|| NaiveDate::from_ymd_opt(new_year + 1, 1, 1));

    let Some(first) = next_month_first else { return };
    let Some(last) = first.checked_sub_days(Days::new(1)) else { return };

    let new_day = day.min(last.day());
    if let Some(new_date) = NaiveDate::from_ymd_opt(new_year, new_month, new_day) {
        state.selected_date = new_date;
    }
}

fn move_next_month(state: &mut AppState) {
    let year = state.selected_date.year();
    let month = state.selected_date.month();
    let day = state.selected_date.day();

    let (new_year, new_month) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };

    let next_month_first = if new_month == 12 {
        NaiveDate::from_ymd_opt(new_year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(new_year, new_month + 1, 1)
    };

    let Some(first) = next_month_first else { return };
    let Some(last) = first.checked_sub_days(Days::new(1)) else { return };

    let new_day = day.min(last.day());
    if let Some(new_date) = NaiveDate::from_ymd_opt(new_year, new_month, new_day) {
        state.selected_date = new_date;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    #[test]
    fn h_key_moves_to_previous_day() {
        let mut state = AppState::new();
        state.selected_date = date(2025, 1, 15);

        handle_key(KeyCode::Char('h'), &mut state);

        assert_eq!(state.selected_date, date(2025, 1, 14));
    }

    #[test]
    fn l_key_moves_to_next_day() {
        let mut state = AppState::new();
        state.selected_date = date(2025, 1, 15);

        handle_key(KeyCode::Char('l'), &mut state);

        assert_eq!(state.selected_date, date(2025, 1, 16));
    }

    #[test]
    fn j_key_moves_down_one_week_when_no_events() {
        let mut state = AppState::new();
        state.selected_date = date(2025, 1, 15);

        handle_key(KeyCode::Char('j'), &mut state);

        assert_eq!(state.selected_date, date(2025, 1, 22));
    }

    #[test]
    fn k_key_moves_up_one_week_when_no_events() {
        let mut state = AppState::new();
        state.selected_date = date(2025, 1, 15);

        handle_key(KeyCode::Char('k'), &mut state);

        assert_eq!(state.selected_date, date(2025, 1, 8));
    }

    #[test]
    fn t_key_jumps_to_today() {
        let mut state = AppState::new();
        state.selected_date = date(2025, 1, 1);

        handle_key(KeyCode::Char('t'), &mut state);

        assert_eq!(state.selected_date, chrono::Local::now().date_naive());
    }

    #[test]
    fn g_key_moves_to_first_day_of_month() {
        let mut state = AppState::new();
        state.selected_date = date(2025, 1, 15);

        handle_key(KeyCode::Char('g'), &mut state);

        assert_eq!(state.selected_date, date(2025, 1, 1));
    }

    #[test]
    fn shift_g_moves_to_last_day_of_month() {
        let mut state = AppState::new();
        state.selected_date = date(2025, 1, 15);

        handle_key(KeyCode::Char('G'), &mut state);

        assert_eq!(state.selected_date, date(2025, 1, 31));
    }

    #[test]
    fn left_brace_moves_to_previous_month() {
        let mut state = AppState::new();
        state.selected_date = date(2025, 2, 15);

        handle_key(KeyCode::Char('{'), &mut state);

        assert_eq!(state.selected_date, date(2025, 1, 15));
    }

    #[test]
    fn right_brace_moves_to_next_month() {
        let mut state = AppState::new();
        state.selected_date = date(2025, 1, 15);

        handle_key(KeyCode::Char('}'), &mut state);

        assert_eq!(state.selected_date, date(2025, 2, 15));
    }

    #[test]
    fn m_key_switches_to_month_view() {
        let mut state = AppState::new();
        state.view = ViewType::Day;

        handle_key(KeyCode::Char('m'), &mut state);

        assert_eq!(state.view, ViewType::Month);
    }

    #[test]
    fn w_key_switches_to_week_view() {
        let mut state = AppState::new();
        state.view = ViewType::Month;

        handle_key(KeyCode::Char('w'), &mut state);

        assert_eq!(state.view, ViewType::Week);
    }

    #[test]
    fn d_key_switches_to_day_view() {
        let mut state = AppState::new();
        state.view = ViewType::Month;

        handle_key(KeyCode::Char('d'), &mut state);

        assert_eq!(state.view, ViewType::Day);
    }

    #[test]
    fn y_key_switches_to_year_view() {
        let mut state = AppState::new();
        state.view = ViewType::Month;

        handle_key(KeyCode::Char('y'), &mut state);

        assert_eq!(state.view, ViewType::Year);
    }

    #[test]
    fn a_key_enters_insert_mode_with_form() {
        let mut state = AppState::new();
        state.mode = Mode::Normal;
        state.selected_date = date(2025, 1, 15);

        handle_key(KeyCode::Char('a'), &mut state);

        assert_eq!(state.mode, Mode::Insert);
        assert!(state.event_form.is_some());
        assert_eq!(state.event_form.as_ref().unwrap().date, date(2025, 1, 15));
    }

    #[test]
    fn colon_enters_command_mode() {
        let mut state = AppState::new();
        state.mode = Mode::Normal;

        handle_key(KeyCode::Char(':'), &mut state);

        assert_eq!(state.mode, Mode::Command);
        assert_eq!(state.command_buffer, ":");
    }
}
