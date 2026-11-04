use chrono::{DateTime, Utc};
use uuid::Uuid;

use aphel_kernel::domain::UserId;

pub(crate) const MAX_TAG_NAME_LENGTH: usize = 60;
pub(crate) const DEFAULT_TAG_COLOR: &str = "#64748b";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tag {
    pub id: Uuid,
    pub name: String,
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
