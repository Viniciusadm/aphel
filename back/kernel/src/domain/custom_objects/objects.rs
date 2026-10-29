use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::UserId;
use crate::domain::access::Permission;

pub const CUSTOM_OBJECT_PREFIX: &str = "co_";
pub const MAX_OBJECT_KEY_LENGTH: usize = 40;
pub const MAX_OBJECT_LABEL_LENGTH: usize = 80;
pub const MAX_OBJECT_DESCRIPTION_LENGTH: usize = 500;
pub const MAX_OBJECT_ICON_LENGTH: usize = 48;
pub const DEFAULT_TITLE_FIELD_KEY: &str = "name";

pub const OBJECT_ACTIONS: [&str; 6] = ["read", "create", "update", "delete", "export", "archive"];

pub fn is_object_key(value: &str) -> bool {
    let mut characters = value.chars();
    value.len() <= MAX_OBJECT_KEY_LENGTH
        && characters
            .next()
            .is_some_and(|first| first.is_ascii_lowercase())
        && characters.all(|character| character.is_ascii_lowercase() || character == '_')
        && !value.ends_with('_')
        && !value.contains("__")
}

pub fn object_record_type(key: &str) -> String {
    format!("{CUSTOM_OBJECT_PREFIX}{key}")
}

pub fn object_key_of(record_type: &str) -> Option<&str> {
    record_type
        .strip_prefix(CUSTOM_OBJECT_PREFIX)
        .filter(|key| is_object_key(key))
}

pub fn is_custom_record_type(record_type: &str) -> bool {
    object_key_of(record_type).is_some()
}

pub fn object_permission(record_type: &str, action: &str) -> Option<Permission> {
    object_key_of(record_type)?;
    Permission::parse(format!("{record_type}.{action}")).ok()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomObjectData {
    pub label_singular: String,
    pub label_plural: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub title_field_key: String,
    pub owner_enabled: bool,
    pub is_active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomObject {
    pub id: Uuid,
    pub key: String,
    pub data: CustomObjectData,
    pub position: i32,
    pub module_key: Option<String>,
    pub created_by: Option<UserId>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl CustomObject {
    pub fn record_type(&self) -> String {
        object_record_type(&self.key)
    }

    pub fn permission(&self, action: &str) -> Option<Permission> {
        object_permission(&self.record_type(), action)
    }
}
