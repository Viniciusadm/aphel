use aphel_contract_crm::CrmRecordTypes;
use std::collections::BTreeSet;

use axum::Json;
use axum::extract::rejection::QueryRejection;
use axum::extract::{Query, State};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::domain::{TimelineEntry, TimelineKind, TimelinePage, TimelineQuery};
use crate::service;
use aphel_kernel::Ctx;
use aphel_kernel::api::dto::common::{ErrorEnvelope, timestamp as instant};
use aphel_kernel::api::error::ApiError;
use aphel_kernel::api::extractors::AuthContext;
use aphel_kernel::api::routes::v1::field_permissions::field_policies;
use aphel_kernel::api::state::AppState;
use aphel_kernel::application::shared::RecordType;
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::AppError;

pub(crate) const TAG: &str = "timeline";

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(record_timeline))
}

#[utoipa::path(
    get,
    path = "/timeline",
    tag = TAG,
    operation_id = "timeline_list",
    security(("session_cookie" = [])),
    params(TimelineParams),
    responses(
        (status = 200, description = "Activities (main link or additional links), visible notes and domain events of the record, newest first; for a company also the events of its people and deals. `kinds` is a comma separated list of activity, note, deal_stage, deal_change, record_change, attachment and event. Answers `{ data, meta: { next_cursor } }`: pass `meta.next_cursor` back as `cursor` for the next page (null on the last one)", body = TimelinePageResponse),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing read access to the record (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid record reference, kinds, cursor or page size (`validation_error`); unknown record type (`unsupported_record_type`)", body = ErrorEnvelope),
    )
)]
async fn record_timeline(
    State(state): State<AppState>,
    tenant: AuthContext,
    query: Result<Query<TimelineParams>, QueryRejection>,
) -> Result<Json<TimelinePageResponse>, ApiError> {
    let Query(params) = query.map_err(|rejection| AppError::Validation(rejection.body_text()))?;
    let mut page = service::page(
        &Ctx::from_state(&state, &tenant.scope).await?,
        params.into(),
    )
    .await?;
    mask_timeline(&state, &tenant.scope, &mut page.items).await?;
    Ok(Json(page.into()))
}

const ACTIVITY_DETAIL_KEYS: [&str; 4] = ["due_at", "completed_at", "duration_minutes", "priority"];
const ACTIVITY_LOCATION_KEYS: [(&str, &str); 2] = [
    ("location_accuracy_m", "accuracy_m"),
    ("location_captured_at", "captured_at"),
];

fn mask_location(metadata: &mut serde_json::Map<String, Value>, hidden: &BTreeSet<String>) {
    if hidden.contains("location_latitude") || hidden.contains("location_longitude") {
        metadata.remove("location");
        return;
    }
    let Some(location) = metadata.get_mut("location").and_then(Value::as_object_mut) else {
        return;
    };
    for (field, key) in ACTIVITY_LOCATION_KEYS {
        if hidden.contains(field) {
            location.remove(key);
        }
    }
}

pub(crate) async fn mask_timeline(
    state: &AppState,
    scope: &AccessScope,
    entries: &mut [TimelineEntry],
) -> Result<(), AppError> {
    let record_types = entries.iter().filter_map(|entry| match entry.kind {
        TimelineKind::Activity => Some(RecordType::ACTIVITY),
        _ => event_record_type(entry),
    });
    let policies = field_policies(state, scope, record_types.collect::<Vec<_>>()).await?;
    if policies.is_empty() {
        return Ok(());
    }
    for entry in entries {
        if entry.kind == TimelineKind::Activity {
            let Some(policy) = policies.get(&RecordType::ACTIVITY) else {
                continue;
            };
            let hidden = policy.hidden_keys();
            if hidden.contains("description") {
                entry.body = None;
            }
            if let Some(metadata) = entry.metadata.as_object_mut() {
                for key in ACTIVITY_DETAIL_KEYS {
                    if hidden.contains(key) {
                        metadata.remove(key);
                    }
                }
                mask_location(metadata, &hidden);
            }
            continue;
        }
        let Some(policy) =
            event_record_type(entry).and_then(|record_type| policies.get(&record_type))
        else {
            continue;
        };
        let event_type = entry
            .metadata
            .get("event_type")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        if let Some(payload) = entry.metadata.get_mut("payload") {
            policy.mask_event_payload(&event_type, payload);
        }
    }
    Ok(())
}

fn event_record_type(entry: &TimelineEntry) -> Option<RecordType> {
    entry
        .metadata
        .get("record_type")
        .and_then(Value::as_str)
        .and_then(|record_type| RecordType::parse(record_type).ok())
}

#[derive(Debug, Clone, Copy, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = TimelineKind)]
pub(crate) enum TimelineKindSchema {
    Activity,
    Note,
    DealStage,
    DealChange,
    RecordChange,
    Attachment,
    Event,
}

impl From<TimelineKind> for TimelineKindSchema {
    fn from(kind: TimelineKind) -> Self {
        match kind {
            TimelineKind::Activity => Self::Activity,
            TimelineKind::Note => Self::Note,
            TimelineKind::DealStage => Self::DealStage,
            TimelineKind::DealChange => Self::DealChange,
            TimelineKind::RecordChange => Self::RecordChange,
            TimelineKind::Attachment => Self::Attachment,
            TimelineKind::Event => Self::Event,
        }
    }
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct TimelineParams {
    #[param(example = "organizations.organization")]
    pub(crate) record_type: String,
    pub(crate) record_id: String,
    pub(crate) cursor: Option<String>,
    #[param(example = "activity,note,deal_stage")]
    pub(crate) kinds: Option<String>,
    #[param(minimum = 1, maximum = 100, example = 25)]
    pub(crate) per_page: Option<u32>,
}

impl From<TimelineParams> for TimelineQuery {
    fn from(params: TimelineParams) -> Self {
        Self {
            record_type: params.record_type,
            record_id: params.record_id,
            cursor: params.cursor,
            kinds: params.kinds,
            per_page: params.per_page,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct TimelineActorResponse {
    pub(crate) id: String,
    pub(crate) display_name: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct TimelineRecordResponse {
    pub(crate) record_type: String,
    pub(crate) record_id: String,
    pub(crate) title: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct TimelineItemResponse {
    #[schema(example = "activity:0190a8f2-7c1e-7b3a-9f00-000000000000")]
    pub(crate) id: String,
    pub(crate) kind: TimelineKindSchema,
    pub(crate) occurred_at: String,
    pub(crate) actor: Option<TimelineActorResponse>,
    pub(crate) title: String,
    pub(crate) body: Option<String>,
    pub(crate) record: Option<TimelineRecordResponse>,
    #[schema(value_type = Object)]
    pub(crate) metadata: Value,
}

impl From<TimelineEntry> for TimelineItemResponse {
    fn from(entry: TimelineEntry) -> Self {
        Self {
            id: entry.id,
            kind: entry.kind.into(),
            occurred_at: instant(entry.occurred_at),
            actor: entry.actor.map(|actor| TimelineActorResponse {
                id: actor.id.into_inner(),
                display_name: actor.display_name,
            }),
            title: entry.title,
            body: entry.body,
            record: entry.record.map(|linked| TimelineRecordResponse {
                record_type: linked.record.record_type.as_str().to_owned(),
                record_id: linked.record.record_id.to_string(),
                title: linked.title,
            }),
            metadata: entry.metadata,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct TimelineMeta {
    #[schema(example = "MjAyNi0xMC0yMFQxMzo0NTowMFp8YWN0aXZpdHk6MDE5MA")]
    pub(crate) next_cursor: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct TimelinePageResponse {
    pub(crate) data: Vec<TimelineItemResponse>,
    pub(crate) meta: TimelineMeta,
}

impl From<TimelinePage> for TimelinePageResponse {
    fn from(page: TimelinePage) -> Self {
        Self {
            data: page.items.into_iter().map(Into::into).collect(),
            meta: TimelineMeta {
                next_cursor: page.next_cursor,
            },
        }
    }
}
