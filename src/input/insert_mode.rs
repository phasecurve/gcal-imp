use crossterm::event::KeyCode;
use crate::app::{AppState, FormField};

pub fn handle_key(key: KeyCode, state: &mut AppState) {
    let Some(form) = state.event_form.as_mut() else {
        return;
    };

    match key {
        KeyCode::Tab => {
            match form.active_field {
                FormField::StartTime => form.parse_time_input(),
                FormField::Duration => form.parse_duration_input(),
                _ => {}
            }
            form.next_field();
        }
        KeyCode::BackTab => {
            match form.active_field {
                FormField::StartTime => form.parse_time_input(),
                FormField::Duration => form.parse_duration_input(),
                _ => {}
            }
            form.prev_field();
        }
        KeyCode::Backspace => {
            match form.active_field {
                FormField::Title => {
                    form.title.pop();
                }
                FormField::StartTime => {
                    form.time_input_buffer.pop();
                    form.time_buffer_touched = true;
                }
                FormField::Duration => {
                    form.duration_input_buffer.pop();
                    form.duration_buffer_touched = true;
                }
                FormField::Location => {
                    form.location.pop();
                }
                FormField::Description => {
                    form.description.pop();
                }
            }
        }
        KeyCode::Char(c) => {
            match form.active_field {
                FormField::Title => {
                    form.title.push(c);
                }
                FormField::StartTime => {
                    if c.is_ascii_digit() || c == ':' {
                        if !form.time_buffer_touched {
                            form.time_input_buffer.clear();
                            form.time_buffer_touched = true;
                        }
                        if form.time_input_buffer.len() < 5 {
                            form.time_input_buffer.push(c);
                        }
                    }
                }
                FormField::Duration => {
                    if c.is_ascii_digit() {
                        if !form.duration_buffer_touched {
                            form.duration_input_buffer.clear();
                            form.duration_buffer_touched = true;
                        }
                        if form.duration_input_buffer.len() < 5 {
                            form.duration_input_buffer.push(c);
                        }
                    }
                }
                FormField::Location => {
                    form.location.push(c);
                }
                FormField::Description => {
                    form.description.push(c);
                }
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Local;
    use crate::app::EventForm;

    fn setup_state_with_form() -> AppState {
        let mut state = AppState::new();
        state.event_form = Some(EventForm::new(
            Local::now().date_naive(),
            "Test Event".to_string(),
        ));
        state
    }

    #[test]
    fn tab_moves_to_next_field() {
        let mut state = setup_state_with_form();
        let form = state.event_form.as_ref().unwrap();
        assert_eq!(form.active_field, FormField::Title);

        handle_key(KeyCode::Tab, &mut state);
        let form = state.event_form.as_ref().unwrap();
        assert_eq!(form.active_field, FormField::StartTime);
    }

    #[test]
    fn backtab_moves_to_previous_field() {
        let mut state = setup_state_with_form();
        state.event_form.as_mut().unwrap().active_field = FormField::StartTime;

        handle_key(KeyCode::BackTab, &mut state);
        let form = state.event_form.as_ref().unwrap();
        assert_eq!(form.active_field, FormField::Title);
    }

    #[test]
    fn char_appends_to_title_field() {
        let mut state = setup_state_with_form();
        state.event_form.as_mut().unwrap().title.clear();

        handle_key(KeyCode::Char('H'), &mut state);
        handle_key(KeyCode::Char('i'), &mut state);

        let form = state.event_form.as_ref().unwrap();
        assert_eq!(form.title, "Hi");
    }

    #[test]
    fn backspace_removes_from_title() {
        let mut state = setup_state_with_form();
        state.event_form.as_mut().unwrap().title = "Hello".to_string();

        handle_key(KeyCode::Backspace, &mut state);

        let form = state.event_form.as_ref().unwrap();
        assert_eq!(form.title, "Hell");
    }

    #[test]
    fn digits_modify_time_buffer() {
        let mut state = setup_state_with_form();
        state.event_form.as_mut().unwrap().active_field = FormField::StartTime;
        state.event_form.as_mut().unwrap().time_input_buffer.clear();

        handle_key(KeyCode::Char('1'), &mut state);
        handle_key(KeyCode::Char('4'), &mut state);
        handle_key(KeyCode::Char('3'), &mut state);
        handle_key(KeyCode::Char('0'), &mut state);

        let form = state.event_form.as_ref().unwrap();
        assert_eq!(form.time_input_buffer, "1430");
    }

    #[test]
    fn time_buffer_parses_to_hour_and_minute() {
        let mut state = setup_state_with_form();
        state.event_form.as_mut().unwrap().time_input_buffer = "1430".to_string();

        state.event_form.as_mut().unwrap().parse_time_input();

        let form = state.event_form.as_ref().unwrap();
        assert_eq!(form.start_hour, 14);
        assert_eq!(form.start_minute, 30);
    }

    #[test]
    fn digits_modify_duration_buffer() {
        let mut state = setup_state_with_form();
        state.event_form.as_mut().unwrap().active_field = FormField::Duration;
        state.event_form.as_mut().unwrap().duration_input_buffer.clear();

        handle_key(KeyCode::Char('9'), &mut state);
        handle_key(KeyCode::Char('0'), &mut state);

        let form = state.event_form.as_ref().unwrap();
        assert_eq!(form.duration_input_buffer, "90");
    }
}
