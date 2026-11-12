use std::collections::BTreeMap;

use crate::error::BlockError;
use serde_json::{Map, Value};

pub const MAX_BLOCKS: usize = 300;
pub const MAX_JSON_CHARS: usize = 400_000;
const MAX_BUTTONS: usize = 3;
const MAX_ID_CHARS: usize = 40;
const MAX_LINK_CHARS: usize = 500;
const MAX_IMAGE_ID_CHARS: usize = 36;
const MAX_EMAIL_CHARS: usize = 200;
const MAX_PHONE_CHARS: usize = 40;
const MAX_VIDEO_URL_CHARS: usize = 300;
const FIELD: &str = "blocks";

enum Kind {
    Text(usize),
    Image,
    Link,
    Choice(&'static [&'static str]),
    Flag,
    Buttons,
    Items(usize, &'static [(&'static str, Kind)]),
    Email,
    Phone,
    Video,
    Embed,
    Key,
    Identifier,
}

const ALIGNMENTS: &[&str] = &["center", "left"];
const TEXT_WIDTHS: &[&str] = &["narrow", "wide"];
const IMAGE_WIDTHS: &[&str] = &["wide", "narrow", "full"];
const SIDES: &[&str] = &["left", "right"];
const SIZES: &[&str] = &["medium", "small", "large"];
const BUTTON_STYLES: &[&str] = &["primary", "secondary"];
const GALLERY_LAYOUTS: &[&str] = &["grid", "carousel"];
const GALLERY_COLUMNS: &[&str] = &["3", "2", "4"];
const ICONS: &[&str] = &[
    "none", "check", "star", "heart", "clock", "phone", "map-pin", "truck", "leaf", "shield",
    "sparkles",
];

const HERO: &[(&str, Kind)] = &[
    ("title", Kind::Text(160)),
    ("subtitle", Kind::Text(400)),
    ("image", Kind::Image),
    ("button_label", Kind::Text(60)),
    ("button_link", Kind::Link),
    ("align", Kind::Choice(ALIGNMENTS)),
];
const TEXT: &[(&str, Kind)] = &[
    ("text", Kind::Text(50_000)),
    ("width", Kind::Choice(TEXT_WIDTHS)),
];
const IMAGE: &[(&str, Kind)] = &[
    ("image", Kind::Image),
    ("alt", Kind::Text(300)),
    ("caption", Kind::Text(300)),
    ("width", Kind::Choice(IMAGE_WIDTHS)),
];
const TEXT_IMAGE: &[(&str, Kind)] = &[
    ("title", Kind::Text(160)),
    ("text", Kind::Text(10_000)),
    ("image", Kind::Image),
    ("alt", Kind::Text(300)),
    ("image_side", Kind::Choice(SIDES)),
];
const BUTTONS: &[(&str, Kind)] = &[
    ("buttons", Kind::Buttons),
    ("align", Kind::Choice(ALIGNMENTS)),
];
const SPACER: &[(&str, Kind)] = &[("size", Kind::Choice(SIZES)), ("divider", Kind::Flag)];
const GALLERY_ITEM: &[(&str, Kind)] = &[
    ("image", Kind::Image),
    ("alt", Kind::Text(300)),
    ("caption", Kind::Text(200)),
];
const GALLERY: &[(&str, Kind)] = &[
    ("title", Kind::Text(160)),
    ("items", Kind::Items(24, GALLERY_ITEM)),
    ("layout", Kind::Choice(GALLERY_LAYOUTS)),
    ("columns", Kind::Choice(GALLERY_COLUMNS)),
];
const COLUMN_ITEM: &[(&str, Kind)] = &[
    ("icon", Kind::Choice(ICONS)),
    ("image", Kind::Image),
    ("alt", Kind::Text(300)),
    ("title", Kind::Text(120)),
    ("text", Kind::Text(2_000)),
    ("link_label", Kind::Text(60)),
    ("link", Kind::Link),
];
const COLUMNS: &[(&str, Kind)] = &[
    ("title", Kind::Text(160)),
    ("items", Kind::Items(4, COLUMN_ITEM)),
    ("align", Kind::Choice(ALIGNMENTS)),
];
const FAQ_ITEM: &[(&str, Kind)] = &[("question", Kind::Text(300)), ("answer", Kind::Text(4_000))];
const FAQ: &[(&str, Kind)] = &[
    ("title", Kind::Text(160)),
    ("items", Kind::Items(30, FAQ_ITEM)),
];
const TESTIMONIAL_ITEM: &[(&str, Kind)] = &[
    ("quote", Kind::Text(1_000)),
    ("name", Kind::Text(120)),
    ("role", Kind::Text(120)),
    ("image", Kind::Image),
];
const TESTIMONIALS: &[(&str, Kind)] = &[
    ("title", Kind::Text(160)),
    ("items", Kind::Items(12, TESTIMONIAL_ITEM)),
];
const CONTACT: &[(&str, Kind)] = &[
    ("title", Kind::Text(160)),
    ("text", Kind::Text(1_000)),
    ("phone", Kind::Phone),
    ("whatsapp", Kind::Phone),
    ("email", Kind::Email),
    ("address", Kind::Text(400)),
    ("hours", Kind::Text(1_000)),
    ("show_map", Kind::Flag),
];
const LIST_SORTS: &[&str] = &["newest", "oldest"];
const LIST_LIMITS: &[&str] = &["3", "6", "9", "12"];
const MAX_KEY_CHARS: usize = 64;
const FORM: &[(&str, Kind)] = &[
    ("title", Kind::Text(160)),
    ("text", Kind::Text(1_000)),
    ("form", Kind::Key),
];
const BOOKING: &[(&str, Kind)] = &[
    ("title", Kind::Text(160)),
    ("text", Kind::Text(1_000)),
    ("booking", Kind::Key),
];
const COLLECTION: &[(&str, Kind)] = &[
    ("title", Kind::Text(160)),
    ("owner", Kind::Identifier),
    ("entity", Kind::Identifier),
    ("sort", Kind::Choice(LIST_SORTS)),
    ("limit", Kind::Choice(LIST_LIMITS)),
    ("more_label", Kind::Text(60)),
    ("more_link", Kind::Link),
];
const EMBED_HEIGHTS: &[&str] = &["medium", "small", "large"];
const MAX_EMBED_URL_CHARS: usize = 500;
const MAX_EMBED_ID_CHARS: usize = 120;
const EMBED: &[(&str, Kind)] = &[
    ("url", Kind::Embed),
    ("title", Kind::Text(160)),
    ("height", Kind::Choice(EMBED_HEIGHTS)),
];
const VIDEO: &[(&str, Kind)] = &[
    ("url", Kind::Video),
    ("title", Kind::Text(160)),
    ("caption", Kind::Text(300)),
];

fn fields_of(block_type: &str) -> Option<&'static [(&'static str, Kind)]> {
    match block_type {
        "hero" => Some(HERO),
        "text" => Some(TEXT),
        "image" => Some(IMAGE),
        "text_image" => Some(TEXT_IMAGE),
        "buttons" => Some(BUTTONS),
        "spacer" => Some(SPACER),
        "gallery" => Some(GALLERY),
        "columns" => Some(COLUMNS),
        "faq" => Some(FAQ),
        "testimonials" => Some(TESTIMONIALS),
        "contact" => Some(CONTACT),
        "video" => Some(VIDEO),
        "embed" => Some(EMBED),
        "form" => Some(FORM),
        "booking" => Some(BOOKING),
        "collection" => Some(COLLECTION),
        _ => None,
    }
}

fn invalid(code: &str, message: String) -> BlockError {
    BlockError::new(code, FIELD, message)
}

pub fn is_safe_link(link: &str) -> bool {
    let lower = link.to_ascii_lowercase();
    let local = link.starts_with('/') && !link.starts_with("//");
    let anchor = link.strip_prefix('#').is_some_and(|name| {
        !name.is_empty()
            && name
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || character == '-')
    });
    if anchor {
        return link.chars().count() <= MAX_ID_CHARS + 1;
    }
    let external = ["https://", "http://", "mailto:", "tel:"]
        .iter()
        .any(|scheme| lower.starts_with(scheme) && lower.len() > scheme.len());
    link.chars().count() <= MAX_LINK_CHARS
        && (local || external)
        && !link.chars().any(|character| {
            character.is_whitespace() || character.is_control() || character == '\\'
        })
}

fn text(value: Option<&Value>, max: usize, label: &str) -> Result<Value, BlockError> {
    let text = match value {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(text)) => text.replace('\r', ""),
        Some(_) => {
            return Err(invalid("block_invalid", format!("{label} must be text")));
        }
    };
    if text.chars().count() > max {
        return Err(invalid(
            "block_text_too_long",
            format!("{label} has more than {max} characters"),
        ));
    }
    Ok(Value::String(text))
}

fn link(value: Option<&Value>, label: &str) -> Result<Value, BlockError> {
    let link = match value {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(link)) => link.trim().to_owned(),
        Some(_) => {
            return Err(invalid("block_invalid", format!("{label} must be text")));
        }
    };
    if !link.is_empty() && !is_safe_link(&link) {
        return Err(invalid(
            "block_link_invalid",
            format!(
                "{label}: use a site address such as /contact or one that starts with https://"
            ),
        ));
    }
    Ok(Value::String(link))
}

fn image(value: Option<&Value>, label: &str) -> Result<Value, BlockError> {
    match value {
        None | Some(Value::Null) => Ok(Value::Null),
        Some(Value::String(id)) if id.trim().is_empty() => Ok(Value::Null),
        Some(Value::String(id))
            if id.len() <= MAX_IMAGE_ID_CHARS
                && id
                    .chars()
                    .all(|character| character.is_ascii_hexdigit() || character == '-') =>
        {
            Ok(Value::String(id.to_ascii_lowercase()))
        }
        Some(_) => Err(invalid("block_invalid", format!("{label} is not an image"))),
    }
}

fn choice(value: Option<&Value>, options: &[&str]) -> Value {
    let chosen = value
        .and_then(Value::as_str)
        .filter(|candidate| options.contains(candidate))
        .unwrap_or(options[0]);
    Value::String(chosen.to_owned())
}

fn buttons(value: Option<&Value>) -> Result<Value, BlockError> {
    let list = match value {
        None | Some(Value::Null) => Vec::new(),
        Some(Value::Array(list)) => list.clone(),
        Some(_) => {
            return Err(invalid(
                "block_invalid",
                "buttons must be a list".to_owned(),
            ));
        }
    };
    let mut cleaned = Vec::new();
    for item in list {
        let Value::Object(button) = item else {
            return Err(invalid(
                "block_invalid",
                "a button must be an object".to_owned(),
            ));
        };
        let label = text(button.get("label"), 60, "button text")?;
        if label.as_str().is_some_and(|label| label.trim().is_empty()) {
            continue;
        }
        let mut clean = Map::new();
        clean.insert("label".to_owned(), label);
        clean.insert("link".to_owned(), link(button.get("link"), "button link")?);
        clean.insert(
            "style".to_owned(),
            choice(button.get("style"), BUTTON_STYLES),
        );
        cleaned.push(Value::Object(clean));
    }
    if cleaned.len() > MAX_BUTTONS {
        return Err(invalid(
            "blocks_too_many_buttons",
            format!("a buttons block has at most {MAX_BUTTONS} buttons"),
        ));
    }
    Ok(Value::Array(cleaned))
}

pub(crate) fn email(value: Option<&Value>) -> Result<Value, BlockError> {
    let email = value
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_owned();
    let valid = email.is_empty()
        || (email.chars().count() <= MAX_EMAIL_CHARS
            && !email.chars().any(char::is_whitespace)
            && email
                .split_once('@')
                .is_some_and(|(user, domain)| !user.is_empty() && domain.contains('.')));
    match valid {
        true => Ok(Value::String(email)),
        false => Err(invalid(
            "block_email_invalid",
            "the e-mail address is not valid".to_owned(),
        )),
    }
}

pub(crate) fn phone(value: Option<&Value>) -> Result<Value, BlockError> {
    let phone = value
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_owned();
    let valid = phone.chars().count() <= MAX_PHONE_CHARS
        && phone.chars().all(|character| {
            character.is_ascii_digit() || matches!(character, ' ' | '+' | '(' | ')' | '-' | '.')
        });
    match valid {
        true => Ok(Value::String(phone)),
        false => Err(invalid(
            "block_phone_invalid",
            "a phone number has only digits, spaces and + ( ) -".to_owned(),
        )),
    }
}

fn phone_display(phone: &str) -> String {
    let digits: String = phone.chars().filter(char::is_ascii_digit).collect();
    let national = match (phone.starts_with('+'), digits.strip_prefix("55")) {
        (true, Some(national)) => national,
        (true, None) => return phone.to_owned(),
        (false, _) => digits.as_str(),
    };
    match national.len() {
        11 => format!(
            "({}) {}-{}",
            &national[..2],
            &national[2..7],
            &national[7..]
        ),
        10 => format!(
            "({}) {}-{}",
            &national[..2],
            &national[2..6],
            &national[6..]
        ),
        _ => phone.to_owned(),
    }
}

fn youtube_id(candidate: &str) -> Option<String> {
    let id: String = candidate
        .chars()
        .take_while(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
        .collect();
    (id.len() == 11).then_some(id)
}

fn vimeo_id(candidate: &str) -> Option<String> {
    let id: String = candidate.chars().take_while(char::is_ascii_digit).collect();
    (!id.is_empty() && id.len() <= 12).then_some(id)
}

pub fn video_of(url: &str) -> Option<(&'static str, String)> {
    let url = url.trim();
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))?;
    let rest = rest.strip_prefix("www.").unwrap_or(rest);
    let rest = rest.strip_prefix("m.").unwrap_or(rest);
    if let Some(path) = rest.strip_prefix("youtu.be/") {
        return youtube_id(path).map(|id| ("youtube", id));
    }
    if let Some(path) = rest
        .strip_prefix("youtube.com/")
        .or_else(|| rest.strip_prefix("youtube-nocookie.com/"))
    {
        if let Some(query) = path.strip_prefix("watch?") {
            return query
                .split('&')
                .find_map(|pair| pair.strip_prefix("v="))
                .and_then(youtube_id)
                .map(|id| ("youtube", id));
        }
        return ["shorts/", "embed/", "live/"]
            .iter()
            .find_map(|prefix| path.strip_prefix(prefix))
            .and_then(youtube_id)
            .map(|id| ("youtube", id));
    }
    if let Some(path) = rest.strip_prefix("player.vimeo.com/video/") {
        return vimeo_id(path).map(|id| ("vimeo", id));
    }
    rest.strip_prefix("vimeo.com/")
        .and_then(vimeo_id)
        .map(|id| ("vimeo", id))
}

fn embed_token(candidate: &str) -> Option<String> {
    let token: String = candidate
        .chars()
        .take_while(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
        .collect();
    (!token.is_empty() && token.len() <= MAX_EMBED_ID_CHARS).then_some(token)
}

pub fn embed_of(url: &str) -> Option<(&'static str, String)> {
    let url = url.trim();
    let rest = url.strip_prefix("https://")?;
    let rest = rest.strip_prefix("www.").unwrap_or(rest);
    let (host, path) = rest.split_once('/')?;
    let path = path.split(['?', '#']).next().unwrap_or_default();
    let segments: Vec<&str> = path
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect();
    match (host, segments.as_slice()) {
        ("docs.google.com", ["forms", "d", "e", id, ..]) => {
            embed_token(id).map(|id| ("google_forms", id))
        }
        (host, ["to", id, ..])
            if host == "form.typeform.com" || host.ends_with(".typeform.com") =>
        {
            embed_token(id).map(|id| ("typeform", id))
        }
        ("tally.so", ["r" | "embed", id, ..]) => embed_token(id).map(|id| ("tally", id)),
        ("form.jotform.com" | "jotform.com", [id, ..])
            if id.chars().all(|character| character.is_ascii_digit()) =>
        {
            embed_token(id).map(|id| ("jotform", id))
        }
        ("calendly.com", [user, rest @ ..]) => {
            let user = embed_token(user)?;
            match rest.first() {
                Some(event) => Some(("calendly", format!("{user}/{}", embed_token(event)?))),
                None => Some(("calendly", user)),
            }
        }
        _ => None,
    }
}

fn embed(value: Option<&Value>, clean: &mut Map<String, Value>) -> Result<Value, BlockError> {
    let url = value
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_owned();
    let (provider, id) = match url.is_empty() {
        true => (Value::Null, Value::Null),
        false => {
            let (provider, id) = (url.chars().count() <= MAX_EMBED_URL_CHARS)
                .then(|| embed_of(&url))
                .flatten()
                .ok_or_else(|| {
                    invalid(
                        "block_embed_invalid",
                        "paste the link of a Google Forms, Typeform, Tally, Jotform or Calendly page".to_owned(),
                    )
                })?;
            (Value::String(provider.to_owned()), Value::String(id))
        }
    };
    clean.insert("provider".to_owned(), provider);
    clean.insert("embed_id".to_owned(), id);
    Ok(Value::String(url))
}

fn video(value: Option<&Value>, clean: &mut Map<String, Value>) -> Result<Value, BlockError> {
    let url = value
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_owned();
    let (provider, id) = match url.is_empty() {
        true => (Value::Null, Value::Null),
        false => {
            let (provider, id) = (url.chars().count() <= MAX_VIDEO_URL_CHARS)
                .then(|| video_of(&url))
                .flatten()
                .ok_or_else(|| {
                    invalid(
                        "block_video_invalid",
                        "paste the link of a YouTube or Vimeo video".to_owned(),
                    )
                })?;
            (Value::String(provider.to_owned()), Value::String(id))
        }
    };
    clean.insert("provider".to_owned(), provider);
    clean.insert("video_id".to_owned(), id);
    Ok(Value::String(url))
}

fn key_of(
    value: Option<&Value>,
    label: &str,
    allowed: impl Fn(char) -> bool,
) -> Result<Value, BlockError> {
    let key = value
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_owned();
    match key.len() <= MAX_KEY_CHARS && key.chars().all(allowed) {
        true => Ok(Value::String(key)),
        false => Err(invalid(
            "block_invalid",
            format!("{label} is not a valid choice"),
        )),
    }
}

fn items(value: Option<&Value>, max: usize, fields: &[(&str, Kind)]) -> Result<Value, BlockError> {
    let list = match value {
        None | Some(Value::Null) => Vec::new(),
        Some(Value::Array(list)) => list.clone(),
        Some(_) => {
            return Err(invalid("block_invalid", "items must be a list".to_owned()));
        }
    };
    if list.len() > max {
        return Err(invalid(
            "blocks_too_many_items",
            format!("this block has at most {max} items"),
        ));
    }
    let mut cleaned = Vec::with_capacity(list.len());
    for item in list {
        let Value::Object(item) = item else {
            return Err(invalid(
                "block_invalid",
                "an item must be an object".to_owned(),
            ));
        };
        cleaned.push(Value::Object(clean_fields(&item, fields)?));
    }
    Ok(Value::Array(cleaned))
}

fn clean_fields(
    source: &Map<String, Value>,
    fields: &[(&str, Kind)],
) -> Result<Map<String, Value>, BlockError> {
    let mut clean = Map::new();
    for (key, kind) in fields {
        let value = source.get(*key);
        let cleaned = match kind {
            Kind::Text(max) => text(value, *max, key)?,
            Kind::Image => image(value, key)?,
            Kind::Link => link(value, key)?,
            Kind::Choice(options) => choice(value, options),
            Kind::Flag => Value::Bool(value.and_then(Value::as_bool).unwrap_or(false)),
            Kind::Buttons => buttons(value)?,
            Kind::Items(max, item_fields) => items(value, *max, item_fields)?,
            Kind::Email => email(value)?,
            Kind::Phone => {
                let number = phone(value)?;
                let shown = number.as_str().map(phone_display).unwrap_or_default();
                clean.insert(format!("{key}_display"), Value::String(shown));
                number
            }
            Kind::Video => video(value, &mut clean)?,
            Kind::Embed => embed(value, &mut clean)?,
            Kind::Key => key_of(value, key, |character| {
                character.is_ascii_alphanumeric() || matches!(character, '-' | '_')
            })?,
            Kind::Identifier => key_of(value, key, |character| {
                character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
            })?,
        };
        clean.insert((*key).to_owned(), cleaned);
    }
    Ok(clean)
}

pub fn image_ids(blocks: &Value) -> Vec<String> {
    let mut found = Vec::new();
    collect_images(blocks, &mut found);
    found.sort();
    found.dedup();
    found
}

fn collect_images(value: &Value, found: &mut Vec<String>) {
    match value {
        Value::Array(list) => list.iter().for_each(|item| collect_images(item, found)),
        Value::Object(fields) => {
            for (key, field) in fields {
                match (key.as_str(), field) {
                    ("image", Value::String(id)) => found.push(id.clone()),
                    _ => collect_images(field, found),
                }
            }
        }
        _ => {}
    }
}

pub fn replace_images(value: &mut Value, replacements: &BTreeMap<String, String>) {
    match value {
        Value::Array(list) => list
            .iter_mut()
            .for_each(|item| replace_images(item, replacements)),
        Value::Object(fields) => {
            for (key, field) in fields.iter_mut() {
                if key == "image"
                    && let Value::String(id) = field
                {
                    *field = replacements
                        .get(id.as_str())
                        .map(|new| Value::String(new.clone()))
                        .unwrap_or(Value::Null);
                } else {
                    replace_images(field, replacements);
                }
            }
        }
        _ => {}
    }
}

fn block_id(value: Option<&Value>, index: usize) -> String {
    value
        .and_then(Value::as_str)
        .filter(|id| {
            !id.is_empty()
                && id.len() <= MAX_ID_CHARS
                && id
                    .chars()
                    .all(|character| character.is_ascii_alphanumeric() || character == '-')
        })
        .map(str::to_owned)
        .unwrap_or_else(|| format!("block-{index}"))
}

pub fn block(value: &Value, index: usize) -> Result<Option<Value>, BlockError> {
    let Value::Object(block) = value else {
        return Err(invalid(
            "block_invalid",
            "each block must be an object".to_owned(),
        ));
    };
    let Some(block_type) = block.get("type").and_then(Value::as_str) else {
        return Err(invalid(
            "block_invalid",
            "each block needs a type".to_owned(),
        ));
    };
    let Some(fields) = fields_of(block_type) else {
        return Ok(None);
    };
    let mut clean = clean_fields(block, fields)?;
    clean.insert(
        "id".to_owned(),
        Value::String(block_id(block.get("id"), index)),
    );
    clean.insert("type".to_owned(), Value::String(block_type.to_owned()));
    Ok(Some(Value::Object(clean)))
}

pub fn normalize(source: &str) -> Result<String, BlockError> {
    if source.trim().is_empty() {
        return Ok("[]".to_owned());
    }
    if source.chars().count() > MAX_JSON_CHARS {
        return Err(invalid(
            "blocks_too_large",
            "the content is too large; split it into smaller parts".to_owned(),
        ));
    }
    let parsed: Value = serde_json::from_str(source)
        .map_err(|_| invalid("block_invalid", "the blocks are not valid JSON".to_owned()))?;
    let Value::Array(list) = parsed else {
        return Err(invalid(
            "block_invalid",
            "the blocks must be a list".to_owned(),
        ));
    };
    if list.len() > MAX_BLOCKS {
        return Err(invalid(
            "blocks_too_many_blocks",
            format!("at most {MAX_BLOCKS} blocks are allowed"),
        ));
    }
    let mut cleaned = Vec::with_capacity(list.len());
    for (index, item) in list.iter().enumerate() {
        cleaned.extend(block(item, index)?);
    }
    let encoded = Value::Array(cleaned).to_string();
    if encoded.chars().count() > MAX_JSON_CHARS {
        return Err(invalid(
            "blocks_too_large",
            "the content is too large; split it into smaller parts".to_owned(),
        ));
    }
    Ok(encoded)
}
