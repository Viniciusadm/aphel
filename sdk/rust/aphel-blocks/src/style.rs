use serde_json::{Map, Value};

use crate::BlockError;

pub const SPACE_STEPS: usize = 9;
pub const RADII: &[&str] = &["none", "sm", "md", "lg", "full"];
pub const SHADOWS: &[&str] = &["none", "sm", "md", "lg"];
pub const BORDER_WIDTHS: &[&str] = &["0", "1", "2", "4", "8"];
pub const BORDER_STYLES: &[&str] = &["solid", "dashed", "dotted"];
pub const FONT_WEIGHTS: &[&str] = &["300", "400", "500", "600", "700", "800", "900"];
pub const LINE_HEIGHTS: &[(&str, &str)] = &[
    ("tight", "1.1"),
    ("snug", "1.3"),
    ("normal", "1.6"),
    ("relaxed", "1.8"),
    ("loose", "2"),
];
pub const LETTER_SPACINGS: &[(&str, &str)] = &[
    ("tighter", "-0.04em"),
    ("tight", "-0.02em"),
    ("normal", "0"),
    ("wide", "0.04em"),
    ("wider", "0.1em"),
];
pub const TEXT_TRANSFORMS: &[&str] = &["none", "uppercase", "lowercase", "capitalize"];
pub const TEXT_ALIGNS: &[&str] = &["left", "center", "right"];
pub const DECORATIONS: &[&str] = &["none", "underline"];
pub const WIDTHS: &[&str] = &["none", "wide", "full"];
pub const DEVICES: &[&str] = &["desktop", "tablet", "mobile"];
const SIDES: &[&str] = &["top", "right", "bottom", "left"];
const VERTICAL_SIDES: &[&str] = &["top", "bottom"];
const MAX_SLUG_CHARS: usize = 32;
const MAX_CLASSES: usize = 5;
const MAX_CLASS_CHARS: usize = 40;
const FIELD: &str = "style";

fn invalid(message: String) -> BlockError {
    BlockError::new("block_style_invalid", FIELD, message)
}

pub fn is_slug(text: &str) -> bool {
    !text.is_empty()
        && text.len() <= MAX_SLUG_CHARS
        && text.starts_with(|character: char| {
            character.is_ascii_lowercase() || character.is_ascii_digit()
        })
        && text.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
        })
}

pub fn is_hex_color(text: &str) -> bool {
    matches!(text.len(), 4 | 7 | 9)
        && text.starts_with('#')
        && text[1..].bytes().all(|byte| byte.is_ascii_hexdigit())
}

pub fn is_length(text: &str) -> bool {
    let units = ["rem", "em", "px", "%", "vw", "vh", "ch"];
    if text == "0" {
        return true;
    }
    units.iter().any(|unit| {
        text.strip_suffix(unit).is_some_and(|number| {
            !number.is_empty()
                && number.len() <= 8
                && number
                    .parse::<f64>()
                    .is_ok_and(|value| (0.0..=10_000.0).contains(&value))
                && number
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || byte == b'.')
        })
    })
}

fn string<'a>(value: &'a Value, key: &str) -> Result<&'a str, BlockError> {
    value
        .as_str()
        .ok_or_else(|| invalid(format!("{key} must be text")))
}

pub fn color(value: &Value, key: &str) -> Result<String, BlockError> {
    let text = string(value, key)?.trim();
    if is_slug(text) || is_hex_color(text) {
        Ok(text.to_ascii_lowercase())
    } else {
        Err(invalid(format!(
            "{key} must be a palette color or a #rrggbb code"
        )))
    }
}

fn slug(value: &Value, key: &str) -> Result<String, BlockError> {
    let text = string(value, key)?.trim();
    is_slug(text)
        .then(|| text.to_owned())
        .ok_or_else(|| invalid(format!("{key} is not a valid name")))
}

fn option(value: &Value, key: &str, options: &[&str]) -> Result<String, BlockError> {
    let text = string(value, key)?;
    options
        .contains(&text)
        .then(|| text.to_owned())
        .ok_or_else(|| invalid(format!("{key} is not one of the allowed values")))
}

fn keyed(value: &Value, key: &str, options: &[(&str, &str)]) -> Result<String, BlockError> {
    let names: Vec<&str> = options.iter().map(|(name, _)| *name).collect();
    option(value, key, &names)
}

fn step(value: &Value, key: &str) -> Result<String, BlockError> {
    let text = match value {
        Value::Number(number) => number.to_string(),
        Value::String(text) => text.clone(),
        _ => return Err(invalid(format!("{key} must be a spacing step"))),
    };
    text.parse::<usize>()
        .ok()
        .filter(|step| *step < SPACE_STEPS)
        .map(|step| step.to_string())
        .ok_or_else(|| {
            invalid(format!(
                "{key} must be a spacing step from 0 to {}",
                SPACE_STEPS - 1
            ))
        })
}

fn sides(value: &Value, key: &str, allowed: &[&str]) -> Result<Map<String, Value>, BlockError> {
    let Value::Object(source) = value else {
        return Err(invalid(format!("{key} must be an object")));
    };
    let mut clean = Map::new();
    for (side, step_value) in source {
        if !allowed.contains(&side.as_str()) {
            return Err(invalid(format!("{key} has an unknown side {side}")));
        }
        if !step_value.is_null() {
            clean.insert(side.clone(), Value::String(step(step_value, key)?));
        }
    }
    Ok(clean)
}

fn border(value: &Value) -> Result<Map<String, Value>, BlockError> {
    let Value::Object(source) = value else {
        return Err(invalid("border must be an object".to_owned()));
    };
    let mut clean = Map::new();
    for (key, item) in source {
        if item.is_null() {
            continue;
        }
        let cleaned = match key.as_str() {
            "width" => option(item, "border width", BORDER_WIDTHS)?,
            "style" => option(item, "border style", BORDER_STYLES)?,
            "color" => color(item, "border color")?,
            "radius" => option(item, "border radius", RADII)?,
            _ => return Err(invalid(format!("border has an unknown property {key}"))),
        };
        clean.insert(key.clone(), Value::String(cleaned));
    }
    Ok(clean)
}

fn classes(value: &Value) -> Result<String, BlockError> {
    let text = string(value, "classes")?;
    let list: Vec<&str> = text.split_whitespace().collect();
    let valid = list.len() <= MAX_CLASSES
        && list.iter().all(|class| {
            class.len() <= MAX_CLASS_CHARS
                && class.starts_with(|character: char| character.is_ascii_alphabetic())
                && class.chars().all(|character| {
                    character.is_ascii_alphanumeric() || matches!(character, '-' | '_')
                })
        });
    valid.then(|| list.join(" ")).ok_or_else(|| {
        invalid(format!(
            "use at most {MAX_CLASSES} CSS class names with letters, digits, - and _"
        ))
    })
}

fn devices(value: &Value) -> Result<Value, BlockError> {
    let Value::Array(list) = value else {
        return Err(invalid("hide_on must be a list".to_owned()));
    };
    let mut clean = Vec::new();
    for device in list {
        let device = option(device, "hide_on", DEVICES)?;
        if !clean.contains(&device) {
            clean.push(device);
        }
    }
    Ok(Value::Array(clean.into_iter().map(Value::String).collect()))
}

pub enum Scope {
    Block,
    Element,
}

pub fn normalize(value: Option<&Value>, scope: Scope) -> Result<Value, BlockError> {
    let source = match value {
        None | Some(Value::Null) => return Ok(Value::Object(Map::new())),
        Some(Value::Object(source)) => source,
        Some(_) => return Err(invalid("style must be an object".to_owned())),
    };
    let mut clean = Map::new();
    for (key, item) in source {
        if item.is_null() {
            continue;
        }
        let block_only = matches!(key.as_str(), "width" | "hide_on" | "classes" | "variation");
        if block_only && matches!(scope, Scope::Element) {
            return Err(invalid(format!("{key} is only available on a block")));
        }
        let cleaned = match key.as_str() {
            "text" | "background" | "link" => Value::String(color(item, key)?),
            "gradient" | "font_family" | "font_size" | "variation" => {
                Value::String(slug(item, key)?)
            }
            "padding" => Value::Object(sides(item, key, SIDES)?),
            "margin" => Value::Object(sides(item, key, VERTICAL_SIDES)?),
            "gap" => Value::String(step(item, key)?),
            "border" => Value::Object(border(item)?),
            "radius" => Value::String(option(item, key, RADII)?),
            "shadow" => Value::String(option(item, key, SHADOWS)?),
            "font_weight" => Value::String(option(item, key, FONT_WEIGHTS)?),
            "line_height" => Value::String(keyed(item, key, LINE_HEIGHTS)?),
            "letter_spacing" => Value::String(keyed(item, key, LETTER_SPACINGS)?),
            "text_transform" => Value::String(option(item, key, TEXT_TRANSFORMS)?),
            "text_align" => Value::String(option(item, key, TEXT_ALIGNS)?),
            "decoration" => Value::String(option(item, key, DECORATIONS)?),
            "width" => Value::String(option(item, key, WIDTHS)?),
            "hide_on" => devices(item)?,
            "classes" => Value::String(classes(item)?),
            "hover" if matches!(scope, Scope::Element) => {
                let hover = normalize(Some(item), Scope::Element)?;
                if hover
                    .as_object()
                    .is_some_and(|hover| hover.contains_key("hover"))
                {
                    return Err(invalid("hover cannot be nested".to_owned()));
                }
                hover
            }
            _ => return Err(invalid(format!("unknown style property {key}"))),
        };
        if cleaned.as_object().is_some_and(Map::is_empty) {
            continue;
        }
        clean.insert(key.clone(), cleaned);
    }
    Ok(Value::Object(clean))
}

fn color_value(reference: &str) -> String {
    if is_hex_color(reference) {
        reference.to_owned()
    } else {
        format!("var(--site-color-{reference})")
    }
}

fn space(step: &Value) -> String {
    format!("var(--site-space-{})", step.as_str().unwrap_or("0"))
}

fn radius(name: &str) -> String {
    match name {
        "none" => "0".to_owned(),
        "full" => "9999px".to_owned(),
        other => format!("var(--site-radius-{other})"),
    }
}

fn lookup(options: &[(&str, &str)], name: &str) -> String {
    options
        .iter()
        .find(|(key, _)| *key == name)
        .map(|(_, value)| (*value).to_owned())
        .unwrap_or_default()
}

pub fn declarations(style: &Value) -> Vec<String> {
    let Some(style) = style.as_object() else {
        return Vec::new();
    };
    let text = |key: &str| style.get(key).and_then(Value::as_str);
    let mut out = Vec::new();
    if let Some(value) = text("text") {
        out.push(format!("color:{}", color_value(value)));
    }
    if let Some(value) = text("background") {
        out.push(format!("background-color:{}", color_value(value)));
    }
    if let Some(value) = text("gradient") {
        out.push(format!("background-image:var(--site-gradient-{value})"));
    }
    if let Some(value) = text("link") {
        out.push(format!("--blk-link:{}", color_value(value)));
    }
    for (key, property) in [("padding", "padding"), ("margin", "margin")] {
        if let Some(Value::Object(sides)) = style.get(key) {
            for side in SIDES {
                if let Some(step) = sides.get(*side) {
                    out.push(format!("{property}-{side}:{}", space(step)));
                }
            }
        }
    }
    if let Some(step) = style.get("gap") {
        out.push(format!("--blk-gap:{}", space(step)));
    }
    if let Some(Value::Object(border)) = style.get("border") {
        let width = border.get("width").and_then(Value::as_str);
        if let Some(width) = width {
            out.push(format!(
                "border-width:{}",
                if width == "0" {
                    "0".to_owned()
                } else {
                    format!("{width}px")
                }
            ));
            out.push(format!(
                "border-style:{}",
                border
                    .get("style")
                    .and_then(Value::as_str)
                    .unwrap_or("solid")
            ));
        }
        if let Some(color) = border.get("color").and_then(Value::as_str) {
            out.push(format!("border-color:{}", color_value(color)));
        }
        if let Some(name) = border.get("radius").and_then(Value::as_str) {
            out.push(format!("border-radius:{}", radius(name)));
        }
    }
    if let Some(name) = text("radius") {
        out.push(format!("border-radius:{}", radius(name)));
    }
    if let Some(name) = text("shadow") {
        out.push(format!(
            "box-shadow:{}",
            if name == "none" {
                "none".to_owned()
            } else {
                format!("var(--site-shadow-{name})")
            }
        ));
    }
    if let Some(value) = text("font_family") {
        out.push(format!("font-family:var(--site-font-{value})"));
    }
    if let Some(value) = text("font_size") {
        out.push(format!("font-size:var(--site-size-{value})"));
    }
    if let Some(value) = text("font_weight") {
        out.push(format!("font-weight:{value}"));
    }
    if let Some(value) = text("line_height") {
        out.push(format!("line-height:{}", lookup(LINE_HEIGHTS, value)));
    }
    if let Some(value) = text("letter_spacing") {
        out.push(format!("letter-spacing:{}", lookup(LETTER_SPACINGS, value)));
    }
    if let Some(value) = text("text_transform") {
        out.push(format!("text-transform:{value}"));
    }
    if let Some(value) = text("text_align") {
        out.push(format!("text-align:{value}"));
    }
    if let Some(value) = text("decoration") {
        out.push(format!("text-decoration-line:{value}"));
    }
    out
}

pub fn classes_of(style: &Value) -> Vec<String> {
    let Some(style) = style.as_object() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    match style.get("width").and_then(Value::as_str) {
        Some("wide") => out.push("is-wide".to_owned()),
        Some("full") => out.push("is-full".to_owned()),
        _ => {}
    }
    if let Some(Value::Array(devices)) = style.get("hide_on") {
        out.extend(
            devices
                .iter()
                .filter_map(Value::as_str)
                .map(|device| format!("hide-{device}")),
        );
    }
    if style.contains_key("link") {
        out.push("has-link-color".to_owned());
    }
    if let Some(variation) = style.get("variation").and_then(Value::as_str) {
        out.push(format!("is-style-{variation}"));
    }
    if let Some(classes) = style.get("classes").and_then(Value::as_str) {
        out.extend(classes.split_whitespace().map(str::to_owned));
    }
    out
}
