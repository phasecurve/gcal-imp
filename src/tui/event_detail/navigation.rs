pub fn next_word_position(lines: &[String], line_idx: usize, col: usize) -> (usize, usize) {
    if lines.is_empty() {
        return (0, 0);
    }

    let mut line_idx = line_idx.min(lines.len() - 1);
    let mut col = col;

    loop {
        if line_idx >= lines.len() {
            let last_idx = lines.len() - 1;
            return (last_idx, last_char_index(&lines[last_idx]));
        }

        let line_text = &lines[line_idx];
        let len = line_text.chars().count();

        if len == 0 || col >= len {
            if line_idx + 1 >= lines.len() {
                return (line_idx, 0);
            }
            line_idx += 1;
            let next_line = &lines[line_idx];
            if let Some(first_non_ws) = next_line.chars().position(|c| !c.is_whitespace()) {
                return (line_idx, first_non_ws);
            } else {
                col = 0;
                continue;
            }
        }

        if let Some(next_col) = find_next_word_start(line_text, col) {
            return (line_idx, next_col);
        }

        if line_idx + 1 >= lines.len() {
            return (line_idx, last_char_index(line_text));
        }

        line_idx += 1;
        let next_line = &lines[line_idx];
        if let Some(first_non_ws) = next_line.chars().position(|c| !c.is_whitespace()) {
            return (line_idx, first_non_ws);
        } else {
            col = 0;
        }
    }
}

pub fn word_end_position(lines: &[String], line_idx: usize, col: usize) -> (usize, usize) {
    if lines.is_empty() {
        return (0, 0);
    }

    let mut line_idx = line_idx.min(lines.len() - 1);
    let mut col = col;

    loop {
        if line_idx >= lines.len() {
            let last_idx = lines.len() - 1;
            return (last_idx, last_char_index(&lines[last_idx]));
        }

        let line_text = &lines[line_idx];
        let len = line_text.chars().count();

        if len == 0 || col >= len {
            if line_idx + 1 >= lines.len() {
                return (line_idx, 0);
            }
            line_idx += 1;
            col = 0;
            continue;
        }

        if let Some(end_col) = find_word_end(line_text, col) {
            return (line_idx, end_col);
        }

        if line_idx + 1 >= lines.len() {
            return (line_idx, last_char_index(line_text));
        }

        line_idx += 1;
        col = 0;
    }
}

pub fn prev_word_position(lines: &[String], line_idx: usize, col: usize) -> (usize, usize) {
    if lines.is_empty() {
        return (0, 0);
    }

    let mut line_idx = line_idx.min(lines.len() - 1);
    let mut col = col;

    loop {
        if line_idx >= lines.len() {
            line_idx = lines.len() - 1;
            col = lines[line_idx].chars().count();
        }

        let line_text = &lines[line_idx];
        let len = line_text.chars().count();

        let safe_col = if len == 0 { 0 } else { col.min(len) };

        if let Some(prev_col) = find_prev_word_start(line_text, safe_col) {
            return (line_idx, prev_col);
        }

        if line_idx == 0 {
            return (0, 0);
        }

        line_idx -= 1;
        col = lines[line_idx].chars().count();
    }
}

pub fn last_char_index(text: &str) -> usize {
    text.chars().count().saturating_sub(1)
}

pub fn find_first_non_whitespace(text: &str) -> usize {
    text.chars().position(|c| !c.is_whitespace()).unwrap_or(0)
}

fn is_word_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

fn find_next_word_start(text: &str, current_col: usize) -> Option<usize> {
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() {
        return None;
    }

    let len = chars.len();
    let mut pos = current_col.min(len.saturating_sub(1));

    if chars[pos].is_whitespace() {
        while pos < len && chars[pos].is_whitespace() {
            pos += 1;
        }
    } else if is_word_char(chars[pos]) {
        while pos < len && is_word_char(chars[pos]) {
            pos += 1;
        }
    } else {
        while pos < len && !chars[pos].is_whitespace() && !is_word_char(chars[pos]) {
            pos += 1;
        }
    }

    while pos < len && chars[pos].is_whitespace() {
        pos += 1;
    }

    (pos < len).then_some(pos)
}

fn find_word_end(text: &str, current_col: usize) -> Option<usize> {
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() {
        return None;
    }

    let len = chars.len();
    let mut pos = current_col.saturating_add(1);

    while pos < len && chars[pos].is_whitespace() {
        pos += 1;
    }

    if pos >= len {
        return None;
    }

    let is_word = is_word_char(chars[pos]);

    while pos < len && !chars[pos].is_whitespace() {
        if is_word_char(chars[pos]) != is_word {
            break;
        }
        pos += 1;
    }

    Some(pos.saturating_sub(1).min(len.saturating_sub(1)))
}

fn find_prev_word_start(text: &str, current_col: usize) -> Option<usize> {
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() {
        return None;
    }

    let len = chars.len();
    let mut pos = current_col.min(len);

    if pos == 0 {
        return None;
    }

    pos -= 1;

    while pos > 0 && chars[pos].is_whitespace() {
        pos -= 1;
    }

    if chars[pos].is_whitespace() {
        return None;
    }

    let is_word = is_word_char(chars[pos]);

    while pos > 0 {
        let prev = chars[pos - 1];
        if prev.is_whitespace() {
            break;
        }
        let prev_is_word = is_word_char(prev);
        if prev_is_word != is_word {
            break;
        }
        pos -= 1;
    }

    Some(pos)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_lines() -> Vec<String> {
        vec![
            "alpha beta".to_string(),
            "gamma delta".to_string(),
        ]
    }

    #[test]
    fn next_word_wraps_to_next_line() {
        let lines = sample_lines();
        let (line, col) = next_word_position(
            &lines,
            0,
            lines[0].chars().count().saturating_sub(1),
        );

        assert_eq!(line, 1);
        assert_eq!(col, 0);
    }

    #[test]
    fn prev_word_moves_to_previous_line() {
        let lines = sample_lines();
        let (line, col) = prev_word_position(&lines, 1, 0);

        assert_eq!(line, 0);
        assert_eq!(col, 6);
    }

    #[test]
    fn word_end_wraps_to_next_line() {
        let lines = sample_lines();
        let (line, col) = word_end_position(
            &lines,
            0,
            lines[0].chars().count().saturating_sub(1),
        );

        assert_eq!(line, 1);
        assert_eq!(col, 4);
    }
}
