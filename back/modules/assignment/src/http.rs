use axum::Json;
use axum::extract::rejection::QueryRejection;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::dto::{
    LeadDistributionAssignmentListParams, LeadDistributionAssignmentResponse,
    LeadDistributionRuleListParams, LeadDistributionRuleRequest, LeadDistributionRuleResponse,
    LeadDistributionSimulateRequest, LeadDistributionSimulationResponse,
};
use crate::service;
use aphel_kernel::Ctx;
use aphel_kernel::api::dto::common::{Data, ErrorEnvelope, Page};
use aphel_kernel::api::error::ApiError;
use aphel_kernel::api::extractors::{
    AuthContext, ClientInfo, IfMatch, JsonBody, Pagination, PaginationQuery,
};
use aphel_kernel::api::routes::v1::members::window;
use aphel_kernel::api::state::AppState;
use aphel_kernel::errors::AppError;

pub(crate) const TAG: &str = "assignment";

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_targets))
        .routes(routes!(list_rules, create_rule))
        .routes(routes!(get_rule, update_rule, delete_rule))
        .routes(routes!(simulate_rule))
        .routes(routes!(list_assignments))
}

#[utoipa::path(
    get,
    path = "/assignment/assignments",
    tag = TAG,
    operation_id = "assignment_assignments_list",
    security(("session_cookie" = [])),
    params(LeadDistributionAssignmentListParams, PaginationQuery),
    responses(
        (status = 200, description = "Assignment history of the distribution rules, newest first: who received which record, from which rule and source (`automation`, `incoming_webhook`, `import`, `scheduling`, `lead_form`). `user_id` is null when the user was removed and `record_type`/`record_id` are null for picks without a record. Entries of a deleted rule are deleted with it", body = Page<LeadDistributionAssignmentResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `assignment.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid filter or pagination (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn list_assignments(
    State(state): State<AppState>,
    tenant: AuthContext,
    params: Result<Query<LeadDistributionAssignmentListParams>, QueryRejection>,
    pagination: Pagination,
) -> Result<Json<Page<LeadDistributionAssignmentResponse>>, ApiError> {
    let Query(params) = params.map_err(|rejection| AppError::Validation(rejection.body_text()))?;
    let listing = service::list_assignments(
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
    get,
    path = "/assignment/rules",
    tag = TAG,
    operation_id = "assignment_rules_list",
    security(("session_cookie" = [])),
    params(LeadDistributionRuleListParams, PaginationQuery),
    responses(
        (status = 200, description = "Lead distribution rules of the organization ordered by name, with members (name, active, away now), routing and usage (`assignments`, `last_user_id`, `last_assigned_at`). Needs `assignment.manage` or `automations.manage` (to pick a rule in an automation)", body = Page<LeadDistributionRuleResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `assignment.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid pagination (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn list_rules(
    State(state): State<AppState>,
    tenant: AuthContext,
    Query(params): Query<LeadDistributionRuleListParams>,
    pagination: Pagination,
) -> Result<Json<Page<LeadDistributionRuleResponse>>, ApiError> {
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
    path = "/assignment/rules",
    tag = TAG,
    operation_id = "assignment_rules_create",
    security(("session_cookie" = [])),
    request_body = LeadDistributionRuleRequest,
    responses(
        (status = 201, description = "Rule created (`assignment_rule.created`). `record_type` is any registered entity with an owner; `strategy` is `round_robin` (whoever waited longest), `weighted` (smooth weighted rotation by `weight` 1–100) or `least_loaded` (fewest active records of the rule target; active states come from owner metadata). Members marked `away` (optionally between `away_from` and `away_until`) and users that are not active in the organization are skipped. `routing` maps values of a record field to a team and/or users (first matching route wins, values compared without case); without a match the rule falls back to the members (`fallback: members`) or leaves the record unassigned (`none`). `key` defaults to one derived from the name", body = Data<LeadDistributionRuleResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `assignment.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 409, description = "Another rule already uses the key (`assignment_rule_key_taken`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid properties, unknown users, teams or field (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn create_rule(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    JsonBody(request): JsonBody<LeadDistributionRuleRequest>,
) -> Result<(StatusCode, Json<Data<LeadDistributionRuleResponse>>), ApiError> {
    let rule = service::create(
        &Ctx::from_state(&state, &tenant.scope).await?,
        request.into_command(client.0.ip, None),
    )
    .await?;
    Ok((StatusCode::CREATED, Json(Data::new(rule.into()))))
}

#[utoipa::path(
    get,
    path = "/assignment/rules/{rule_id}",
    tag = TAG,
    operation_id = "assignment_rules_get",
    security(("session_cookie" = [])),
    params(("rule_id" = String, Path, description = "Rule id")),
    responses(
        (status = 200, description = "Rule with members, routing and usage", body = Data<LeadDistributionRuleResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `assignment.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Rule not found (`assignment_rule_not_found`)", body = ErrorEnvelope),
    )
)]
async fn get_rule(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(rule_id): Path<String>,
) -> Result<Json<Data<LeadDistributionRuleResponse>>, ApiError> {
    let rule = service::get(&Ctx::from_state(&state, &tenant.scope).await?, &rule_id).await?;
    Ok(Json(Data::new(rule.into())))
}

#[utoipa::path(
    patch,
    path = "/assignment/rules/{rule_id}",
    tag = TAG,
    operation_id = "assignment_rules_update",
    security(("session_cookie" = [])),
    params(
        ("rule_id" = String, Path, description = "Rule id"),
        ("If-Match" = Option<String>, Header, description = "`updated_at` of the rule as read (ISO-8601, quotes optional); the update fails with 409 when it changed since"),
    ),
    request_body = LeadDistributionRuleRequest,
    responses(
        (status = 200, description = "Rule updated (`assignment_rule.updated`); omitted properties stay unchanged, `members` replaces the whole list and `routing: null` removes the routing. The rotation state is kept", body = Data<LeadDistributionRuleResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `assignment.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Rule not found (`assignment_rule_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "Key already used (`assignment_rule_key_taken`) or the rule changed since the `If-Match` version (`version_conflict`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid properties or malformed `If-Match` (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn update_rule(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    IfMatch(expected): IfMatch,
    Path(rule_id): Path<String>,
    JsonBody(request): JsonBody<LeadDistributionRuleRequest>,
) -> Result<Json<Data<LeadDistributionRuleResponse>>, ApiError> {
    let rule = service::update(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &rule_id,
        request.into_command(client.0.ip, expected),
    )
    .await?;
    Ok(Json(Data::new(rule.into())))
}

#[utoipa::path(
    delete,
    path = "/assignment/rules/{rule_id}",
    tag = TAG,
    operation_id = "assignment_rules_delete",
    security(("session_cookie" = [])),
    params(("rule_id" = String, Path, description = "Rule id")),
    responses(
        (status = 204, description = "Rule deleted with its rotation state and assignment log (`assignment_rule.deleted`). Automations, incoming webhooks and imports that still point to it stop assigning (the automation step fails with `assignment_rule_not_found`)"),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `assignment.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Rule not found (`assignment_rule_not_found`)", body = ErrorEnvelope),
    )
)]
async fn delete_rule(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(rule_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    service::delete(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &rule_id,
        client.0.ip,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/assignment/rules/{rule_id}/simulate",
    tag = TAG,
    operation_id = "assignment_rules_simulate",
    security(("session_cookie" = [])),
    params(("rule_id" = String, Path, description = "Rule id")),
    request_body = LeadDistributionSimulateRequest,
    responses(
        (status = 200, description = "Who would receive the next `count` (default 10, at most 50) records from the current rotation state, without writing anything. `values` are record field values used for the routing (`selection` tells whether the members, a route (`route_index`), the fallback or nobody would be used). `skipped` lists away and inactive users of the pool. An inactive rule simulates as if it were active", body = Data<LeadDistributionSimulationResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `assignment.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Rule not found (`assignment_rule_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid count (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn simulate_rule(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(rule_id): Path<String>,
    JsonBody(request): JsonBody<LeadDistributionSimulateRequest>,
) -> Result<Json<Data<LeadDistributionSimulationResponse>>, ApiError> {
    let simulation = service::simulate(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &rule_id,
        request.into_command(),
    )
    .await?;
    Ok(Json(Data::new(simulation.into())))
}

#[utoipa::path(get,path="/assignment/targets",tag=TAG,operation_id="assignment_targets",security(("session_cookie"=[])),responses((status=200,description="Registered entities with assignment ownership support",body=Data<Vec<service::AssignmentTarget>>)))]
async fn list_targets(
    State(state): State<AppState>,
    tenant: AuthContext,
) -> Result<Json<Data<Vec<service::AssignmentTarget>>>, ApiError> {
    Ok(Json(Data::new(service::targets(
        &Ctx::from_state(&state, &tenant.scope).await?,
    )?)))
}
