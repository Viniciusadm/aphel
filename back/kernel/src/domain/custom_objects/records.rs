use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::UserId;

pub const MAX_RECORD_TITLE_LENGTH: usize = 255;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomRecord {
    pub id: Uuid,
    pub object_id: Uuid,
    pub owner_id: Option<UserId>,
    pub title: String,
    pub created_by: Option<UserId>,
    pub archived_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub fn record_title(text: &str) -> String {
    let trimmed = text.trim();
    match trimmed.char_indices().nth(MAX_RECORD_TITLE_LENGTH) {
        Some((index, _)) => trimmed[..index].to_owned(),
        None => trimmed.to_owned(),
    }
}
