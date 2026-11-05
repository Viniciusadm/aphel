use chrono::{DateTime, Utc};
use uuid::Uuid;

use aphel_sdk::platform::application::shared::RecordType;

pub(crate) const FAVORITE_NOT_FOUND: &str = "favorite_not_found";
pub(crate) const FAVORITE_TARGET_NOT_FOUND: &str = "favorite_target_not_found";
pub(crate) const FAVORITE_ALREADY_EXISTS: &str = "favorite_already_exists";
pub(crate) const FAVORITE_LIMIT_REACHED: &str = "favorite_limit_reached";

pub(crate) const MAX_FAVORITES: usize = 50;

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
