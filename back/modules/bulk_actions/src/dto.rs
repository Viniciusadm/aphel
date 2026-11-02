use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::{
    BulkAction, BulkExportCommand, BulkItem, BulkOperation, BulkQuery, BulkSelection, ItemOutcome,
    StartBulkCommand,
};
use aphel_kernel::api::dto::query::FilterNodeSchema;
use aphel_kernel::application::query::{ArchivedFilter, Filter};
use aphel_kernel::errors::{AppError, FieldViolation};
use exports::dto::ExportFormatDto;

fn timestamp(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Secs, true)
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum BulkActionRequest {
    AssignOwner {
        owner_id: String,
    },
    SetField {
        #[schema(example = "source")]
        field: String,
        #[serde(default)]
        #[schema(value_type = Option<Value>)]
        value: Value,
    },
    AddTag {
        tag_id: String,
    },
    RemoveTag {
        tag_id: String,
    },
    MoveStage {
        stage_id: String,
    },
    EnrollInSequence {
        sequence_id: String,
    },
    Archive,
    Unarchive,
    Delete,
}

impl From<BulkActionRequest> for BulkAction {
    fn from(request: BulkActionRequest) -> Self {
        match request {
            BulkActionRequest::AssignOwner { owner_id } => Self::AssignOwner { owner_id },
            BulkActionRequest::SetField { field, value } => Self::SetField {
                field,
                value,
                custom: false,
            },
            BulkActionRequest::AddTag { tag_id } => Self::AddTag { tag_id },
            BulkActionRequest::RemoveTag { tag_id } => Self::RemoveTag { tag_id },
            BulkActionRequest::MoveStage { stage_id } => Self::MoveStage { stage_id },
            BulkActionRequest::EnrollInSequence { sequence_id } => {
                Self::EnrollInSequence { sequence_id }
            }
            BulkActionRequest::Archive => Self::Archive,
            BulkActionRequest::Unarchive => Self::Unarchive,
            BulkActionRequest::Delete => Self::Delete,
        }
    }
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct BulkQueryRequest {
    #[schema(value_type = Option<FilterNodeSchema>)]
    pub(crate) filter: Option<Filter>,
    pub(crate) search: Option<String>,
    #[schema(example = "exclude")]
    pub(crate) archived: Option<String>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct BulkSelectionRequest {
    #[serde(default)]
    pub(crate) ids: Option<Vec<String>>,
    pub(crate) query: Option<BulkQueryRequest>,
}

impl BulkSelectionRequest {
    fn into_selection(self) -> Result<BulkSelection, AppError> {
        let invalid = |field: &str, message: &str| {
            AppError::invalid_fields(
                "invalid selection",
                vec![FieldViolation::new(field.to_owned(), message.to_owned())],
            )
        };
        match (self.ids, self.query) {
            (Some(ids), None) => {
                let ids = ids
                    .iter()
                    .map(|id| Uuid::parse_str(id.trim()))
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|_| invalid("selection.ids", "must be a list of record ids"))?;
                Ok(BulkSelection::Ids { ids })
            }
            (None, Some(query)) => {
                let archived = match query.archived.as_deref().map(str::trim) {
                    None | Some("") => ArchivedFilter::Exclude,
                    Some(value) => ArchivedFilter::parse(value).ok_or_else(|| {
                        invalid(
                            "selection.query.archived",
                            "must be exclude, include or only",
                        )
                    })?,
                };
                Ok(BulkSelection::Query {
                    query: BulkQuery {
                        filter: query.filter,
                        search: query
                            .search
                            .map(|search| search.trim().to_owned())
                            .filter(|search| !search.is_empty()),
                        archived,
                    },
                })
            }
            _ => Err(invalid("selection", "must have either ids or query")),
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct StartBulkRequest {
    #[schema(example = "contacts.contact")]
    pub(crate) record_type: String,
    pub(crate) action: BulkActionRequest,
    pub(crate) selection: BulkSelectionRequest,
}

impl StartBulkRequest {
    pub(crate) fn into_command(self, ip: Option<String>) -> Result<StartBulkCommand, AppError> {
        Ok(StartBulkCommand {
            record_type: self.record_type,
            action: self.action.into(),
            selection: self.selection.into_selection()?,
            ip,
        })
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct BulkExportRequest {
    #[schema(example = "contacts.contact")]
    pub(crate) record_type: String,
    pub(crate) format: ExportFormatDto,
    #[serde(default)]
    pub(crate) columns: Vec<String>,
    pub(crate) selection: BulkSelectionRequest,
}

impl BulkExportRequest {
    pub(crate) fn into_command(self, ip: Option<String>) -> Result<BulkExportCommand, AppError> {
        Ok(BulkExportCommand {
            record_type: self.record_type,
            selection: self.selection.into_selection()?,
            format: self.format.into(),
            columns: self.columns,
            ip,
        })
    }
}

#[derive(Debug, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct BulkItemsParams {
    #[param(example = "skipped")]
    pub(crate) outcome: Option<String>,
}

impl BulkItemsParams {
    pub(crate) fn outcome(&self) -> Result<Option<ItemOutcome>, AppError> {
        match self.outcome.as_deref().map(str::trim) {
            None | Some("") => Ok(None),
            Some(value) => ItemOutcome::parse(value).map(Some).ok_or_else(|| {
                AppError::invalid_fields(
                    "invalid outcome",
                    vec![FieldViolation::new("outcome", "must be skipped or failed")],
                )
            }),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct BulkOperationResponse {
    pub(crate) id: String,
    #[schema(example = "contacts.contact")]
    pub(crate) record_type: String,
    #[schema(example = "assign_owner")]
    pub(crate) action: String,
    #[schema(value_type = Object)]
    pub(crate) params: Value,
    #[schema(example = "completed")]
    pub(crate) status: String,
    pub(crate) background: bool,
    pub(crate) total: u64,
    pub(crate) processed: u64,
    pub(crate) succeeded: u64,
    pub(crate) skipped: u64,
    pub(crate) failed: u64,
    pub(crate) failure_reason: Option<String>,
    pub(crate) requested_by: String,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
    pub(crate) finished_at: Option<String>,
}

impl From<BulkOperation> for BulkOperationResponse {
    fn from(operation: BulkOperation) -> Self {
        Self {
            id: operation.id.to_string(),
            record_type: operation.record_type,
            action: operation.action.kind().to_owned(),
            params: serde_json::to_value(&operation.action).unwrap_or(Value::Null),
            status: operation.status.as_str().to_owned(),
            background: operation.total as usize > crate::BULK_INLINE_LIMIT,
            total: operation.total,
            processed: operation.processed,
            succeeded: operation.succeeded,
            skipped: operation.skipped,
            failed: operation.failed,
            failure_reason: operation.failure_reason,
            requested_by: operation.requested_by.into_inner(),
            created_at: timestamp(operation.created_at),
            updated_at: timestamp(operation.updated_at),
            finished_at: operation.finished_at.map(timestamp),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct BulkItemResponse {
    pub(crate) record_id: String,
    #[schema(example = "skipped")]
    pub(crate) outcome: String,
    #[schema(example = "permission_denied")]
    pub(crate) code: Option<String>,
    pub(crate) message: Option<String>,
}

impl From<BulkItem> for BulkItemResponse {
    fn from(item: BulkItem) -> Self {
        Self {
            record_id: item.record_id.to_string(),
            outcome: item.outcome.as_str().to_owned(),
            code: item.code,
            message: item.message,
        }
    }
}
