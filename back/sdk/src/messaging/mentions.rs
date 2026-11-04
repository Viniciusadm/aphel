use std::sync::LazyLock;

use regex::Regex;

static MENTION: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"@\[([^\[\]\n]{1,200})\]\(user:([0-9A-Fa-f]{8}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{12})\)")
        .expect("the mention pattern is valid")
});

pub fn extract_mentions(body: &str) -> Vec<String> {
    let mut ids: Vec<String> = Vec::new();
    for captures in MENTION.captures_iter(body) {
        let id = captures[2].to_ascii_lowercase();
        if !ids.contains(&id) {
            ids.push(id);
        }
    }
    ids
}

pub fn mentions_to_text(body: &str) -> std::borrow::Cow<'_, str> {
    MENTION.replace_all(body, "@$1")
}
