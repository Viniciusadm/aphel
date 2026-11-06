use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

use regex::Regex;
use serde::Deserialize;
use serde_json::Value;

const CONTRACT_JSON: &str = include_str!("../tokens.json");
const MAX_VALUE_LENGTH: usize = 300;
const MAX_FONT_FAMILIES: usize = 6;
const FORBIDDEN_FRAGMENTS: [&str; 12] = [
    ";",
    "{",
    "}",
    "\\",
    "\"",
    "'",
    "<",
    ">",
    "@",
    "/*",
    "url(",
    "expression",
];
const GENERIC_FALLBACKS: [&str; 7] = [
    "sans-serif",
    "serif",
    "monospace",
    "system-ui",
    "ui-sans-serif",
    "ui-serif",
    "ui-monospace",
];
const SYSTEM_FAMILIES: [&str; 17] = [
    "Inter",
    "Arial",
    "Helvetica",
    "Georgia",
    "Times New Roman",
    "Courier New",
    "Verdana",
    "Tahoma",
    "Trebuchet MS",
    "Segoe UI",
    "Roboto",
    "Menlo",
    "Consolas",
    "Monaco",
    "SF Pro Text",
    "Helvetica Neue",
    "Noto Sans",
];

static HEX_COLOR: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^#(?:[0-9a-fA-F]{3,4}|[0-9a-fA-F]{6}|[0-9a-fA-F]{8})$").expect("valid regex")
});
static FUNCTION_COLOR: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?:rgb|rgba|hsl|hsla|oklch|oklab)\(\s*[0-9A-Za-z.%+\-/,\s]{1,80}\)$")
        .expect("valid regex")
});
static VARIABLE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^var\(--([a-z0-9-]+)\)$").expect("valid regex"));
static LENGTH: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^-?(?:0|\d+(?:\.\d+)?(?:px|rem|em|%))$").expect("valid regex"));
static SHADOW_LENGTH: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^-?(?:0|\d+(?:\.\d+)?(?:px|rem))$").expect("valid regex"));
static RADIUS_CALC: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^calc\(var\(--radius\)\s*[+-]\s*\d+(?:\.\d+)?(?:px|rem|em)\)$")
        .expect("valid regex")
});
static DURATION: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\d+(?:\.\d+)?(?:ms|s)$").expect("valid regex"));
static FAMILY_NAME: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[A-Za-z0-9][A-Za-z0-9 _-]{0,63}$").expect("valid regex"));

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TokenKind {
    Color,
    Length,
    Shadow,
    Duration,
    Font,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct TokenDefinition {
    pub(crate) name: String,
    pub(crate) kind: TokenKind,
    pub(crate) org_light: Option<String>,
    pub(crate) org_dark: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Contract {
    tokens: Vec<TokenDefinition>,
}

static CONTRACT: LazyLock<Vec<TokenDefinition>> = LazyLock::new(|| {
    serde_json::from_str::<Contract>(CONTRACT_JSON)
        .expect("tokens.json is valid")
        .tokens
});
static INDEX: LazyLock<HashMap<&'static str, usize>> = LazyLock::new(|| {
    CONTRACT
        .iter()
        .enumerate()
        .map(|(position, token)| (token.name.as_str(), position))
        .collect()
});

pub(crate) fn definition(name: &str) -> Option<&'static TokenDefinition> {
    INDEX.get(name).map(|position| &CONTRACT[*position])
}

pub(crate) fn render_value(
    token: &TokenDefinition,
    value: &Value,
    bundled_families: &HashSet<String>,
) -> Result<String, String> {
    if token.kind == TokenKind::Font {
        return render_font(token, value, bundled_families);
    }
    let text = value
        .as_str()
        .ok_or_else(|| "must be a string".to_owned())?
        .trim();
    check_text(text)?;
    if let Some(reference) = VARIABLE.captures(text) {
        return render_reference(token, &reference[1], text);
    }
    match token.kind {
        TokenKind::Color => check_color(text),
        TokenKind::Length => check_length(text),
        TokenKind::Duration => check_duration(text),
        TokenKind::Shadow => check_shadow(text),
        TokenKind::Font => unreachable!("fonts are rendered above"),
    }
    .map(|()| text.to_owned())
}

pub(crate) fn wrap_organization_override(value: &str, property: Option<&str>) -> String {
    match property {
        Some(property) => format!("var(--{property}, {value})"),
        None => value.to_owned(),
    }
}

fn check_text(text: &str) -> Result<(), String> {
    if text.is_empty() {
        return Err("must not be empty".to_owned());
    }
    if text.len() > MAX_VALUE_LENGTH {
        return Err(format!("must have at most {MAX_VALUE_LENGTH} characters"));
    }
    let lowered = text.to_ascii_lowercase();
    if FORBIDDEN_FRAGMENTS
        .iter()
        .any(|fragment| lowered.contains(fragment))
    {
        return Err("contains a character or function that is not allowed".to_owned());
    }
    Ok(())
}

fn render_reference(token: &TokenDefinition, name: &str, text: &str) -> Result<String, String> {
    match definition(name) {
        Some(target) if target.kind == token.kind => Ok(text.to_owned()),
        Some(_) => Err(format!("--{name} is not a token of the same kind")),
        None => Err(format!("--{name} is not a design token")),
    }
}

fn check_color(text: &str) -> Result<(), String> {
    let accepted = HEX_COLOR.is_match(text)
        || FUNCTION_COLOR.is_match(text)
        || matches!(text, "transparent" | "currentColor");
    accepted
        .then_some(())
        .ok_or_else(|| "must be a hex, rgb(), hsl(), oklch() or oklab() color".to_owned())
}

fn check_length(text: &str) -> Result<(), String> {
    (LENGTH.is_match(text) || RADIUS_CALC.is_match(text))
        .then_some(())
        .ok_or_else(|| {
            "must be a length such as 4px, 0.5rem or calc(var(--radius) - 2px)".to_owned()
        })
}

fn check_duration(text: &str) -> Result<(), String> {
    DURATION
        .is_match(text)
        .then_some(())
        .ok_or_else(|| "must be a duration such as 150ms or 0.2s".to_owned())
}

fn check_shadow(text: &str) -> Result<(), String> {
    if text == "none" {
        return Ok(());
    }
    split_top_level(text, |character| character == ',')
        .iter()
        .try_for_each(|layer| check_shadow_layer(layer.trim()))
}

fn check_shadow_layer(layer: &str) -> Result<(), String> {
    let invalid = || "must be a list of shadows: [inset] x y [blur [spread]] color".to_owned();
    let mut parts = split_top_level(layer, char::is_whitespace);
    parts.retain(|part| !part.is_empty());
    if parts.first().is_some_and(|part| part == "inset") {
        parts.remove(0);
    }
    let color = parts.pop().ok_or_else(invalid)?;
    if !(2..=4).contains(&parts.len()) || !parts.iter().all(|part| SHADOW_LENGTH.is_match(part)) {
        return Err(invalid());
    }
    check_color(&color).map_err(|_| invalid())
}

fn split_top_level(text: &str, is_separator: impl Fn(char) -> bool) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut depth = 0usize;
    for character in text.chars() {
        match character {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            _ => {}
        }
        if depth == 0 && is_separator(character) {
            parts.push(std::mem::take(&mut current));
        } else {
            current.push(character);
        }
    }
    parts.push(current);
    parts
}

fn render_font(
    token: &TokenDefinition,
    value: &Value,
    bundled_families: &HashSet<String>,
) -> Result<String, String> {
    if let Some(text) = value.as_str() {
        let text = text.trim();
        check_text(text)?;
        return VARIABLE
            .captures(text)
            .ok_or_else(|| "must be an object with `families` and `fallback`".to_owned())
            .and_then(|reference| render_reference(token, &reference[1], text));
    }
    let object = value
        .as_object()
        .ok_or_else(|| "must be an object with `families` and `fallback`".to_owned())?;
    let families = object
        .get("families")
        .and_then(Value::as_array)
        .filter(|families| (1..=MAX_FONT_FAMILIES).contains(&families.len()))
        .ok_or_else(|| format!("`families` must list 1 to {MAX_FONT_FAMILIES} font names"))?;
    let fallback = object
        .get("fallback")
        .and_then(Value::as_str)
        .filter(|fallback| GENERIC_FALLBACKS.contains(fallback))
        .ok_or_else(|| format!("`fallback` must be one of {}", GENERIC_FALLBACKS.join(", ")))?;
    let system: HashSet<&str> = SYSTEM_FAMILIES.into_iter().collect();
    let mut rendered = Vec::with_capacity(families.len() + 1);
    for family in families {
        let name = family
            .as_str()
            .filter(|name| FAMILY_NAME.is_match(name))
            .ok_or_else(|| "font names use letters, digits, spaces, `_` and `-`".to_owned())?;
        if !bundled_families.contains(name) && !system.contains(name) {
            return Err(format!(
                "font {name} is neither bundled in `fonts` nor a known system font"
            ));
        }
        rendered.push(format!("'{name}'"));
    }
    rendered.push(fallback.to_owned());
    Ok(rendered.join(", "))
}
