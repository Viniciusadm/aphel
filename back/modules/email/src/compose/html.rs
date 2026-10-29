use std::sync::LazyLock;

use regex::{Captures, Regex};

const DANGEROUS_BLOCK_TAGS: [&str; 6] =
    ["script", "iframe", "object", "applet", "form", "frameset"];
const HIDDEN_BLOCK_TAGS: [&str; 4] = ["style", "head", "title", "script"];

static DANGEROUS_BLOCKS: LazyLock<Vec<Regex>> =
    LazyLock::new(|| block_patterns(&DANGEROUS_BLOCK_TAGS));
static HIDDEN_BLOCKS: LazyLock<Vec<Regex>> = LazyLock::new(|| block_patterns(&HIDDEN_BLOCK_TAGS));
static DANGEROUS_TAGS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?is)</?\s*(script|iframe|object|embed|applet|form|frameset|frame|base|meta|link)\b[^>]*>",
    )
    .expect("valid regex")
});
static EVENT_ATTRIBUTES: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?is)\s+on[a-z]+\s*=\s*("[^"]*"|'[^']*'|[^\s>]+)"#).expect("valid regex")
});
static SCRIPT_URLS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?is)\b(href|src|action|formaction|background|xlink:href)\s*=\s*("\s*(?:javascript|vbscript|data:text/html)[^"]*"|'\s*(?:javascript|vbscript|data:text/html)[^']*'|(?:javascript|vbscript):[^\s>]*)"#,
    )
    .expect("valid regex")
});
static LINE_BREAKS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)<br\s*/?>").expect("valid regex"));
static BLOCK_ENDS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)</\s*(p|div|li|tr|h[1-6]|blockquote|table|ul|ol)\s*>").expect("valid regex")
});
static LIST_ITEMS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)<li\b[^>]*>").expect("valid regex"));
static TAGS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?s)<[^>]*>").expect("valid regex"));
static ANCHOR_HREFS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?is)(<a\b[^>]*?\bhref\s*=\s*)("([^"]*)"|'([^']*)')"#).expect("valid regex")
});
static BODY_END: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)</\s*body\s*>").expect("valid regex"));

fn block_patterns(tags: &[&str]) -> Vec<Regex> {
    tags.iter()
        .map(|tag| {
            Regex::new(&format!(r"(?is)<{tag}\b[^>]*>.*?</\s*{tag}\s*>")).expect("valid regex")
        })
        .collect()
}

fn remove_blocks(html: &str, patterns: &[Regex]) -> String {
    patterns.iter().fold(html.to_owned(), |html, pattern| {
        pattern.replace_all(&html, "").into_owned()
    })
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

pub(crate) fn decode_entities(value: &str) -> String {
    value
        .replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&#x27;", "'")
        .replace("&amp;", "&")
}

pub fn sanitize_html(html: &str) -> String {
    let html = remove_blocks(html, &DANGEROUS_BLOCKS);
    let html = DANGEROUS_TAGS.replace_all(&html, "");
    let html = EVENT_ATTRIBUTES.replace_all(&html, "");
    SCRIPT_URLS
        .replace_all(&html, |captures: &Captures| {
            format!("{}=\"#\"", &captures[1])
        })
        .into_owned()
}

pub(crate) fn html_to_text(html: &str) -> String {
    let text = remove_blocks(html, &HIDDEN_BLOCKS);
    let text = LINE_BREAKS.replace_all(&text, "\n");
    let text = LIST_ITEMS.replace_all(&text, "- ");
    let text = BLOCK_ENDS.replace_all(&text, "\n");
    let text = TAGS.replace_all(&text, "");
    let text = decode_entities(&text);
    let mut lines: Vec<String> = Vec::new();
    for line in text.lines() {
        let line = line.split_whitespace().collect::<Vec<_>>().join(" ");
        if line.is_empty() && lines.last().is_none_or(String::is_empty) {
            continue;
        }
        lines.push(line);
    }
    while lines.last().is_some_and(String::is_empty) {
        lines.pop();
    }
    lines.join("\n")
}

pub(crate) fn is_trackable_url(url: &str) -> bool {
    let lower = url.trim().to_ascii_lowercase();
    (lower.starts_with("http://") || lower.starts_with("https://"))
        && !lower.contains(char::is_whitespace)
}

pub(crate) fn rewrite_links(html: &str, mut tracked: impl FnMut(&str) -> Option<String>) -> String {
    ANCHOR_HREFS
        .replace_all(html, |captures: &Captures| {
            let raw = captures
                .get(3)
                .or_else(|| captures.get(4))
                .map(|value| value.as_str())
                .unwrap_or_default();
            let url = decode_entities(raw.trim());
            match is_trackable_url(&url).then(|| tracked(&url)).flatten() {
                Some(replacement) => {
                    format!("{}\"{}\"", &captures[1], escape_html(&replacement))
                }
                None => captures[0].to_owned(),
            }
        })
        .into_owned()
}

pub(crate) fn append_pixel(html: &str, pixel_url: &str) -> String {
    let pixel = format!(
        "<img src=\"{}\" width=\"1\" height=\"1\" alt=\"\" style=\"display:block;width:1px;height:1px;border:0\">",
        escape_html(pixel_url)
    );
    match BODY_END.find(html) {
        Some(found) => format!(
            "{}{pixel}{}",
            &html[..found.start()],
            &html[found.start()..]
        ),
        None => format!("{html}{pixel}"),
    }
}
