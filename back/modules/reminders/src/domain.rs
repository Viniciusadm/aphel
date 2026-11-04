use chrono::{DateTime, Utc};
use uuid::Uuid;

use aphel_kernel::application::organization_admin::ListWindow;
use aphel_kernel::application::shared::RecordRef;
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;
use aphel_sdk::timeline::LinkedRecord;

pub(crate) const MAX_REMINDER_NOTE_LENGTH: usize = 500;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ReminderStatus {
    Pending,
    Sent,
    Dismissed,
}

impl ReminderStatus {
    pub(crate) const ALL: [Self; 3] = [Self::Pending, Self::Sent, Self::Dismissed];

    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Sent => "sent",
            Self::Dismissed => "dismissed",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|status| status.as_str() == value)
    }
}

pub(crate) fn normalize_reminder_note(value: Option<&str>) -> Result<Option<String>, &'static str> {
    let Some(note) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    if note.chars().count() > MAX_REMINDER_NOTE_LENGTH {
        return Err("must have at most 500 characters");
    }
    Ok(Some(note.to_owned()))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReminderView {
    pub(crate) id: Uuid,
    pub(crate) record: Option<LinkedRecord>,
    pub(crate) remind_at: DateTime<Utc>,
    pub(crate) note: Option<String>,
    pub(crate) status: ReminderStatus,
    pub(crate) sent_at: Option<DateTime<Utc>>,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReminderData {
    pub(crate) record: Option<RecordRef>,
    pub(crate) remind_at: DateTime<Utc>,
    pub(crate) note: Option<String>,
    pub(crate) status: ReminderStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct ReminderFilter {
    pub(crate) status: Option<ReminderStatus>,
    pub(crate) record: Option<RecordRef>,
}

pub(crate) use aphel_sdk::tasks::DueTaskReminder;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DueReminder {
    pub(crate) id: Uuid,
    pub(crate) user_id: UserId,
    pub(crate) record: Option<RecordRef>,
    pub(crate) note: Option<String>,
    pub(crate) remind_at: DateTime<Utc>,
}

pub(crate) const REMINDER_NOT_FOUND: &str = "reminder_not_found";

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct CreateReminderCommand {
    pub(crate) remind_at: String,
    pub(crate) note: Option<String>,
    pub(crate) record_type: Option<String>,
    pub(crate) record_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct UpdateReminderCommand {
    pub(crate) remind_at: Option<String>,
    pub(crate) note: Option<Option<String>>,
    pub(crate) status: Option<ReminderStatus>,
    pub(crate) record_type: Option<Option<String>>,
    pub(crate) record_id: Option<Option<String>>,
    pub(crate) expected_updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ListRemindersQuery {
    pub(crate) status: Option<String>,
    pub(crate) record_type: Option<String>,
    pub(crate) record_id: Option<String>,
    pub(crate) window: ListWindow,
}

pub(crate) fn reminder_not_found() -> AppError {
    AppError::not_found_code(REMINDER_NOT_FOUND, "reminder was not found")
}
