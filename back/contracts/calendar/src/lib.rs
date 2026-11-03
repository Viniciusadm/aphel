use aphel_contract::{Contract, ContractDescriptor, KeyEntry, KeyKind, contract};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

pub const CONTRACT: Contract = contract!("calendar", "1.0.0");
pub const DESCRIPTOR: ContractDescriptor = ContractDescriptor {
    contract: CONTRACT,
    entries: &[
        KeyEntry::new(
            "calendar.source",
            KeyKind::ExtensionPoint,
            "Authorized calendar entries supplied by their owner",
        ),
        KeyEntry::new(
            "calendar.availability",
            KeyKind::ExtensionPoint,
            "Owner-provided busy intervals for a member",
        ),
    ],
};

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct CalendarCategory {
    #[serde(default)]
    pub source: String,
    pub id: String,
    pub name: String,
    pub icon: Option<String>,
    pub color: Option<String>,
}
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct CalendarOwner {
    pub id: String,
    pub display_name: String,
}
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct CalendarRecord {
    pub id: String,
    pub source: String,
    pub record_id: String,
    pub title: String,
    pub due_at: String,
    pub duration_minutes: Option<i64>,
    pub status: String,
    pub finished: bool,
    pub category: CalendarCategory,
    pub owner: Option<CalendarOwner>,
    pub href: String,
    pub updated_at: String,
    pub can_update: bool,
}
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct CalendarQuery {
    pub from: String,
    pub to: String,
    pub owner_id: Option<String>,
    pub team_id: Option<String>,
    pub category_id: Option<String>,
    pub status: Option<String>,
    pub source: Option<String>,
}
#[derive(Debug, Default)]
pub struct CalendarBatch {
    pub items: Vec<CalendarRecord>,
    pub total: u64,
    pub truncated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BusyInterval {
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
}

impl BusyInterval {
    pub fn new(starts_at: DateTime<Utc>, ends_at: DateTime<Utc>) -> Self {
        Self { starts_at, ends_at }
    }
}
