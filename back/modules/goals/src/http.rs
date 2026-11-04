use axum::Json;
use axum::extract::rejection::QueryRejection;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::domain::{
    CreateGoalCommand, DATE_FORMAT, GoalCreditMode, GoalListQuery, GoalMetric, GoalOwnerInput,
    GoalOwnerKind, GoalOwnerOption, GoalOwnerOptions, GoalPeriod, GoalProgress, GoalStatus,
    GoalView, UpdateGoalCommand,
};
use crate::service;
use aphel_kernel::Ctx;
use aphel_kernel::api::dto::common::{Data, ErrorEnvelope, Page, timestamp};
use aphel_kernel::api::error::ApiError;
use aphel_kernel::api::extractors::{
    AuthContext, ClientInfo, IfMatch, JsonBody, Pagination, PaginationQuery,
};
use aphel_kernel::api::routes::v1::members::window;
use aphel_kernel::api::state::AppState;
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;
use aphel_sdk::money::DecimalInputSchema;

pub(crate) const TAG: &str = "goals";

fn money(value: Decimal) -> String {
    format!("{:.2}", value.round_dp(2))
}

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_goals, create_goal))
        .routes(routes!(goal_owner_options))
        .routes(routes!(get_goal, update_goal, delete_goal))
}

#[utoipa::path(
    get,
    path = "/goals",
    tag = TAG,
    operation_id = "goals_list",
    security(("session_cookie" = [])),
    params(GoalListParams, PaginationQuery),
    responses(
        (status = 200, description = "Goals the user reaches: organization goals, goals of the user's teams and the user's own goals; with `goals.manage` at team scope also the goals of the team members, and at organization scope every goal. Newest period first. `current=true` keeps the goals whose period contains today (organization timezone) and `active_on` the ones containing that day. Unless `progress=false`, each goal carries its progress computed by the aggregate engine with the read scope and field permissions of the caller (`progress: null` when the caller cannot read the records or the metric field)", body = Page<GoalResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid filter or pagination (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn list_goals(
    State(state): State<AppState>,
    tenant: AuthContext,
    params: Result<Query<GoalListParams>, QueryRejection>,
    pagination: Pagination,
) -> Result<Json<Page<GoalResponse>>, ApiError> {
    let Query(params) = params.map_err(|rejection| AppError::Validation(rejection.body_text()))?;
    let listing = service::list(
        &Ctx::from_state(&state, &tenant.scope).await?,
        params.into_query(),
        window(pagination),
    )
    .await?;
    Ok(Json(Page::new(
        listing.items.into_iter().map(Into::into).collect(),
        listing.total,
        pagination,
    )))
}

#[utoipa::path(
    post,
    path = "/goals",
    tag = TAG,
    operation_id = "goals_create",
    security(("session_cookie" = [])),
    request_body = CreateGoalRequest,
    responses(
        (status = 201, description = "Goal created (`goal.created`). `starts_on` is moved to the first day of its month, quarter or year and `ends_on` is the last day of that period. Count metrics take whole targets. Needs `goals.manage`: organization scope for any owner, team scope for the user's teams and their members, own scope only for the user's own goals. `credit_mode` (`owner`, default, or `split`) chooses how `won_amount` goals of users and teams count a deal: the whole amount for its owner, or the amount times the revenue credit of each member of the deal team (the owner keeps the share not given to members)", body = Data<GoalResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `goals.manage` or the scope does not cover the owner (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 409, description = "A goal with the same owner, metric and period already exists (`goal_already_exists`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid owner, metric, period, date or target (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn create_goal(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    JsonBody(request): JsonBody<CreateGoalRequest>,
) -> Result<(StatusCode, Json<Data<GoalResponse>>), ApiError> {
    let goal = service::create(
        &Ctx::from_state(&state, &tenant.scope).await?,
        request.into_command(client.0.ip),
    )
    .await?;
    Ok((StatusCode::CREATED, Json(Data::new(goal.into()))))
}

#[utoipa::path(
    get,
    path = "/goals/owners",
    tag = TAG,
    operation_id = "goals_owner_options",
    security(("session_cookie" = [])),
    responses(
        (status = 200, description = "Owners the user can create goals for with the scope of `goals.manage`: `organization` tells whether organization goals are allowed, `users` are active members and `teams` are teams. Empty without the permission", body = Data<GoalOwnerOptionsResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
    )
)]
async fn goal_owner_options(
    State(state): State<AppState>,
    tenant: AuthContext,
) -> Result<Json<Data<GoalOwnerOptionsResponse>>, ApiError> {
    let options = service::owner_options(&Ctx::from_state(&state, &tenant.scope).await?).await?;
    Ok(Json(Data::new(options.into())))
}

#[utoipa::path(
    get,
    path = "/goals/{goal_id}",
    tag = TAG,
    operation_id = "goals_get",
    security(("session_cookie" = [])),
    params(("goal_id" = String, Path, description = "Goal id")),
    responses(
        (status = 200, description = "Goal with its progress (read scope of the caller)", body = Data<GoalResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 404, description = "Goal not found or not visible to the user (`goal_not_found`)", body = ErrorEnvelope),
    )
)]
async fn get_goal(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(goal_id): Path<String>,
) -> Result<Json<Data<GoalResponse>>, ApiError> {
    let goal = service::get(&Ctx::from_state(&state, &tenant.scope).await?, &goal_id).await?;
    Ok(Json(Data::new(goal.into())))
}

#[utoipa::path(
    patch,
    path = "/goals/{goal_id}",
    tag = TAG,
    operation_id = "goals_update",
    security(("session_cookie" = [])),
    params(
        ("goal_id" = String, Path, description = "Goal id"),
        ("If-Match" = Option<String>, Header, description = "`updated_at` of the goal as read (ISO-8601, quotes optional); the update fails with 409 when it changed since"),
    ),
    request_body = UpdateGoalRequest,
    responses(
        (status = 200, description = "Goal updated (`goal.updated`); omitted properties stay unchanged. Changing the owner sends `owner_type` with `user_id` or `team_id`. The scope of `goals.manage` must cover the current and the new owner", body = Data<GoalResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `goals.manage` or the scope does not cover the owner (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Goal not found (`goal_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "Duplicate goal (`goal_already_exists`) or the goal changed since the `If-Match` version (`version_conflict`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid properties or malformed `If-Match` (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn update_goal(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    IfMatch(expected): IfMatch,
    Path(goal_id): Path<String>,
    JsonBody(request): JsonBody<UpdateGoalRequest>,
) -> Result<Json<Data<GoalResponse>>, ApiError> {
    let goal = service::update(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &goal_id,
        request.into_command(client.0.ip, expected),
    )
    .await?;
    Ok(Json(Data::new(goal.into())))
}

#[utoipa::path(
    delete,
    path = "/goals/{goal_id}",
    tag = TAG,
    operation_id = "goals_delete",
    security(("session_cookie" = [])),
    params(("goal_id" = String, Path, description = "Goal id")),
    responses(
        (status = 204, description = "Goal deleted (`goal.deleted`)"),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `goals.manage` or the scope does not cover the owner (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Goal not found (`goal_not_found`)", body = ErrorEnvelope),
    )
)]
async fn delete_goal(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(goal_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    service::delete(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &goal_id,
        client.0.ip,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

fn decimal_text(value: Value) -> String {
    match value {
        Value::String(text) => text,
        other => other.to_string(),
    }
}

fn quantity(metric: GoalMetric, value: Decimal) -> String {
    if metric.is_count() {
        value.trunc().normalize().to_string()
    } else {
        money(value)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = GoalOwnerType)]
pub(crate) enum GoalOwnerTypeSchema {
    User,
    Team,
    Organization,
}

impl From<GoalOwnerKind> for GoalOwnerTypeSchema {
    fn from(value: GoalOwnerKind) -> Self {
        match value {
            GoalOwnerKind::User => Self::User,
            GoalOwnerKind::Team => Self::Team,
            GoalOwnerKind::Organization => Self::Organization,
        }
    }
}

impl From<GoalOwnerTypeSchema> for GoalOwnerKind {
    fn from(value: GoalOwnerTypeSchema) -> Self {
        match value {
            GoalOwnerTypeSchema::User => Self::User,
            GoalOwnerTypeSchema::Team => Self::Team,
            GoalOwnerTypeSchema::Organization => Self::Organization,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = GoalMetric)]
pub(crate) enum GoalMetricSchema {
    WonAmount,
    WonCount,
    ActivitiesDone,
    NewContacts,
}

impl From<GoalMetric> for GoalMetricSchema {
    fn from(value: GoalMetric) -> Self {
        match value {
            GoalMetric::WonAmount => Self::WonAmount,
            GoalMetric::WonCount => Self::WonCount,
            GoalMetric::ActivitiesDone => Self::ActivitiesDone,
            GoalMetric::NewContacts => Self::NewContacts,
        }
    }
}

impl From<GoalMetricSchema> for GoalMetric {
    fn from(value: GoalMetricSchema) -> Self {
        match value {
            GoalMetricSchema::WonAmount => Self::WonAmount,
            GoalMetricSchema::WonCount => Self::WonCount,
            GoalMetricSchema::ActivitiesDone => Self::ActivitiesDone,
            GoalMetricSchema::NewContacts => Self::NewContacts,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = GoalPeriod)]
pub(crate) enum GoalPeriodSchema {
    Month,
    Quarter,
    Year,
}

impl From<GoalPeriod> for GoalPeriodSchema {
    fn from(value: GoalPeriod) -> Self {
        match value {
            GoalPeriod::Month => Self::Month,
            GoalPeriod::Quarter => Self::Quarter,
            GoalPeriod::Year => Self::Year,
        }
    }
}

impl From<GoalPeriodSchema> for GoalPeriod {
    fn from(value: GoalPeriodSchema) -> Self {
        match value {
            GoalPeriodSchema::Month => Self::Month,
            GoalPeriodSchema::Quarter => Self::Quarter,
            GoalPeriodSchema::Year => Self::Year,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = GoalCreditMode)]
pub(crate) enum GoalCreditModeSchema {
    Owner,
    Split,
}

impl From<GoalCreditMode> for GoalCreditModeSchema {
    fn from(value: GoalCreditMode) -> Self {
        match value {
            GoalCreditMode::Owner => Self::Owner,
            GoalCreditMode::Split => Self::Split,
        }
    }
}

impl From<GoalCreditModeSchema> for GoalCreditMode {
    fn from(value: GoalCreditModeSchema) -> Self {
        match value {
            GoalCreditModeSchema::Owner => Self::Owner,
            GoalCreditModeSchema::Split => Self::Split,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = GoalStatus)]
pub(crate) enum GoalStatusSchema {
    Upcoming,
    OnTrack,
    Behind,
    Achieved,
    Missed,
}

impl From<GoalStatus> for GoalStatusSchema {
    fn from(value: GoalStatus) -> Self {
        match value {
            GoalStatus::Upcoming => Self::Upcoming,
            GoalStatus::OnTrack => Self::OnTrack,
            GoalStatus::Behind => Self::Behind,
            GoalStatus::Achieved => Self::Achieved,
            GoalStatus::Missed => Self::Missed,
        }
    }
}

#[derive(Debug, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct GoalListParams {
    pub(crate) owner_type: Option<GoalOwnerTypeSchema>,
    pub(crate) user_id: Option<String>,
    pub(crate) team_id: Option<String>,
    pub(crate) metric: Option<GoalMetricSchema>,
    pub(crate) period: Option<GoalPeriodSchema>,
    #[param(example = "2026-10-21")]
    pub(crate) active_on: Option<String>,
    pub(crate) current: Option<bool>,
    pub(crate) progress: Option<bool>,
}

impl GoalListParams {
    pub(crate) fn into_query(self) -> GoalListQuery {
        GoalListQuery {
            owner_type: self.owner_type.map(Into::into),
            user_id: self.user_id,
            team_id: self.team_id,
            metric: self.metric.map(Into::into),
            period: self.period.map(Into::into),
            active_on: self.active_on,
            current: self.current.unwrap_or(false),
            with_progress: self.progress.unwrap_or(true),
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreateGoalRequest {
    #[schema(example = "user")]
    pub(crate) owner_type: GoalOwnerTypeSchema,
    pub(crate) user_id: Option<String>,
    pub(crate) team_id: Option<String>,
    #[schema(example = "won_amount")]
    pub(crate) metric: GoalMetricSchema,
    #[schema(example = "month")]
    pub(crate) period: GoalPeriodSchema,
    #[schema(example = "2026-10-01")]
    pub(crate) starts_on: String,
    #[schema(value_type = DecimalInputSchema, example = "50000.00")]
    pub(crate) target: Value,
    #[schema(example = "owner")]
    pub(crate) credit_mode: Option<GoalCreditModeSchema>,
}

impl CreateGoalRequest {
    pub(crate) fn into_command(self, ip: Option<String>) -> CreateGoalCommand {
        CreateGoalCommand {
            owner: GoalOwnerInput {
                owner_type: self.owner_type.into(),
                user_id: self.user_id,
                team_id: self.team_id,
            },
            metric: self.metric.into(),
            period: self.period.into(),
            starts_on: self.starts_on,
            target: decimal_text(self.target),
            credit_mode: self.credit_mode.map(Into::into),
            ip,
        }
    }
}

#[derive(Debug, Default, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct UpdateGoalRequest {
    pub(crate) owner_type: Option<GoalOwnerTypeSchema>,
    pub(crate) user_id: Option<String>,
    pub(crate) team_id: Option<String>,
    pub(crate) metric: Option<GoalMetricSchema>,
    pub(crate) period: Option<GoalPeriodSchema>,
    #[schema(example = "2026-10-01")]
    pub(crate) starts_on: Option<String>,
    #[schema(value_type = Option<DecimalInputSchema>, example = "50000.00")]
    pub(crate) target: Option<Value>,
    pub(crate) credit_mode: Option<GoalCreditModeSchema>,
}

impl UpdateGoalRequest {
    pub(crate) fn into_command(
        self,
        ip: Option<String>,
        expected_updated_at: Option<DateTime<Utc>>,
    ) -> UpdateGoalCommand {
        UpdateGoalCommand {
            owner: self.owner_type.map(|owner_type| GoalOwnerInput {
                owner_type: owner_type.into(),
                user_id: self.user_id,
                team_id: self.team_id,
            }),
            metric: self.metric.map(Into::into),
            period: self.period.map(Into::into),
            starts_on: self.starts_on,
            target: self.target.map(decimal_text),
            credit_mode: self.credit_mode.map(Into::into),
            ip,
            expected_updated_at,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct GoalProgressResponse {
    #[schema(example = "32500.00")]
    pub(crate) actual: String,
    #[schema(example = 0.65)]
    pub(crate) ratio: f64,
    #[schema(example = 0.68)]
    pub(crate) expected_ratio: f64,
    pub(crate) status: GoalStatusSchema,
}

impl GoalProgressResponse {
    fn new(metric: GoalMetric, progress: GoalProgress) -> Self {
        Self {
            actual: quantity(metric, progress.actual),
            ratio: progress.ratio,
            expected_ratio: progress.expected_ratio,
            status: progress.status.into(),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct GoalResponse {
    pub(crate) id: String,
    pub(crate) owner_type: GoalOwnerTypeSchema,
    pub(crate) user_id: Option<String>,
    pub(crate) team_id: Option<String>,
    pub(crate) owner_name: Option<String>,
    pub(crate) metric: GoalMetricSchema,
    pub(crate) period: GoalPeriodSchema,
    #[schema(example = "2026-10-01")]
    pub(crate) starts_on: String,
    #[schema(example = "2026-10-31")]
    pub(crate) ends_on: String,
    #[schema(example = "50000.00")]
    pub(crate) target: String,
    pub(crate) credit_mode: GoalCreditModeSchema,
    #[schema(example = "BRL")]
    pub(crate) currency: Option<String>,
    pub(crate) can_manage: bool,
    pub(crate) progress: Option<GoalProgressResponse>,
    pub(crate) created_by: Option<String>,
    #[schema(example = "2026-10-20T13:45:00Z")]
    pub(crate) created_at: String,
    #[schema(example = "2026-10-20T13:45:00Z")]
    pub(crate) updated_at: String,
}

impl From<GoalView> for GoalResponse {
    fn from(view: GoalView) -> Self {
        let goal = view.goal;
        let metric = goal.metric;
        Self {
            id: goal.id.to_string(),
            owner_type: goal.owner.kind().into(),
            user_id: goal.owner.user_id().map(|id| id.as_str().to_owned()),
            team_id: goal.owner.team_id().map(|id| id.as_str().to_owned()),
            owner_name: goal.owner_name,
            metric: metric.into(),
            period: goal.period.into(),
            starts_on: goal.starts_on.format(DATE_FORMAT).to_string(),
            ends_on: goal.ends_on.format(DATE_FORMAT).to_string(),
            target: quantity(metric, goal.target),
            credit_mode: goal.credit_mode.into(),
            currency: view.currency,
            can_manage: view.can_manage,
            progress: view
                .progress
                .map(|progress| GoalProgressResponse::new(metric, progress)),
            created_by: goal.created_by.map(UserId::into_inner),
            created_at: timestamp(goal.created_at),
            updated_at: timestamp(goal.updated_at),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct GoalOwnerOptionResponse {
    pub(crate) id: String,
    pub(crate) name: String,
}

impl From<GoalOwnerOption> for GoalOwnerOptionResponse {
    fn from(option: GoalOwnerOption) -> Self {
        Self {
            id: option.id,
            name: option.name,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct GoalOwnerOptionsResponse {
    pub(crate) can_manage: bool,
    pub(crate) organization: bool,
    pub(crate) users: Vec<GoalOwnerOptionResponse>,
    pub(crate) teams: Vec<GoalOwnerOptionResponse>,
}

impl From<GoalOwnerOptions> for GoalOwnerOptionsResponse {
    fn from(options: GoalOwnerOptions) -> Self {
        Self {
            can_manage: options.organization || !options.users.is_empty(),
            organization: options.organization,
            users: options.users.into_iter().map(Into::into).collect(),
            teams: options.teams.into_iter().map(Into::into).collect(),
        }
    }
}
