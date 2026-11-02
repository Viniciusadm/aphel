use async_trait::async_trait;
use sqlx::{MySql, MySqlPool, QueryBuilder};

use activities::domain::LinkedRecord;
use aphel_kernel::application::files::UserReference;
use aphel_kernel::application::shared::RecordRef;
use aphel_kernel::domain::access::{AccessScope, RecordScopeFilter};
use aphel_kernel::domain::custom_objects::is_custom_record_type;
use aphel_kernel::errors::AppError;
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::collections::BTreeSet;

pub(crate) const DEFAULT_TIMELINE_PAGE_SIZE: u32 = 25;
pub(crate) const MAX_TIMELINE_PAGE_SIZE: u32 = 100;

pub(crate) const DEAL_STAGE_EVENTS: &[&str] = &["deal.stage_changed"];

pub(crate) const DEAL_CHANGE_EVENTS: &[&str] = &[
    "deal.amount_changed",
    "deal.owner_changed",
    "deal.expected_close_date_changed",
    "deal.won",
    "deal.lost",
    "deal.reopened",
];

pub(crate) const RECORD_CHANGE_EVENTS: &[&str] = &[
    "contact.created",
    "contact.updated",
    "contact.deleted",
    "contact.archived",
    "contact.unarchived",
    "company.created",
    "company.updated",
    "company.deleted",
    "company.archived",
    "company.unarchived",
    "deal.created",
    "deal.updated",
    "deal.deleted",
    "deal.archived",
    "deal.unarchived",
];

pub(crate) const ATTACHMENT_EVENTS: &[&str] = &["attachment.added", "attachment.removed"];

pub(crate) const HIDDEN_EVENT_PREFIXES: &[&str] = &["note.", "activity."];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) enum TimelineKind {
    Activity,
    Note,
    DealStage,
    DealChange,
    RecordChange,
    Attachment,
    Event,
}

impl TimelineKind {
    pub(crate) const ALL: [Self; 7] = [
        Self::Activity,
        Self::Note,
        Self::DealStage,
        Self::DealChange,
        Self::RecordChange,
        Self::Attachment,
        Self::Event,
    ];

    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Activity => "activity",
            Self::Note => "note",
            Self::DealStage => "deal_stage",
            Self::DealChange => "deal_change",
            Self::RecordChange => "record_change",
            Self::Attachment => "attachment",
            Self::Event => "event",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.as_str() == value)
    }

    pub(crate) fn is_event(&self) -> bool {
        !matches!(self, Self::Activity | Self::Note)
    }

    pub(crate) fn event_types(&self) -> &'static [&'static str] {
        match self {
            Self::DealStage => DEAL_STAGE_EVENTS,
            Self::DealChange => DEAL_CHANGE_EVENTS,
            Self::RecordChange => RECORD_CHANGE_EVENTS,
            Self::Attachment => ATTACHMENT_EVENTS,
            Self::Activity | Self::Note | Self::Event => &[],
        }
    }
}

pub(crate) fn is_hidden_event(event_type: &str) -> bool {
    HIDDEN_EVENT_PREFIXES
        .iter()
        .any(|prefix| event_type.starts_with(prefix))
}

pub(crate) fn is_custom_record_change(event_type: &str) -> bool {
    event_type
        .rsplit_once('.')
        .is_some_and(|(prefix, action)| action == "updated" && is_custom_record_type(prefix))
}

pub(crate) fn timeline_kind_for_event(event_type: &str) -> Option<TimelineKind> {
    if is_hidden_event(event_type) {
        return None;
    }
    if is_custom_record_change(event_type) {
        return Some(TimelineKind::RecordChange);
    }
    let kind = [
        TimelineKind::DealStage,
        TimelineKind::DealChange,
        TimelineKind::RecordChange,
        TimelineKind::Attachment,
    ]
    .into_iter()
    .find(|kind| kind.event_types().contains(&event_type))
    .unwrap_or(TimelineKind::Event);
    Some(kind)
}

pub(crate) fn known_event_types() -> impl Iterator<Item = &'static str> {
    DEAL_STAGE_EVENTS
        .iter()
        .chain(DEAL_CHANGE_EVENTS)
        .chain(RECORD_CHANGE_EVENTS)
        .chain(ATTACHMENT_EVENTS)
        .copied()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TimelineCursor {
    pub(crate) occurred_at: DateTime<Utc>,
    pub(crate) item_id: String,
}

impl TimelineCursor {
    pub(crate) fn new(occurred_at: DateTime<Utc>, item_id: impl Into<String>) -> Self {
        Self {
            occurred_at,
            item_id: item_id.into(),
        }
    }

    pub(crate) fn encode(&self) -> String {
        format!("{}|{}", self.occurred_at.timestamp_micros(), self.item_id)
            .bytes()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }

    pub(crate) fn decode(value: &str) -> Option<Self> {
        if value.is_empty() || !value.len().is_multiple_of(2) || value.len() > 512 {
            return None;
        }
        let bytes = (0..value.len())
            .step_by(2)
            .map(|index| u8::from_str_radix(value.get(index..index + 2)?, 16).ok())
            .collect::<Option<Vec<u8>>>()?;
        let text = String::from_utf8(bytes).ok()?;
        let (micros, item_id) = text.split_once('|')?;
        let valid_id = !item_id.is_empty()
            && item_id.chars().all(|character| {
                character.is_ascii_alphanumeric() || matches!(character, ':' | '-' | '_')
            });
        if !valid_id {
            return None;
        }
        let occurred_at = DateTime::<Utc>::from_timestamp_micros(micros.parse().ok()?)?;
        Some(Self::new(occurred_at, item_id))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct TimelineQuery {
    pub(crate) record_type: String,
    pub(crate) record_id: String,
    pub(crate) cursor: Option<String>,
    pub(crate) kinds: Option<String>,
    pub(crate) per_page: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineRequest {
    pub(crate) subject: RecordRef,
    pub(crate) kinds: BTreeSet<TimelineKind>,
    pub(crate) cursor: Option<TimelineCursor>,
    pub(crate) limit: u32,
    pub(crate) activities: Option<RecordScopeFilter>,
    pub(crate) notes: bool,
    pub(crate) related_contacts: Option<RecordScopeFilter>,
    pub(crate) related_deals: Option<RecordScopeFilter>,
    pub emails: bool,
}

impl TimelineRequest {
    pub(crate) fn wants(&self, kind: TimelineKind) -> bool {
        self.kinds.contains(&kind)
    }

    pub(crate) fn wants_events(&self) -> bool {
        self.kinds.iter().any(TimelineKind::is_event)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TimelineEntry {
    pub(crate) id: String,
    pub(crate) kind: TimelineKind,
    pub(crate) occurred_at: DateTime<Utc>,
    pub(crate) actor: Option<UserReference>,
    pub(crate) title: String,
    pub(crate) body: Option<String>,
    pub(crate) record: Option<LinkedRecord>,
    pub metadata: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TimelinePage {
    pub(crate) items: Vec<TimelineEntry>,
    pub(crate) next_cursor: Option<String>,
}

#[async_trait]
pub trait TimelineEventExtension: Send + Sync {
    fn restrict(
        &self,
        builder: &mut QueryBuilder<MySql>,
        scope: &AccessScope,
        subject: &RecordRef,
        request: &TimelineRequest,
    );
    async fn enrich(
        &self,
        pool: &MySqlPool,
        scope: &AccessScope,
        entries: &mut [TimelineEntry],
    ) -> Result<(), AppError>;
}
