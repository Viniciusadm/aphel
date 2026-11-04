use chrono::{DateTime, NaiveDateTime, SecondsFormat, Utc};

pub const EXCERPT_CHARS: usize = 280;
pub const PREVIEW_CHARS: usize = 160;

pub fn truncate(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        value.to_owned()
    } else {
        let mut text: String = value.chars().take(max.saturating_sub(1)).collect();
        text.push('…');
        text
    }
}

pub fn single_line(value: &str, max: usize) -> String {
    let collapsed = value.split_whitespace().collect::<Vec<_>>().join(" ");
    truncate(&collapsed, max)
}

pub fn clean_body(value: &str) -> String {
    let normalized = value.replace("\r\n", "\n").replace('\r', "\n");
    let cleaned: String = normalized
        .chars()
        .filter(|character| !character.is_control() || *character == '\n' || *character == '\t')
        .collect();
    cleaned.trim().to_owned()
}

pub fn normalize_phone(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    let digits: String = trimmed.chars().filter(char::is_ascii_digit).collect();
    let allowed = trimmed
        .chars()
        .all(|character| character.is_ascii_digit() || " +-().".contains(character));
    if !allowed || !(8..=15).contains(&digits.len()) || digits.starts_with('0') {
        return None;
    }
    Some(format!("+{digits}"))
}

pub fn normalize_email(raw: &str) -> Option<String> {
    let email = raw
        .trim()
        .trim_start_matches('<')
        .trim_end_matches('>')
        .to_lowercase();
    let (local, domain) = email.split_once('@')?;
    let valid = !local.is_empty()
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
        && email.len() <= 320
        && !email.chars().any(|character| {
            character.is_whitespace() || character.is_control() || "<>,;\"".contains(character)
        });
    valid.then_some(email)
}

pub fn normalize_message_id(raw: &str) -> Option<String> {
    let value = raw
        .trim()
        .trim_start_matches('<')
        .trim_end_matches('>')
        .trim();
    (!value.is_empty() && value.len() <= 250 && !value.chars().any(char::is_whitespace))
        .then(|| value.to_owned())
}

pub fn timestamp(value: NaiveDateTime) -> String {
    value.and_utc().to_rfc3339_opts(SecondsFormat::Secs, true)
}

pub fn now() -> NaiveDateTime {
    Utc::now().naive_utc()
}

pub fn from_unix(seconds: &str) -> Option<NaiveDateTime> {
    seconds
        .trim()
        .parse::<i64>()
        .ok()
        .and_then(|seconds| DateTime::from_timestamp(seconds, 0))
        .map(|value| value.naive_utc())
}

pub fn decode_hex(value: &str) -> Option<Vec<u8>> {
    if !value.len().is_multiple_of(2) {
        return None;
    }
    (0..value.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(value.get(index..index + 2)?, 16).ok())
        .collect()
}

pub fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right)
        .fold(0u8, |difference, (a, b)| difference | (a ^ b))
        == 0
}

pub fn split_name(name: &str) -> (String, Option<String>) {
    let mut parts = name.trim().splitn(2, char::is_whitespace);
    let first = parts.next().unwrap_or_default().trim().to_owned();
    let last = parts
        .next()
        .map(|last| last.trim().to_owned())
        .filter(|last| !last.is_empty());
    (first, last)
}

pub fn escape_html(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            other => escaped.push(other),
        }
    }
    escaped
}
