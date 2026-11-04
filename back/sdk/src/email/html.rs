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

pub use crate::messaging::text::escape_html;

pub fn decode_entities(value: &str) -> String {
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

pub fn html_to_text(html: &str) -> String {
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
