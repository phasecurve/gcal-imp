use chrono::Days;
use crossterm::event::KeyCode;

use crate::app::{AppState, EventForm, Mode};

pub fn handle_key(key: KeyCode, state: &mut AppState) {
    match key {
        KeyCode::Char('h') => {
            if let Some(new_date) = state.selected_date.checked_sub_days(Days::new(1)) {
                state.selected_date = new_date;
            }
        }
        KeyCode::Char('l') => {
            if let Some(new_date) = state.selected_date.checked_add_days(Days::new(1)) {
                state.selected_date = new_date;
            }
        }
        KeyCode::Char('j') => {
            if let Some(new_date) = state.selected_date.checked_add_days(Days::new(7)) {
                state.selected_date = new_date;
            }
        }
        KeyCode::Char('k') => {
            if let Some(new_date) = state.selected_date.checked_sub_days(Days::new(7)) {
                state.selected_date = new_date;
            }
        }
        KeyCode::Char('a') | KeyCode::Enter => {
            if let Some((start, end)) = state.get_visual_selection_range() {
                let duration_days = (end - start).num_days() as u32 + 1;
                let title = if duration_days == 1 {
                    String::from("New Event")
                } else {
                    format!("{}-day Event", duration_days)
                };

                let form = if duration_days == 1 {
                    EventForm::new(start, title)
                } else {
                    EventForm::new_all_day(start, title, duration_days)
                };

                state.event_form = Some(form);
                state.visual_selection_start = None;
                state.mode = Mode::Insert;
            }
        }
        KeyCode::Esc => {
            state.visual_selection_start = None;
            state.mode = Mode::Normal;
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_visual_state() -> AppState {
        let mut state = AppState::new();
        let start_date = chrono::NaiveDate::from_ymd_opt(2025, 1, 10).unwrap();
        state.selected_date = start_date;
        state.visual_selection_start = Some(start_date);
        state.mode = Mode::Visual;
        state
    }

    #[test]
    fn h_key_moves_selection_left() {
        let mut state = setup_visual_state();
        let start = state.selected_date;

        handle_key(KeyCode::Char('h'), &mut state);

        assert_eq!(state.selected_date, start.checked_sub_days(Days::new(1)).unwrap());
    }

    #[test]
    fn l_key_moves_selection_right() {
        let mut state = setup_visual_state();
        let start = state.selected_date;

        handle_key(KeyCode::Char('l'), &mut state);

        assert_eq!(state.selected_date, start.checked_add_days(Days::new(1)).unwrap());
    }

    #[test]
    fn esc_exits_visual_mode() {
        let mut state = setup_visual_state();

        handle_key(KeyCode::Esc, &mut state);

        assert_eq!(state.mode, Mode::Normal);
        assert_eq!(state.visual_selection_start, None);
    }

    #[test]
    fn enter_creates_multiday_event_form_for_visual_range() {
        let mut state = setup_visual_state();
        state.selected_date = state.selected_date.checked_add_days(Days::new(2)).unwrap();

        handle_key(KeyCode::Enter, &mut state);

        assert_eq!(state.mode, Mode::Insert);
        assert!(state.event_form.is_some());
        let form = state.event_form.as_ref().unwrap();
        assert_eq!(form.duration_minutes, 3 * 24 * 60);
        assert_eq!(form.all_day, true);
    }
}
