use chrono::{DateTime, Utc};
use uuid::Uuid;

use aphel_kernel::application::files::UserReference;
use aphel_kernel::application::shared::RecordRef;
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;
use aphel_sdk::messaging::mentions::mentions_to_text;

pub(crate) const NOTE_NOT_FOUND: &str = "note_not_found";

pub(crate) const NOTE_CREATED: &str = "note.created";
pub(crate) const NOTE_UPDATED: &str = "note.updated";
pub(crate) const NOTE_DELETED: &str = "note.deleted";
pub(crate) const NOTE_PINNED: &str = "note.pinned";
pub(crate) const NOTE_UNPINNED: &str = "note.unpinned";

pub(crate) fn note_not_found() -> AppError {
    AppError::not_found_code(NOTE_NOT_FOUND, "note was not found")
}

pub(crate) const MAX_NOTE_BODY_LENGTH: usize = 50_000;
pub(crate) const NOTE_PREVIEW_LENGTH: usize = 140;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum NoteVisibility {
    #[default]
    Team,
    Private,
}

impl NoteVisibility {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Team => "team",
            Self::Private => "private",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "team" => Some(Self::Team),
            "private" => Some(Self::Private),
            _ => None,
        }
    }

    pub(crate) fn is_private(&self) -> bool {
        matches!(self, Self::Private)
    }
}

pub(crate) fn normalize_note_body(value: &str) -> Result<String, &'static str> {
    let body = value.trim();
    if body.is_empty() {
        return Err("must not be empty");
    }
    if body.chars().count() > MAX_NOTE_BODY_LENGTH {
        return Err("must have at most 50000 characters");
    }
    Ok(body.to_owned())
}

pub fn note_preview(body: &str) -> String {
    let body = mentions_to_text(body);
    let line: String = body
        .lines()
        .map(strip_block_markers)
        .find(|line| !line.is_empty())
        .unwrap_or_default()
        .chars()
        .filter(|character| !matches!(character, '*' | '`' | '~'))
        .collect();
    let line = line.trim();
    let mut preview: String = line.chars().take(NOTE_PREVIEW_LENGTH).collect();
    if line.chars().count() > NOTE_PREVIEW_LENGTH {
        preview.push('…');
    }
    preview
}

fn strip_block_markers(line: &str) -> &str {
    let mut line = line.trim();
    loop {
        let stripped = line
            .strip_prefix(['#', '>'])
            .or_else(|| {
                ["- ", "* ", "+ "]
                    .iter()
                    .find_map(|marker| line.strip_prefix(marker))
            })
            .map(str::trim_start);
        match stripped {
            Some(rest) => line = rest,
            None => return line.trim(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CreateNoteCommand {
    pub record_type: String,
    pub record_id: String,
    pub body: String,
    pub visibility: Option<String>,
    pub pinned: Option<bool>,
    pub ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct UpdateNoteCommand {
    pub(crate) body: Option<String>,
    pub(crate) visibility: Option<NoteVisibility>,
    pub(crate) pinned: Option<bool>,
    pub(crate) expected_updated_at: Option<DateTime<Utc>>,
    pub(crate) ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListNotesQuery {
    pub record_type: String,
    pub record_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteView {
    pub id: Uuid,
    pub author: Option<UserReference>,
    pub(crate) record: RecordRef,
    pub body: String,
    pub visibility: NoteVisibility,
    pub pinned: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl NoteView {
    pub fn author_id(&self) -> Option<&UserId> {
        self.author.as_ref().map(|author| &author.id)
    }

    pub(crate) fn data(&self) -> NoteData {
        NoteData {
            body: self.body.clone(),
            visibility: self.visibility,
            pinned: self.pinned,
        }
    }

    pub(crate) fn is_visible_to(&self, user_id: &UserId) -> bool {
        !self.visibility.is_private() || self.author_id() == Some(user_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NoteData {
    pub(crate) body: String,
    pub(crate) visibility: NoteVisibility,
    pub(crate) pinned: bool,
}
