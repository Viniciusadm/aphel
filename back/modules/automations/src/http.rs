use axum::Json;
use axum::extract::rejection::QueryRejection;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
use uuid::Uuid;

use crate::dto::{
    ActionCatalogEntry, AutomationCatalogResponse, AutomationListItemResponse, AutomationResponse,
    AutomationRunDetailsResponse, AutomationRunResponse, AutomationVersionResponse,
    CreateAutomationRequest, DryRunRequest, DryRunResponse, ListAutomationsParams, ListRunsParams,
    RunAutomationRequest, TriggerCatalogEntry, UpdateAutomationRequest,
};
use crate::service;
use crate::{AutomationListFilter, RunListFilter};
use aphel_kernel::Ctx;
use aphel_kernel::api::dto::common::{Data, ErrorEnvelope, Page};
use aphel_kernel::api::error::ApiError;
use aphel_kernel::api::extractors::{
    AuthContext, ClientInfo, IfMatch, JsonBody, Pagination, PaginationQuery,
};
use aphel_kernel::api::routes::v1::members::window;
use aphel_kernel::api::state::AppState;
use aphel_kernel::errors::{AppError, FieldViolation};

pub(crate) const TAG: &str = "automations";

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_automations, create_automation))
        .routes(routes!(automation_catalog))
        .routes(routes!(
            get_automation,
            update_automation,
            delete_automation
        ))
        .routes(routes!(publish_automation))
        .routes(routes!(pause_automation))
        .routes(routes!(activate_automation))
        .routes(routes!(dry_run_automation))
        .routes(routes!(run_automation))
        .routes(routes!(list_versions))
        .routes(routes!(list_runs))
        .routes(routes!(get_run))
}

fn query_params<T>(query: Result<Query<T>, QueryRejection>) -> Result<T, AppError> {
    query
        .map(|Query(params)| params)
        .map_err(|rejection| AppError::Validation(rejection.body_text()))
}

fn invalid_param(field: &str, message: &str) -> AppError {
    AppError::invalid_fields(
        "invalid query",
        vec![FieldViolation::new(field.to_owned(), message.to_owned())],
    )
}

#[utoipa::path(
    get,
    path = "/automations",
    tag = TAG,
    operation_id = "automations_list",
    security(("session_cookie" = [])),
    params(ListAutomationsParams, PaginationQuery),
    responses(
        (status = 200, description = "Automations of the organization ordered by name, optionally filtered by `status`, `record_type` and `search` (name or key); each item carries `stats` (last run, runs and failures in the last 30 days, runs waiting)", body = Page<AutomationListItemResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `automations.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid filter or pagination (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn list_automations(
    State(state): State<AppState>,
    tenant: AuthContext,
    pagination: Pagination,
    query: Result<Query<ListAutomationsParams>, QueryRejection>,
) -> Result<Json<Page<AutomationListItemResponse>>, ApiError> {
    let params = query_params(query)?;
    let filter = AutomationListFilter {
        status: params.status.map(Into::into),
        record_type: params
            .record_type
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty()),
        search: params
            .search
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty()),
    };
    let listing = service::list(
        &Ctx::from_state(&state, &tenant.scope).await?,
        filter,
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
    path = "/automations",
    tag = TAG,
    operation_id = "automations_create",
    security(("session_cookie" = [])),
    description = "Creates a draft automation over a record type (`contact`, `company`, `deal`, `activity` or `custom.<key>`). `trigger` has a `type` from the catalog plus its options; `definition` is the step graph: `start` names the first step and each step is a `condition` (filter AST of the listings, with `trigger.*` paths evaluated over the event: `trigger.previous.<field>`, `trigger.changed_fields`, `trigger.payload.*`; when false the run ends), a `branch` (first matching `cases[]` or `default`) or an `action` (catalog `action` + `config`, whose strings accept templates such as `{{record.title}}`, `{{record.custom.cpf}}`, `{{trigger.previous.stage_id}}`, `{{now + 2d}}` and `{{today + 1w}}`). The graph must be acyclic, without orphan steps, and fields, templates, tags, stages and users are validated now (paths under `trigger.` and `definition.`). Write-only secrets of actions (such as `signing_secret` of `call_webhook`) go to the secret vault and come back only as `<field>_configured`. `key` defaults to a slug of the name.",
    request_body = CreateAutomationRequest,
    responses(
        (status = 201, description = "Draft created (`automation.created`)", body = Data<AutomationResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `automations.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 409, description = "Key already used (`automation_key_taken`) or limit of automations reached (`automation_limit_reached`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid name, trigger or definition (`invalid_automation`, `validation_error`)", body = ErrorEnvelope),
    )
)]
async fn create_automation(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    JsonBody(request): JsonBody<CreateAutomationRequest>,
) -> Result<(StatusCode, Json<Data<AutomationResponse>>), ApiError> {
    let automation = service::create(
        &Ctx::from_state(&state, &tenant.scope).await?,
        request.into_command(client.0.ip),
    )
    .await?;
    Ok((StatusCode::CREATED, Json(Data::new(automation.into()))))
}

#[utoipa::path(
    get,
    path = "/automations/catalog",
    tag = TAG,
    operation_id = "automations_catalog",
    security(("session_cookie" = [])),
    responses(
        (status = 200, description = "Triggers and actions available for automations, with the record types each one supports, the JSON schema of their options, the template roots, the `trigger.*` paths and the loop protection limits", body = Data<AutomationCatalogResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `automations.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
    )
)]
async fn automation_catalog(
    State(state): State<AppState>,
    tenant: AuthContext,
) -> Result<Json<Data<AutomationCatalogResponse>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let entities = ctx
        .registry()
        .definitions()
        .filter(|entity| tenant.scope.filter_for(&entity.read_permission).is_some())
        .map(|entity| entity.key.clone())
        .collect::<Vec<_>>();
    let (triggers, actions) = service::catalog(&ctx)?;
    let triggers = triggers
        .all()
        .iter()
        .map(|trigger| TriggerCatalogEntry::from_trigger(trigger.as_ref(), &entities))
        .collect();
    let actions = actions
        .all()
        .iter()
        .map(|action| ActionCatalogEntry::from_action(action.as_ref(), &entities))
        .collect();
    Ok(Json(Data::new(AutomationCatalogResponse::new(
        triggers, actions,
    ))))
}

#[utoipa::path(
    get,
    path = "/automations/{automation_id}",
    tag = TAG,
    operation_id = "automations_get",
    security(("session_cookie" = [])),
    params(("automation_id" = String, Path, description = "Automation id")),
    responses(
        (status = 200, description = "The automation with its draft trigger and definition; `version` is the published version that runs (0 = never published)", body = Data<AutomationResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `automations.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown automation (`automation_not_found`)", body = ErrorEnvelope),
    )
)]
async fn get_automation(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(automation_id): Path<String>,
) -> Result<Json<Data<AutomationResponse>>, ApiError> {
    let automation = service::get(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &automation_id,
    )
    .await?;
    Ok(Json(Data::new(automation.into())))
}

#[utoipa::path(
    patch,
    path = "/automations/{automation_id}",
    tag = TAG,
    operation_id = "automations_update",
    security(("session_cookie" = [])),
    params(
        ("automation_id" = String, Path, description = "Automation id"),
        ("If-Match" = Option<String>, Header, description = "`updated_at` of the automation as last read; when it changed since, answers 409 `version_conflict`"),
    ),
    request_body = UpdateAutomationRequest,
    responses(
        (status = 200, description = "Draft updated (`automation.updated`); omitted properties stay unchanged. The record type cannot change. The published version keeps running until the next publish", body = Data<AutomationResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `automations.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown automation (`automation_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The automation changed since `If-Match` (`version_conflict`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid name, trigger or definition (`invalid_automation`)", body = ErrorEnvelope),
    )
)]
async fn update_automation(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(automation_id): Path<String>,
    IfMatch(expected_updated_at): IfMatch,
    JsonBody(request): JsonBody<UpdateAutomationRequest>,
) -> Result<Json<Data<AutomationResponse>>, ApiError> {
    let automation = service::update(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &automation_id,
        request.into_command(expected_updated_at, client.0.ip),
    )
    .await?;
    Ok(Json(Data::new(automation.into())))
}

#[utoipa::path(
    delete,
    path = "/automations/{automation_id}",
    tag = TAG,
    operation_id = "automations_delete",
    security(("session_cookie" = [])),
    params(("automation_id" = String, Path, description = "Automation id")),
    responses(
        (status = 204, description = "Automation deleted with its versions, runs and secrets (`automation.deleted`)"),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `automations.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown automation (`automation_not_found`)", body = ErrorEnvelope),
    )
)]
async fn delete_automation(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(automation_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    service::delete(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &automation_id,
        client.0.ip,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/automations/{automation_id}/publish",
    tag = TAG,
    operation_id = "automations_publish",
    security(("session_cookie" = [])),
    description = "Validates the draft again (it needs at least one action) and checks that the caller has every permission its actions require; then snapshots it as the next version and activates the automation. Runs use the published version and act with the permissions of the user who published it, reloaded at each run.",
    params(
        ("automation_id" = String, Path, description = "Automation id"),
        ("If-Match" = Option<String>, Header, description = "`updated_at` of the automation as last read"),
    ),
    responses(
        (status = 200, description = "Version published and automation active (`automation.published`)", body = Data<AutomationResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `automations.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown automation (`automation_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The automation changed since `If-Match` (`version_conflict`)", body = ErrorEnvelope),
        (status = 422, description = "The draft is invalid or uses actions the caller cannot perform (`invalid_automation`)", body = ErrorEnvelope),
    )
)]
async fn publish_automation(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(automation_id): Path<String>,
    IfMatch(expected_updated_at): IfMatch,
) -> Result<Json<Data<AutomationResponse>>, ApiError> {
    let automation = service::publish(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &automation_id,
        expected_updated_at,
        client.0.ip,
    )
    .await?;
    Ok(Json(Data::new(automation.into())))
}

#[utoipa::path(
    post,
    path = "/automations/{automation_id}/pause",
    tag = TAG,
    operation_id = "automations_pause",
    security(("session_cookie" = [])),
    params(("automation_id" = String, Path, description = "Automation id")),
    responses(
        (status = 200, description = "Automation paused (`automation.paused`); runs already queued end as `skipped` with `automation_inactive`", body = Data<AutomationResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `automations.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown automation (`automation_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The automation was never published (`automation_not_published`)", body = ErrorEnvelope),
    )
)]
async fn pause_automation(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(automation_id): Path<String>,
) -> Result<Json<Data<AutomationResponse>>, ApiError> {
    let automation = service::set_active(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &automation_id,
        false,
        client.0.ip,
    )
    .await?;
    Ok(Json(Data::new(automation.into())))
}

#[utoipa::path(
    post,
    path = "/automations/{automation_id}/activate",
    tag = TAG,
    operation_id = "automations_activate",
    security(("session_cookie" = [])),
    params(("automation_id" = String, Path, description = "Automation id")),
    responses(
        (status = 200, description = "Automation active again with its last published version (`automation.activated`)", body = Data<AutomationResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `automations.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown automation (`automation_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The automation was never published (`automation_not_published`)", body = ErrorEnvelope),
    )
)]
async fn activate_automation(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(automation_id): Path<String>,
) -> Result<Json<Data<AutomationResponse>>, ApiError> {
    let automation = service::set_active(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &automation_id,
        true,
        client.0.ip,
    )
    .await?;
    Ok(Json(Data::new(automation.into())))
}

#[utoipa::path(
    post,
    path = "/automations/{automation_id}/dry-run",
    tag = TAG,
    operation_id = "automations_dry_run",
    security(("session_cookie" = [])),
    description = "Tests the published version (or the draft with `use_draft`, or the draft when nothing was published) against an existing record with the permissions of the caller: conditions and branches are evaluated for real and each action reports what it would do (`status: planned`, rendered input with sensitive values masked and details such as the resolved assignee and due date), without changing anything. `changed_fields` and `previous` simulate the event for `trigger.*` conditions.",
    params(("automation_id" = String, Path, description = "Automation id")),
    request_body = DryRunRequest,
    responses(
        (status = 200, description = "Steps that would run", body = Data<DryRunResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `automations.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown automation (`automation_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid record id (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn dry_run_automation(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(automation_id): Path<String>,
    JsonBody(request): JsonBody<DryRunRequest>,
) -> Result<Json<Data<DryRunResponse>>, ApiError> {
    let result = service::dry_run(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &automation_id,
        request.into_command(),
    )
    .await?;
    Ok(Json(Data::new(result.into())))
}

#[utoipa::path(
    post,
    path = "/automations/{automation_id}/run",
    tag = TAG,
    operation_id = "automations_run",
    security(("session_cookie" = [])),
    description = "Runs an active automation with the `manual` trigger on a record readable by the caller. The run is queued for the worker (`automation.run`) and acts with the permissions of the publisher of the active version.",
    params(("automation_id" = String, Path, description = "Automation id")),
    request_body = RunAutomationRequest,
    responses(
        (status = 202, description = "Run queued", body = Data<AutomationRunResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `automations.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown automation (`automation_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "Not published (`automation_not_published`), paused (`automation_not_active`) or without the manual trigger (`automation_not_manual`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid or unknown record (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn run_automation(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(automation_id): Path<String>,
    JsonBody(request): JsonBody<RunAutomationRequest>,
) -> Result<(StatusCode, Json<Data<AutomationRunResponse>>), ApiError> {
    let run = service::run_now(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &automation_id,
        &request.record_id,
    )
    .await?;
    Ok((StatusCode::ACCEPTED, Json(Data::new(run.into()))))
}

#[utoipa::path(
    get,
    path = "/automations/{automation_id}/versions",
    tag = TAG,
    operation_id = "automations_versions",
    security(("session_cookie" = [])),
    params(("automation_id" = String, Path, description = "Automation id")),
    responses(
        (status = 200, description = "Published versions, newest first", body = Data<Vec<AutomationVersionResponse>>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `automations.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown automation (`automation_not_found`)", body = ErrorEnvelope),
    )
)]
async fn list_versions(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(automation_id): Path<String>,
) -> Result<Json<Data<Vec<AutomationVersionResponse>>>, ApiError> {
    let versions = service::versions(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &automation_id,
    )
    .await?;
    Ok(Json(Data::new(
        versions.into_iter().map(Into::into).collect(),
    )))
}

#[utoipa::path(
    get,
    path = "/automations/{automation_id}/runs",
    tag = TAG,
    operation_id = "automations_runs",
    security(("session_cookie" = [])),
    params(("automation_id" = String, Path, description = "Automation id"), ListRunsParams, PaginationQuery),
    responses(
        (status = 200, description = "Runs of the automation, newest first, optionally filtered by `status` (`waiting` runs carry `resume_at` and `waiting_step_id`; `cancelled` runs carry the `reason`: `record_deleted`, `automation_paused` or `version_published`) and `record_id`", body = Page<AutomationRunResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `automations.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown automation (`automation_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid filter or pagination (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn list_runs(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(automation_id): Path<String>,
    pagination: Pagination,
    query: Result<Query<ListRunsParams>, QueryRejection>,
) -> Result<Json<Page<AutomationRunResponse>>, ApiError> {
    let params = query_params(query)?;
    let status = params.status.map(Into::into);
    let record_id = match params
        .record_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some(value) => {
            Some(Uuid::parse_str(value).map_err(|_| invalid_param("record_id", "must be a UUID"))?)
        }
        None => None,
    };
    let listing = service::runs(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &automation_id,
        RunListFilter { status, record_id },
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
    path = "/automations/{automation_id}/runs/{run_id}",
    tag = TAG,
    operation_id = "automations_run_details",
    security(("session_cookie" = [])),
    params(
        ("automation_id" = String, Path, description = "Automation id"),
        ("run_id" = String, Path, description = "Run id"),
    ),
    responses(
        (status = 200, description = "The run with its steps: evaluated conditions (`condition_result`, chosen `next_step_id`), actions with their rendered input (values of sensitive fields masked, secrets never shown), output, error and duration", body = Data<AutomationRunDetailsResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `automations.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown automation or run (`automation_not_found`, `automation_run_not_found`)", body = ErrorEnvelope),
    )
)]
async fn get_run(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path((automation_id, run_id)): Path<(String, String)>,
) -> Result<Json<Data<AutomationRunDetailsResponse>>, ApiError> {
    let details = service::run_details(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &automation_id,
        &run_id,
    )
    .await?;
    Ok(Json(Data::new(details.into())))
}
