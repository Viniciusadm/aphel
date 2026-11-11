use chrono::{DateTime, Utc};
use uuid::Uuid;

use aphel_kernel::domain::UserId;

pub(crate) const MAX_TAG_NAME_LENGTH: usize = 60;
pub(crate) const DEFAULT_TAG_COLOR: &str = "#64748b";
pub(crate) const GENERAL_VOCABULARY: &str = "general";
pub(crate) const GENERAL_VOCABULARY_ID: Uuid =
    Uuid::from_u128(0x0000_0000_0000_7000_8000_0000_0000_0001);
pub(crate) const MAX_TERM_DEPTH: usize = 8;
pub(crate) const MAX_SLUG_LENGTH: usize = 80;
const MAX_VOCABULARY_KEY_LENGTH: usize = 48;
const MAX_VOCABULARY_NAME_LENGTH: usize = 80;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vocabulary {
    pub id: Uuid,
    pub key: String,
    pub name: String,
    pub hierarchical: bool,
    pub public: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tag {
    pub id: Uuid,
    pub vocabulary_id: Uuid,
    pub name: String,
    pub slug: Option<String>,
    pub parent_id: Option<Uuid>,
    pub color: String,
    pub(crate) record_types: Vec<String>,
    pub(crate) created_by: Option<UserId>,
    pub created_at: DateTime<Utc>,
    pub(crate) updated_at: DateTime<Utc>,
}

impl Tag {
    pub(crate) fn accepts(&self, record_type: &str) -> bool {
        self.record_types.is_empty() || self.record_types.iter().any(|kind| kind == record_type)
    }
}

pub(crate) fn normalize_tag_name(value: &str) -> Result<String, &'static str> {
    let name = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if name.is_empty() {
        return Err("must not be empty");
    }
    if name.chars().count() > MAX_TAG_NAME_LENGTH {
        return Err("must have at most 60 characters");
    }
    Ok(name)
}

pub(crate) fn normalize_tag_color(value: &str) -> Result<String, &'static str> {
    let color = value.trim().to_ascii_lowercase();
    let valid = color.len() == 7
        && color.starts_with('#')
        && color[1..]
            .chars()
            .all(|character| character.is_ascii_hexdigit());
    if valid {
        Ok(color)
    } else {
        Err("must be a hex color like #1f6feb")
    }
}

pub(crate) fn slugify(value: &str) -> String {
    let mut slug = String::new();
    let mut dash = false;
    for character in value.trim().to_lowercase().chars() {
        let mapped = match character {
            'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            'ñ' => 'n',
            other => other,
        };
        if mapped.is_ascii_alphanumeric() {
            slug.push(mapped);
            dash = false;
        } else if !dash && !slug.is_empty() {
            slug.push('-');
            dash = true;
        }
    }
    let slug: String = slug
        .trim_end_matches('-')
        .chars()
        .take(MAX_SLUG_LENGTH)
        .collect();
    slug.trim_end_matches('-').to_owned()
}

pub(crate) fn normalize_slug(value: &str) -> Result<String, &'static str> {
    let slug = value.trim().to_ascii_lowercase();
    let valid = !slug.is_empty()
        && slug.len() <= MAX_SLUG_LENGTH
        && !slug.starts_with('-')
        && !slug.ends_with('-')
        && !slug.contains("--")
        && slug.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
        });
    if valid {
        Ok(slug)
    } else {
        Err("must use lowercase letters, digits and single hyphens")
    }
}

pub(crate) fn normalize_vocabulary_key(value: &str) -> Result<String, &'static str> {
    let key = value.trim();
    let valid = !key.is_empty()
        && key.len() <= MAX_VOCABULARY_KEY_LENGTH
        && key.starts_with(|character: char| character.is_ascii_lowercase())
        && key.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
        });
    if valid {
        Ok(key.to_owned())
    } else {
        Err("must start with a letter and use lowercase letters, digits and underscores")
    }
}

pub(crate) fn normalize_vocabulary_name(value: &str) -> Result<String, &'static str> {
    let name = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if name.is_empty() {
        return Err("must not be empty");
    }
    if name.chars().count() > MAX_VOCABULARY_NAME_LENGTH {
        return Err("must have at most 80 characters");
    }
    Ok(name)
}
