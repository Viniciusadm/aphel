pub use aphel_sdk::email::html::{decode_entities, escape_html, html_to_text, sanitize_html};
use regex::{Captures, Regex};
use std::sync::LazyLock;

static ANCHOR_HREFS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?is)(<a\b[^>]*?\bhref\s*=\s*)("([^"]*)"|'([^']*)')"#).expect("valid regex")
});
static BODY_END: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)</\s*body\s*>").expect("valid regex"));

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
