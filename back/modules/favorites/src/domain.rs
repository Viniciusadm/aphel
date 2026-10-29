use chrono::{DateTime, Utc};
use uuid::Uuid;

use aphel_kernel::application::fields::RecordTypeDefinition;
use aphel_kernel::application::shared::RecordType;
use aphel_kernel::domain::access::RecordScopeFilter;

pub(crate) const FAVORITE_NOT_FOUND: &str = "favorite_not_found";
pub(crate) const FAVORITE_TARGET_NOT_FOUND: &str = "favorite_target_not_found";
pub(crate) const FAVORITE_ALREADY_EXISTS: &str = "favorite_already_exists";
pub(crate) const FAVORITE_LIMIT_REACHED: &str = "favorite_limit_reached";

pub(crate) const MAX_FAVORITES: usize = 50;

pub(crate) const FAVORITE_TITLE_COLUMNS: [(&str, &str); 5] = [
    ("contact", "full_name"),
    ("company", "name"),
    ("deal", "title"),
    ("activity", "title"),
    ("product", "name"),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FavoriteKind {
    Record,
    View,
}

impl FavoriteKind {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Record => "record",
            Self::View => "view",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "record" => Some(Self::Record),
            "view" => Some(Self::View),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Favorite {
    pub(crate) id: Uuid,
    pub(crate) kind: FavoriteKind,
    pub(crate) record_type: RecordType,
    pub(crate) target_id: Uuid,
    pub(crate) position: i32,
    pub(crate) created_at: DateTime<Utc>,
}

impl Favorite {
    pub(crate) fn record_id(&self) -> Option<Uuid> {
        (self.kind == FavoriteKind::Record).then_some(self.target_id)
    }

    pub(crate) fn view_id(&self) -> Option<Uuid> {
        (self.kind == FavoriteKind::View).then_some(self.target_id)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ReadableRecordType {
    pub(crate) definition: RecordTypeDefinition,
    pub(crate) title_column: &'static str,
    pub(crate) visibility: RecordScopeFilter,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VisibleView {
    pub(crate) record_type: RecordType,
    pub(crate) name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FavoriteEntry {
    pub(crate) favorite: Favorite,
    pub(crate) title: Option<String>,
    pub(crate) available: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CreateFavoriteCommand {
    pub(crate) favorite_type: FavoriteKind,
    pub(crate) record_type: Option<String>,
    pub(crate) record_id: Option<String>,
    pub(crate) view_id: Option<String>,
}
