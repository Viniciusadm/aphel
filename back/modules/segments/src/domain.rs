use chrono::{DateTime, Utc};
use serde_json::{Value, json};
use uuid::Uuid;

use aphel_kernel::application::organization_admin::ListWindow;
use aphel_kernel::application::query::{ArchivedFilter, Filter, SortSpec};
use aphel_kernel::application::shared::RecordType;
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::{AppError, FieldViolation};

pub(crate) const SEGMENT_NOT_FOUND: &str = "segment_not_found";
pub(crate) const SEGMENT_NAME_TAKEN: &str = "segment_name_taken";
pub(crate) const SEGMENT_CREATED: &str = "segment.created";
pub(crate) const SEGMENT_UPDATED: &str = "segment.updated";
pub(crate) const SEGMENT_DELETED: &str = "segment.deleted";
pub(crate) const SEGMENT_RECORD_TYPE: RecordType = RecordType::from_static("segment");

pub(crate) const MAX_SEGMENT_NAME_LENGTH: usize = 140;
pub(crate) const MAX_SEGMENT_DESCRIPTION_LENGTH: usize = 500;
pub(crate) const MAX_SEGMENTS_PER_TYPE: usize = 200;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Segment {
    pub(crate) id: Uuid,
    pub(crate) record_type: RecordType,
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) filter: Filter,
    pub(crate) created_by: Option<UserId>,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) updated_at: DateTime<Utc>,
}

impl Segment {
    pub(crate) fn snapshot(&self) -> Value {
        json!({
            "name": self.name,
            "description": self.description,
            "filter": self.filter,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct CreateSegmentCommand {
    pub(crate) record_type: String,
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) filter: Option<Filter>,
    pub(crate) ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct UpdateSegmentCommand {
    pub(crate) name: Option<String>,
    pub(crate) description: Option<Option<String>>,
    pub(crate) filter: Option<Filter>,
    pub(crate) ip: Option<String>,
    pub(crate) expected_updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SegmentView {
    pub(crate) segment: Segment,
    pub(crate) member_count: Option<u64>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SegmentMembersQuery {
    pub(crate) filter: Option<Filter>,
    pub(crate) sort: Vec<SortSpec>,
    pub(crate) search: Option<String>,
    pub(crate) archived: ArchivedFilter,
    pub(crate) window: ListWindow,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SegmentMember {
    pub(crate) id: Uuid,
    pub(crate) label: Option<String>,
}

pub(crate) fn segment_not_found() -> AppError {
    AppError::not_found_code(SEGMENT_NOT_FOUND, "segment was not found")
}

pub(crate) fn segment_name_taken(name: &str) -> AppError {
    AppError::conflict_code(
        SEGMENT_NAME_TAKEN,
        format!("a segment named {name} already exists"),
    )
}

pub(crate) fn normalize_name(value: &str, violations: &mut Vec<FieldViolation>) -> Option<String> {
    let name = value.trim();
    if name.is_empty() || name.chars().count() > MAX_SEGMENT_NAME_LENGTH {
        violations.push(FieldViolation::new(
            "name",
            format!("must have between 1 and {MAX_SEGMENT_NAME_LENGTH} characters"),
        ));
        return None;
    }
    Some(name.to_owned())
}

pub(crate) fn normalize_description(
    value: Option<&str>,
    violations: &mut Vec<FieldViolation>,
) -> Option<String> {
    let description = value.map(str::trim).filter(|text| !text.is_empty())?;
    if description.chars().count() > MAX_SEGMENT_DESCRIPTION_LENGTH {
        violations.push(FieldViolation::new(
            "description",
            format!("must have at most {MAX_SEGMENT_DESCRIPTION_LENGTH} characters"),
        ));
        return None;
    }
    Some(description.to_owned())
}
