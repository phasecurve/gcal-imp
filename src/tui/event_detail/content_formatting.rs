use std::sync::OnceLock;
use html2text;
use regex::Regex;

pub fn strip_html(html: &str) -> String {
    let normalized = expand_anchor_tags(html);
    html2text::from_read(normalized.as_bytes(), 1000)
}

fn expand_anchor_tags(html: &str) -> String {
    static LINK_RE: OnceLock<Regex> = OnceLock::new();
    let regex = LINK_RE.get_or_init(|| {
        Regex::new(r#"(?is)<a\s+[^>]*?href=["']([^"']+)["'][^>]*>(.*?)</a>"#)
            .expect("invalid anchor regex")
    });

    regex
        .replace_all(html, |caps: &regex::Captures| {
            let url = caps.get(1).map(|m| m.as_str()).unwrap_or_default().trim();
            let text = caps.get(2).map(|m| m.as_str()).unwrap_or_default().trim();

            if text.is_empty() {
                url.to_string()
            } else if url.eq_ignore_ascii_case(text) {
                url.to_string()
            } else {
                format!("{text} ({url})")
            }
        })
        .into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anchor_tags_include_url_after_strip() {
        let html = r#"<p>Visit <a href="https://example.com">Example</a> now.</p>"#;
        let text = strip_html(html);
        assert!(text.contains("Example (https://example.com)"));
    }

    #[test]
    fn anchor_without_text_falls_back_to_url() {
        let html = r#"<a href="https://example.com"></a>"#;
        let text = strip_html(html);
        assert!(text.contains("https://example.com"));
    }
}
