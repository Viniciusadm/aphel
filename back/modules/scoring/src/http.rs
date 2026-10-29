use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::domain::{
    BehaviorSignal, CreateScoringModelCommand, CriterionInput, CriterionKind, RecordScoreView,
    ScoreBand, ScoreContribution, ScoringCriterion, ScoringModel, UpdateScoringModelCommand,
};
use crate::service;
use aphel_kernel::Ctx;
use aphel_kernel::api::dto::common::{Data, ErrorEnvelope};
use aphel_kernel::api::dto::common::{double_option, timestamp};
use aphel_kernel::api::error::ApiError;
use aphel_kernel::api::extractors::{AuthContext, ClientInfo, IfMatch, JsonBody};
use aphel_kernel::api::state::AppState;

pub(crate) const TAG: &str = "scoring";

const UNAUTHORIZED: &str = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)";
const FORBIDDEN_MANAGE: &str =
    "Missing `scoring.manage` (`permission_denied`, `insufficient_scope`)";
const NOT_FOUND: &str = "Scoring model not found (`scoring_model_not_found`)";
const INVALID: &str = "Invalid name, key, bands or criteria (`validation_error` with `details.fields`; filter problems under `criteria.<n>.filter`)";

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_scoring_models, create_scoring_model))
        .routes(routes!(
            get_scoring_model,
            update_scoring_model,
            delete_scoring_model
        ))
        .routes(routes!(recalculate_scoring_model))
        .routes(routes!(get_record_scores))
}

#[utoipa::path(
    get,
    path = "/scoring/models",
    tag = TAG,
    operation_id = "scoring_models_list",
    security(("session_cookie" = [])),
    description = "Lead scoring models of the organization with their criteria, primary model first within each record type. Needs `scoring.manage`.",
    params(ScoringModelListParams),
    responses(
        (status = 200, description = "Scoring models", body = Data<Vec<ScoringModelResponse>>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = FORBIDDEN_MANAGE, body = ErrorEnvelope),
    )
)]
async fn list_scoring_models(
    State(state): State<AppState>,
    tenant: AuthContext,
    Query(params): Query<ScoringModelListParams>,
) -> Result<Json<Data<Vec<ScoringModelResponse>>>, ApiError> {
    let models = service::list(
        &Ctx::from_state(&state, &tenant.scope).await?,
        params.record_type.as_deref(),
    )
    .await?;
    Ok(Json(Data::new(
        models.into_iter().map(Into::into).collect(),
    )))
}

#[utoipa::path(
    post,
    path = "/scoring/models",
    tag = TAG,
    operation_id = "scoring_models_create",
    security(("session_cookie" = [])),
    description = "Creates a scoring model (`scoring_model.created`) for a record type (`contact`, `company`, `deal` or a custom object) and queues a full recalculation. Criteria are `profile` (a `filter` in the same format as views, validated against the fields of the type; `points` when the record matches) or `behavior` (a `signal`: `email_opened`, `email_clicked`, `email_replied`, `meeting_booked`, `form_submitted`, `call_connected`; `points` per occurrence, optional `max_points` cap and `decay_days`: an occurrence loses its points linearly until zero after that many days). Signals of contacts also count for the scoring models of their company. Bands: cold below `warm_from` (default 30), warm below `hot_from` (default 70), hot from there. The first model of a record type becomes its primary model, exposed as the read-only `score` field. Needs `scoring.manage`.",
    request_body = CreateScoringModelRequest,
    responses(
        (status = 201, description = "Scoring model created", body = Data<ScoringModelResponse>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = FORBIDDEN_MANAGE, body = ErrorEnvelope),
        (status = 404, description = "Unknown record type (`record_type_not_found`)", body = ErrorEnvelope),
        (status = 422, description = INVALID, body = ErrorEnvelope),
    )
)]
async fn create_scoring_model(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    JsonBody(request): JsonBody<CreateScoringModelRequest>,
) -> Result<(StatusCode, Json<Data<ScoringModelResponse>>), ApiError> {
    let model = service::create(
        &Ctx::from_state(&state, &tenant.scope).await?,
        request.into_command(client.0.ip),
    )
    .await?;
    Ok((StatusCode::CREATED, Json(Data::new(model.into()))))
}

#[utoipa::path(
    get,
    path = "/scoring/models/{model_id}",
    tag = TAG,
    operation_id = "scoring_models_get",
    security(("session_cookie" = [])),
    params(("model_id" = String, Path, description = "Scoring model id")),
    responses(
        (status = 200, description = "Scoring model", body = Data<ScoringModelResponse>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = FORBIDDEN_MANAGE, body = ErrorEnvelope),
        (status = 404, description = NOT_FOUND, body = ErrorEnvelope),
    )
)]
async fn get_scoring_model(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(model_id): Path<String>,
) -> Result<Json<Data<ScoringModelResponse>>, ApiError> {
    let model = service::get(&Ctx::from_state(&state, &tenant.scope).await?, &model_id).await?;
    Ok(Json(Data::new(model.into())))
}

#[utoipa::path(
    patch,
    path = "/scoring/models/{model_id}",
    tag = TAG,
    operation_id = "scoring_models_update",
    security(("session_cookie" = [])),
    description = "Updates a scoring model (`scoring_model.updated`). `criteria`, when sent, replaces the whole list (criteria keep their id by `key`, so past signals keep counting). Changing criteria, bands or activation queues a full recalculation. Marking it primary unmarks the previous primary model of the record type. The record type and the key never change. Needs `scoring.manage`.",
    params(
        ("model_id" = String, Path, description = "Scoring model id"),
        ("If-Match" = Option<String>, Header, description = "`updated_at` of the model as read (ISO-8601)"),
    ),
    request_body = UpdateScoringModelRequest,
    responses(
        (status = 200, description = "Scoring model updated", body = Data<ScoringModelResponse>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = FORBIDDEN_MANAGE, body = ErrorEnvelope),
        (status = 404, description = NOT_FOUND, body = ErrorEnvelope),
        (status = 409, description = "The model changed since the `If-Match` version (`version_conflict`)", body = ErrorEnvelope),
        (status = 422, description = INVALID, body = ErrorEnvelope),
    )
)]
async fn update_scoring_model(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    IfMatch(expected): IfMatch,
    Path(model_id): Path<String>,
    JsonBody(request): JsonBody<UpdateScoringModelRequest>,
) -> Result<Json<Data<ScoringModelResponse>>, ApiError> {
    let model = service::update(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &model_id,
        request.into_command(client.0.ip, expected),
    )
    .await?;
    Ok(Json(Data::new(model.into())))
}

#[utoipa::path(
    delete,
    path = "/scoring/models/{model_id}",
    tag = TAG,
    operation_id = "scoring_models_delete",
    security(("session_cookie" = [])),
    params(("model_id" = String, Path, description = "Scoring model id")),
    responses(
        (status = 204, description = "Scoring model and its scores removed (`scoring_model.deleted`)"),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = FORBIDDEN_MANAGE, body = ErrorEnvelope),
        (status = 404, description = NOT_FOUND, body = ErrorEnvelope),
    )
)]
async fn delete_scoring_model(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(model_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    service::delete(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &model_id,
        client.0.ip,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/scoring/models/{model_id}/recalculate",
    tag = TAG,
    operation_id = "scoring_models_recalculate",
    security(("session_cookie" = [])),
    description = "Queues a full recalculation of the model (job `scoring.recalculate`). Scores are also recalculated by the `scoring.events` consumer and by the daily `scoring.decay` job. Needs `scoring.manage`.",
    params(("model_id" = String, Path, description = "Scoring model id")),
    responses(
        (status = 202, description = "Recalculation queued"),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = FORBIDDEN_MANAGE, body = ErrorEnvelope),
        (status = 404, description = NOT_FOUND, body = ErrorEnvelope),
    )
)]
async fn recalculate_scoring_model(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(model_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    service::recalculate(&Ctx::from_state(&state, &tenant.scope).await?, &model_id).await?;
    Ok(StatusCode::ACCEPTED)
}

#[utoipa::path(
    get,
    path = "/scoring/records/{record_type}/{record_id}",
    tag = TAG,
    operation_id = "scoring_records_get",
    security(("session_cookie" = [])),
    description = "Scores of a record in every active scoring model of its type (primary first), with the band and the explanation: the criteria that added or removed points, and how many behavior occurrences counted. Needs read access to the record.",
    params(
        ("record_type" = String, Path, description = "Record type, e.g. `contact`"),
        ("record_id" = String, Path, description = "Record id"),
    ),
    responses(
        (status = 200, description = "Scores of the record", body = Data<Vec<RecordScoreResponse>>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = "Missing read permission on the record type (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown record type (`record_type_not_found`) or record not visible (`record_not_found`)", body = ErrorEnvelope),
    )
)]
async fn get_record_scores(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path((record_type, record_id)): Path<(String, String)>,
) -> Result<Json<Data<Vec<RecordScoreResponse>>>, ApiError> {
    let scores = service::record_scores(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &record_type,
        &record_id,
    )
    .await?;
    Ok(Json(Data::new(
        scores.into_iter().map(Into::into).collect(),
    )))
}

#[derive(Debug, Clone, Copy, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = ScoreBand)]
pub(crate) enum ScoreBandSchema {
    Cold,
    Warm,
    Hot,
}

impl From<ScoreBand> for ScoreBandSchema {
    fn from(value: ScoreBand) -> Self {
        match value {
            ScoreBand::Cold => Self::Cold,
            ScoreBand::Warm => Self::Warm,
            ScoreBand::Hot => Self::Hot,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = ScoringCriterionKind)]
pub(crate) enum ScoringCriterionKindSchema {
    Profile,
    Behavior,
}

impl From<CriterionKind> for ScoringCriterionKindSchema {
    fn from(value: CriterionKind) -> Self {
        match value {
            CriterionKind::Profile => Self::Profile,
            CriterionKind::Behavior => Self::Behavior,
        }
    }
}

impl From<ScoringCriterionKindSchema> for CriterionKind {
    fn from(value: ScoringCriterionKindSchema) -> Self {
        match value {
            ScoringCriterionKindSchema::Profile => Self::Profile,
            ScoringCriterionKindSchema::Behavior => Self::Behavior,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = ScoringSignal)]
pub(crate) enum ScoringSignalSchema {
    EmailOpened,
    EmailClicked,
    EmailReplied,
    MeetingBooked,
    FormSubmitted,
    CallConnected,
}

impl From<BehaviorSignal> for ScoringSignalSchema {
    fn from(value: BehaviorSignal) -> Self {
        match value {
            BehaviorSignal::EmailOpened => Self::EmailOpened,
            BehaviorSignal::EmailClicked => Self::EmailClicked,
            BehaviorSignal::EmailReplied => Self::EmailReplied,
            BehaviorSignal::MeetingBooked => Self::MeetingBooked,
            BehaviorSignal::FormSubmitted => Self::FormSubmitted,
            BehaviorSignal::CallConnected => Self::CallConnected,
        }
    }
}

impl From<ScoringSignalSchema> for BehaviorSignal {
    fn from(value: ScoringSignalSchema) -> Self {
        match value {
            ScoringSignalSchema::EmailOpened => Self::EmailOpened,
            ScoringSignalSchema::EmailClicked => Self::EmailClicked,
            ScoringSignalSchema::EmailReplied => Self::EmailReplied,
            ScoringSignalSchema::MeetingBooked => Self::MeetingBooked,
            ScoringSignalSchema::FormSubmitted => Self::FormSubmitted,
            ScoringSignalSchema::CallConnected => Self::CallConnected,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct ScoringCriterionResponse {
    pub(crate) id: String,
    #[schema(example = "replied_email")]
    pub(crate) key: String,
    #[schema(example = "Replied to email")]
    pub(crate) label: String,
    pub(crate) kind: ScoringCriterionKindSchema,
    #[schema(value_type = Option<Object>)]
    pub(crate) filter: Option<Value>,
    pub(crate) signal: Option<ScoringSignalSchema>,
    #[schema(example = 10)]
    pub(crate) points: i32,
    #[schema(example = 30)]
    pub(crate) max_points: Option<i32>,
    #[schema(example = 30)]
    pub(crate) decay_days: Option<i32>,
    pub(crate) position: i32,
}

impl From<ScoringCriterion> for ScoringCriterionResponse {
    fn from(criterion: ScoringCriterion) -> Self {
        Self {
            id: criterion.id.to_string(),
            kind: criterion.rule.kind().into(),
            filter: criterion
                .rule
                .filter()
                .and_then(|filter| serde_json::to_value(filter).ok()),
            signal: criterion.rule.signal().map(Into::into),
            max_points: criterion.rule.max_points(),
            decay_days: criterion.rule.decay_days(),
            key: criterion.key,
            label: criterion.label,
            points: criterion.points,
            position: criterion.position,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct ScoringModelResponse {
    pub(crate) id: String,
    #[schema(example = "lead_score")]
    pub(crate) key: String,
    #[schema(example = "contact")]
    pub(crate) record_type: String,
    #[schema(example = "Lead scoring")]
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) is_primary: bool,
    pub(crate) is_active: bool,
    #[schema(example = 30)]
    pub(crate) warm_from: i32,
    #[schema(example = 70)]
    pub(crate) hot_from: i32,
    pub(crate) criteria: Vec<ScoringCriterionResponse>,
    pub(crate) recalculated_at: Option<String>,
    pub(crate) created_by: Option<String>,
    pub(crate) updated_by: Option<String>,
    #[schema(example = "2026-10-26T13:45:00Z")]
    pub(crate) created_at: String,
    #[schema(example = "2026-10-26T13:45:00Z")]
    pub(crate) updated_at: String,
}

impl From<ScoringModel> for ScoringModelResponse {
    fn from(model: ScoringModel) -> Self {
        Self {
            id: model.id.to_string(),
            key: model.key,
            record_type: model.record_type,
            name: model.name,
            description: model.description,
            is_primary: model.is_primary,
            is_active: model.is_active,
            warm_from: model.bands.warm_from,
            hot_from: model.bands.hot_from,
            criteria: model.criteria.into_iter().map(Into::into).collect(),
            recalculated_at: model.recalculated_at.map(timestamp),
            created_by: model.created_by.map(|id| id.into_inner()),
            updated_by: model.updated_by.map(|id| id.into_inner()),
            created_at: timestamp(model.created_at),
            updated_at: timestamp(model.updated_at),
        }
    }
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct ScoringModelListParams {
    pub(crate) record_type: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct ScoringCriterionRequest {
    #[schema(example = "replied_email")]
    pub(crate) key: Option<String>,
    pub(crate) label: String,
    pub(crate) kind: ScoringCriterionKindSchema,
    #[schema(value_type = Option<Object>)]
    pub(crate) filter: Option<Value>,
    pub(crate) signal: Option<ScoringSignalSchema>,
    pub(crate) points: i64,
    pub(crate) max_points: Option<i64>,
    pub(crate) decay_days: Option<i64>,
}

impl From<ScoringCriterionRequest> for CriterionInput {
    fn from(request: ScoringCriterionRequest) -> Self {
        Self {
            key: request.key,
            label: request.label,
            kind: CriterionKind::from(request.kind).as_str().to_owned(),
            filter: request.filter,
            signal: request
                .signal
                .map(|signal| BehaviorSignal::from(signal).as_str().to_owned()),
            points: request.points,
            max_points: request.max_points,
            decay_days: request.decay_days,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct CreateScoringModelRequest {
    #[schema(example = "contact")]
    pub(crate) record_type: String,
    #[schema(example = "lead_score")]
    pub(crate) key: Option<String>,
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) is_primary: Option<bool>,
    pub(crate) is_active: Option<bool>,
    pub(crate) warm_from: Option<i64>,
    pub(crate) hot_from: Option<i64>,
    #[serde(default)]
    pub(crate) criteria: Vec<ScoringCriterionRequest>,
}

impl CreateScoringModelRequest {
    pub(crate) fn into_command(self, ip: Option<String>) -> CreateScoringModelCommand {
        CreateScoringModelCommand {
            record_type: self.record_type,
            key: self.key,
            name: self.name,
            description: self.description,
            is_primary: self.is_primary,
            is_active: self.is_active,
            warm_from: self.warm_from,
            hot_from: self.hot_from,
            criteria: self.criteria.into_iter().map(Into::into).collect(),
            ip,
        }
    }
}

#[derive(Debug, Default, Deserialize, ToSchema)]
pub(crate) struct UpdateScoringModelRequest {
    pub(crate) name: Option<String>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub(crate) description: Option<Option<String>>,
    pub(crate) is_primary: Option<bool>,
    pub(crate) is_active: Option<bool>,
    pub(crate) warm_from: Option<i64>,
    pub(crate) hot_from: Option<i64>,
    pub(crate) criteria: Option<Vec<ScoringCriterionRequest>>,
}

impl UpdateScoringModelRequest {
    pub(crate) fn into_command(
        self,
        ip: Option<String>,
        expected_updated_at: Option<DateTime<Utc>>,
    ) -> UpdateScoringModelCommand {
        UpdateScoringModelCommand {
            name: self.name,
            description: self.description,
            is_primary: self.is_primary,
            is_active: self.is_active,
            warm_from: self.warm_from,
            hot_from: self.hot_from,
            criteria: self
                .criteria
                .map(|criteria| criteria.into_iter().map(Into::into).collect()),
            expected_updated_at,
            ip,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct ScoreContributionResponse {
    pub(crate) criterion_id: String,
    pub(crate) key: String,
    pub(crate) label: String,
    pub(crate) kind: ScoringCriterionKindSchema,
    #[schema(example = 18)]
    pub(crate) points: i32,
    #[schema(example = 2)]
    pub(crate) occurrences: u32,
}

impl From<ScoreContribution> for ScoreContributionResponse {
    fn from(contribution: ScoreContribution) -> Self {
        Self {
            criterion_id: contribution.criterion_id.to_string(),
            key: contribution.key,
            label: contribution.label,
            kind: contribution.kind.into(),
            points: contribution.points,
            occurrences: contribution.occurrences,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct RecordScoreResponse {
    pub(crate) model_id: String,
    pub(crate) model_key: String,
    pub(crate) model_name: String,
    pub(crate) is_primary: bool,
    #[schema(example = 42)]
    pub(crate) score: i32,
    pub(crate) band: ScoreBandSchema,
    pub(crate) warm_from: i32,
    pub(crate) hot_from: i32,
    pub(crate) explanation: Vec<ScoreContributionResponse>,
    pub(crate) calculated_at: Option<String>,
}

impl From<RecordScoreView> for RecordScoreResponse {
    fn from(view: RecordScoreView) -> Self {
        Self {
            model_id: view.model.id.to_string(),
            model_key: view.model.key,
            model_name: view.model.name,
            is_primary: view.model.is_primary,
            score: view.score,
            band: view.band.into(),
            warm_from: view.model.bands.warm_from,
            hot_from: view.model.bands.hot_from,
            explanation: view.explanation.into_iter().map(Into::into).collect(),
            calculated_at: view.calculated_at.map(timestamp),
        }
    }
}
