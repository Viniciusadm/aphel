use serde_json::{Map, Value};

pub(crate) const MAX_BODY_BYTES: usize = 256 * 1024;
pub(crate) const MAX_LOGGED_BODY_BYTES: usize = 8 * 1024;
pub(crate) const MAX_LOGGED_HEADER_LENGTH: usize = 512;

pub(crate) const LOGGED_HEADERS: [&str; 12] = [
    "content-type",
    "content-length",
    "user-agent",
    "origin",
    "referer",
    "x-forwarded-for",
    "x-real-ip",
    "x-request-id",
    "x-aphel-delivery",
    "x-aphel-event",
    "x-github-event",
    "x-github-delivery",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PayloadFormat {
    Json,
    Form,
}

pub(crate) fn detect_format(content_type: Option<&str>) -> Option<PayloadFormat> {
    let media_type = content_type
        .and_then(|value| value.split(';').next())
        .map(|value| value.trim().to_ascii_lowercase())
        .unwrap_or_default();
    match media_type.as_str() {
        "" | "application/json" | "text/plain" => Some(PayloadFormat::Json),
        "application/x-www-form-urlencoded" => Some(PayloadFormat::Form),
        other if other.ends_with("+json") => Some(PayloadFormat::Json),
        _ => None,
    }
}

pub(crate) fn parse_payload(content_type: Option<&str>, body: &[u8]) -> Result<Value, String> {
    match detect_format(content_type) {
        Some(PayloadFormat::Json) => {
            if body.iter().all(u8::is_ascii_whitespace) {
                return Ok(Value::Object(Map::new()));
            }
            match serde_json::from_slice::<Value>(body) {
                Ok(value @ (Value::Object(_) | Value::Array(_))) => Ok(value),
                Ok(_) => Err("the JSON body must be an object or an array".to_owned()),
                Err(error) => Err(format!("the body is not valid JSON: {error}")),
            }
        }
        Some(PayloadFormat::Form) => Ok(parse_form(body)),
        None => Err(
            "unsupported Content-Type; send application/json or application/x-www-form-urlencoded"
                .to_owned(),
        ),
    }
}

fn parse_form(body: &[u8]) -> Value {
    let mut object = Map::new();
    for (key, value) in form_urlencoded::parse(body) {
        let key = key.trim_end_matches("[]").to_owned();
        let value = Value::String(value.into_owned());
        match object.get_mut(&key) {
            Some(Value::Array(items)) => items.push(value),
            Some(existing) => {
                let previous = existing.take();
                *existing = Value::Array(vec![previous, value]);
            }
            None => {
                object.insert(key, value);
            }
        }
    }
    Value::Object(object)
}

pub(crate) fn logged_headers<'a>(
    headers: impl IntoIterator<Item = (&'a str, &'a str)>,
) -> Map<String, Value> {
    let mut logged = Map::new();
    for (name, value) in headers {
        let name = name.to_ascii_lowercase();
        if LOGGED_HEADERS.contains(&name.as_str()) {
            logged.insert(
                name,
                Value::String(value.chars().take(MAX_LOGGED_HEADER_LENGTH).collect()),
            );
        }
    }
    logged
}

pub(crate) fn truncated_body(body: &[u8]) -> (String, bool) {
    let text = String::from_utf8_lossy(body);
    if text.len() <= MAX_LOGGED_BODY_BYTES {
        return (text.into_owned(), false);
    }
    let mut end = MAX_LOGGED_BODY_BYTES;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    (text[..end].to_owned(), true)
}
