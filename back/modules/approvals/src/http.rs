use axum::Json;
use axum::extract::rejection::QueryRejection;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::domain::{
    ApprovalDecision, ApprovalMoment, ApprovalPolicy, ApprovalRule, ApprovalStatus, ApproverKind,
    ConditionKind, DecideCommand, PolicyCommand, PolicyListFilter, PolicyStep, RequestListQuery,
    RequestView, StepInput, SubmissionView, SubmitApprovalCommand,
};
use crate::service;
use aphel_kernel::Ctx;
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::api::dto::common::{Data, ErrorEnvelope, Page, double_option, timestamp};
use aphel_sdk::platform::api::error::ApiError;
use aphel_sdk::platform::api::extractors::{
    AuthContext, ClientInfo, IfMatch, JsonBody, Pagination, PaginationQuery,
};
use aphel_sdk::platform::api::routes::v1::members::window;
use aphel_sdk::platform::api::state::AppState;

pub(crate) const TAG: &str = "approvals";

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_approval_policies, create_approval_policy))
        .routes(routes!(order_approval_policies))
        .routes(routes!(
            get_approval_policy,
            update_approval_policy,
            delete_approval_policy
        ))
        .routes(routes!(list_approval_requests, submit_approval_request))
        .routes(routes!(approval_pending_count))
        .routes(routes!(get_approval_request))
        .routes(routes!(approve_approval_request))
        .routes(routes!(reject_approval_request))
        .routes(routes!(cancel_approval_request))
        .routes(routes!(revoke_approval_request))
        .routes(routes!(list_approval_moments))
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct ApprovalMomentResponse {
    pub(crate) moment: ApprovalMomentSchema,
    pub(crate) label: String,
    #[schema(value_type = String, example = "none")]
    pub(crate) target: aphel_sdk::approvals::MomentTarget,
    pub(crate) record_types: Option<Vec<String>>,
}

#[utoipa::path(
    get,
    path = "/approval-moments",
    tag = TAG,
    operation_id = "approval_moments_list",
    security(("session_cookie" = [])),
    description = "Moments at which a policy can require approval: the built-in `stage_change`, `quote_send` and `on_demand` plus moments contributed by modules (such as `publication.activate`). `target` is `none` (no target), `policy` (the policy names the target, such as a stage) or `subject` (each request names its own target, such as a revision). `record_types` limits the moment to those types when present.",
    responses(
        (status = 200, description = "Known approval moments", body = Data<Vec<ApprovalMomentResponse>>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
    )
)]
async fn list_approval_moments(_tenant: AuthContext) -> Json<Data<Vec<ApprovalMomentResponse>>> {
    Json(Data::new(
        crate::moments::all()
            .into_iter()
            .map(|definition| ApprovalMomentResponse {
                moment: definition.moment.into(),
                label: definition.label,
                target: definition.target,
                record_types: definition.record_types,
            })
            .collect(),
    ))
}

#[utoipa::path(
    post,
    path = "/approval-requests/{request_id}/revoke",
    tag = TAG,
    operation_id = "approval_requests_revoke",
    security(("session_cookie" = [])),
    description = "Revokes an approval that was not used yet (`approval.cancelled`), so the action it allowed needs a new request. Requires `approvals.manage`.",
    params(("request_id" = String, Path, description = "Approval request id")),
    request_body = ApprovalDecisionRequest,
    responses(
        (status = 200, description = "Revoked approval request", body = Data<ApprovalRequestResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `approvals.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Approval request not found (`approval_request_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The request is not an unused approval (`approval_not_revocable`)", body = ErrorEnvelope),
    )
)]
async fn revoke_approval_request(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(request_id): Path<String>,
    JsonBody(request): JsonBody<ApprovalDecisionRequest>,
) -> Result<Json<Data<ApprovalRequestResponse>>, ApiError> {
    let view = service::revoke(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &request_id,
        request.into_command(client.0.ip),
    )
    .await?;
    Ok(Json(Data::new(view.into())))
}

#[utoipa::path(
    get,
    path = "/approval-policies",
    tag = TAG,
    operation_id = "approval_policies_list",
    security(("session_cookie" = [])),
    description = "Approval policies of the organization in evaluation order. Needs `approvals.manage`.",
    params(ApprovalPolicyListParams, PaginationQuery),
    responses(
        (status = 200, description = "Approval policies", body = Page<ApprovalPolicyResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `approvals.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
    )
)]
async fn list_approval_policies(
    State(state): State<AppState>,
    tenant: AuthContext,
    Query(params): Query<ApprovalPolicyListParams>,
    pagination: Pagination,
) -> Result<Json<Page<ApprovalPolicyResponse>>, ApiError> {
    let policies = service::list_policies(
        &Ctx::from_state(&state, &tenant.scope).await?,
        params.into_filter(),
    )
    .await?;
    let total = u64::try_from(policies.len()).unwrap_or_default();
    let window = window(pagination);
    let items = policies
        .into_iter()
        .skip(usize::try_from(window.offset).unwrap_or(usize::MAX))
        .take(usize::try_from(window.limit).unwrap_or(usize::MAX))
        .map(Into::into)
        .collect();
    Ok(Json(Page::new(items, total, pagination)))
}

#[utoipa::path(
    post,
    path = "/approval-policies",
    tag = TAG,
    operation_id = "approval_policies_create",
    security(("session_cookie" = [])),
    description = "Creates an approval policy (`approval_policy.created`). The policy applies to records of `record_type` at a `moment`: `stage_change` (before a record with stages moves to the stage `target_id`), `quote_send` (before a quote is sent) or `on_demand` (when someone or an automation asks for it). The condition is `always`, a `filter` validated against the fields of the type (same format as views) or a boolean `formula`; it is evaluated over the stored record with every field visible. Steps run in order (1 to 10): approvers are a `user`, the members of a `role` or the managers of the teams of the record owner (`owner_team_manager`), with rule `any` (one approval closes the step) or `all`. When a step resolves nobody besides the requester, the members with `approvals.manage` approve it; the requester never approves their own request, and when nobody is eligible the request is refused with 409 `approval_no_approver` instead of staying pending forever. While a request is pending, `locked_fields` (empty = whole record) cannot be changed (409 `approval_pending`). Needs `approvals.manage`.",
    request_body = ApprovalPolicyRequest,
    responses(
        (status = 201, description = "Approval policy created", body = Data<ApprovalPolicyResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `approvals.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 409, description = "Another policy already uses the key (`approval_policy_key_taken`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid properties, condition, locked fields or steps (`validation_error` with `details.fields`)", body = ErrorEnvelope),
    )
)]
async fn create_approval_policy(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    JsonBody(request): JsonBody<ApprovalPolicyRequest>,
) -> Result<(StatusCode, Json<Data<ApprovalPolicyResponse>>), ApiError> {
    let policy = service::create_policy(
        &Ctx::from_state(&state, &tenant.scope).await?,
        request.into_command(client.0.ip, None),
    )
    .await?;
    Ok((StatusCode::CREATED, Json(Data::new(policy.into()))))
}

#[utoipa::path(
    put,
    path = "/approval-policies/order",
    tag = TAG,
    operation_id = "approval_policies_order",
    security(("session_cookie" = [])),
    description = "Sets the evaluation order of the approval policies (`approval_policy.reordered`); policies left out keep their relative order after the listed ones. Needs `approvals.manage`.",
    request_body = ApprovalPolicyOrderRequest,
    responses(
        (status = 200, description = "Approval policies in the new order", body = Data<Vec<ApprovalPolicyResponse>>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `approvals.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 422, description = "Unknown or repeated id (`validation_error` with `details.fields`)", body = ErrorEnvelope),
    )
)]
async fn order_approval_policies(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    JsonBody(request): JsonBody<ApprovalPolicyOrderRequest>,
) -> Result<Json<Data<Vec<ApprovalPolicyResponse>>>, ApiError> {
    let policies = service::reorder_policies(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &request.ids,
        client.0.ip,
    )
    .await?;
    Ok(Json(Data::new(
        policies.into_iter().map(Into::into).collect(),
    )))
}

#[utoipa::path(
    get,
    path = "/approval-policies/{policy_id}",
    tag = TAG,
    operation_id = "approval_policies_get",
    security(("session_cookie" = [])),
    description = "An approval policy with its steps. Needs `approvals.manage`.",
    params(("policy_id" = String, Path, description = "Approval policy id")),
    responses(
        (status = 200, description = "Approval policy", body = Data<ApprovalPolicyResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `approvals.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Approval policy not found (`approval_policy_not_found`)", body = ErrorEnvelope),
    )
)]
async fn get_approval_policy(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(policy_id): Path<String>,
) -> Result<Json<Data<ApprovalPolicyResponse>>, ApiError> {
    let policy =
        service::get_policy(&Ctx::from_state(&state, &tenant.scope).await?, &policy_id).await?;
    Ok(Json(Data::new(policy.into())))
}

#[utoipa::path(
    patch,
    path = "/approval-policies/{policy_id}",
    tag = TAG,
    operation_id = "approval_policies_update",
    security(("session_cookie" = [])),
    description = "Changes an approval policy (`approval_policy.updated`); fields left out keep their value, `steps` replaces every step and `key` cannot change. Requests already open keep the steps and locked fields they were created with. Needs `approvals.manage`.",
    params(
        ("policy_id" = String, Path, description = "Approval policy id"),
        ("If-Match" = Option<String>, Header, description = "`updated_at` of the policy as read (ISO-8601)"),
    ),
    request_body = ApprovalPolicyRequest,
    responses(
        (status = 200, description = "Approval policy saved", body = Data<ApprovalPolicyResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `approvals.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Approval policy not found (`approval_policy_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The policy changed since the `If-Match` version (`version_conflict`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid properties (`validation_error` with `details.fields`)", body = ErrorEnvelope),
    )
)]
async fn update_approval_policy(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    IfMatch(expected): IfMatch,
    Path(policy_id): Path<String>,
    JsonBody(request): JsonBody<ApprovalPolicyRequest>,
) -> Result<Json<Data<ApprovalPolicyResponse>>, ApiError> {
    let policy = service::update_policy(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &policy_id,
        request.into_command(client.0.ip, expected),
    )
    .await?;
    Ok(Json(Data::new(policy.into())))
}

#[utoipa::path(
    delete,
    path = "/approval-policies/{policy_id}",
    tag = TAG,
    operation_id = "approval_policies_delete",
    security(("session_cookie" = [])),
    description = "Deletes an approval policy (`approval_policy.deleted`); its pending requests are cancelled (`approval.cancelled`) and decided requests stay in the history. Needs `approvals.manage`.",
    params(
        ("policy_id" = String, Path, description = "Approval policy id"),
        ("If-Match" = Option<String>, Header, description = "`updated_at` of the policy as read (ISO-8601)"),
    ),
    responses(
        (status = 204, description = "Approval policy deleted"),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `approvals.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Approval policy not found (`approval_policy_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The policy changed since the `If-Match` version (`version_conflict`)", body = ErrorEnvelope),
    )
)]
async fn delete_approval_policy(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    IfMatch(expected): IfMatch,
    Path(policy_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    service::delete_policy(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &policy_id,
        expected,
        client.0.ip,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get,
    path = "/approval-requests",
    tag = TAG,
    operation_id = "approval_requests_list",
    security(("session_cookie" = [])),
    description = "Approval requests, newest first. With `record_type` + `record_id`: every request of the record (needs read access to the record). Otherwise by `box`: `pending_for_me` (default; pending requests waiting for a decision of the current user in the current step), `requested_by_me` or `all` (needs `approvals.manage`). `status` narrows the result.",
    params(ApprovalRequestListParams, PaginationQuery),
    responses(
        (status = 200, description = "Approval requests", body = Page<ApprovalRequestResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "No read access to the record or missing `approvals.manage` for `box=all` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Record not found (`record_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid filters or pagination (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn list_approval_requests(
    State(state): State<AppState>,
    tenant: AuthContext,
    params: Result<Query<ApprovalRequestListParams>, QueryRejection>,
    pagination: Pagination,
) -> Result<Json<Page<ApprovalRequestResponse>>, ApiError> {
    let Query(params) = params.map_err(|rejection| AppError::Validation(rejection.body_text()))?;
    let listing = service::list_requests(
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
    path = "/approval-requests",
    tag = TAG,
    operation_id = "approval_requests_submit",
    security(("session_cookie" = [])),
    description = "Asks for the approvals a record needs at a moment (`on_demand`, `stage_change` with `target_id` = stage, `quote_send`), usually after a 409 `approval_required`. For every active policy of the type and moment whose condition matches the record, opens a request (`approval.requested`, approvers of the first step notified) unless one is already pending or approved and not yet used. `policy_key` restricts the submission to one policy. `outcome` is `not_required` (no policy applies), `pending` or `approved` (every applicable policy already approved: retry the change). Needs edit access to the record.",
    request_body = ApprovalSubmitRequest,
    responses(
        (status = 200, description = "Submission result", body = Data<ApprovalSubmissionResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "No edit access to the record (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Record not found (`record_not_found`, `record_type_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "A step of an applicable policy has no eligible approver besides the requester, not even a member with `approvals.manage` (`approval_no_approver`, nothing is opened; `details` has the policy and step)", body = ErrorEnvelope),
        (status = 422, description = "Invalid record, moment, target or comment (`validation_error` with `details.fields`)", body = ErrorEnvelope),
    )
)]
async fn submit_approval_request(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    JsonBody(request): JsonBody<ApprovalSubmitRequest>,
) -> Result<Json<Data<ApprovalSubmissionResponse>>, ApiError> {
    let (command, policy_key) = request.into_command(client.0.ip);
    let submission = service::submit(
        &Ctx::from_state(&state, &tenant.scope).await?,
        command,
        policy_key.as_deref(),
    )
    .await?;
    Ok(Json(Data::new(submission.into())))
}

#[utoipa::path(
    get,
    path = "/approval-requests/pending-count",
    tag = TAG,
    operation_id = "approval_requests_pending_count",
    security(("session_cookie" = [])),
    description = "Number of pending approval requests waiting for a decision of the current user.",
    responses(
        (status = 200, description = "Pending approvals", body = Data<ApprovalPendingCountResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
    )
)]
async fn approval_pending_count(
    State(state): State<AppState>,
    tenant: AuthContext,
) -> Result<Json<Data<ApprovalPendingCountResponse>>, ApiError> {
    let count = service::pending_count(&Ctx::from_state(&state, &tenant.scope).await?).await?;
    Ok(Json(Data::new(ApprovalPendingCountResponse { count })))
}

#[utoipa::path(
    get,
    path = "/approval-requests/{request_id}",
    tag = TAG,
    operation_id = "approval_requests_get",
    security(("session_cookie" = [])),
    description = "An approval request with its steps, approvers and decisions. Visible to the requester, the approvers, who reads the record and who has `approvals.manage`.",
    params(("request_id" = String, Path, description = "Approval request id")),
    responses(
        (status = 200, description = "Approval request", body = Data<ApprovalRequestResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 404, description = "Approval request not found (`approval_request_not_found`)", body = ErrorEnvelope),
    )
)]
async fn get_approval_request(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(request_id): Path<String>,
) -> Result<Json<Data<ApprovalRequestResponse>>, ApiError> {
    let view =
        service::get_request(&Ctx::from_state(&state, &tenant.scope).await?, &request_id).await?;
    Ok(Json(Data::new(view.into())))
}

#[utoipa::path(
    post,
    path = "/approval-requests/{request_id}/approve",
    tag = TAG,
    operation_id = "approval_requests_approve",
    security(("session_cookie" = [])),
    description = "Approves the current step as one of its approvers. With rule `any` the step closes; with `all` it closes when every approver approved (`approval.decision_recorded` meanwhile). A closed step opens the next one (`approval.step_advanced`, new approvers notified); the last one approves the request (`approval.approved`, requester notified). The approved request is used by the next matching change of the record.",
    params(("request_id" = String, Path, description = "Approval request id")),
    request_body = ApprovalDecisionRequest,
    responses(
        (status = 200, description = "Approval request after the decision", body = Data<ApprovalRequestResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Not an approver of the current step or already decided (`approval_not_approver`)", body = ErrorEnvelope),
        (status = 404, description = "Approval request not found (`approval_request_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The request is not pending (`approval_not_pending`) or the next step has no eligible approver anymore (`approval_no_approver`; reject or cancel the request)", body = ErrorEnvelope),
        (status = 422, description = "Comment too long (`validation_error` with `details.fields`)", body = ErrorEnvelope),
    )
)]
async fn approve_approval_request(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(request_id): Path<String>,
    JsonBody(request): JsonBody<ApprovalDecisionRequest>,
) -> Result<Json<Data<ApprovalRequestResponse>>, ApiError> {
    let view = service::approve(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &request_id,
        request.into_command(client.0.ip),
    )
    .await?;
    Ok(Json(Data::new(view.into())))
}

#[utoipa::path(
    post,
    path = "/approval-requests/{request_id}/reject",
    tag = TAG,
    operation_id = "approval_requests_reject",
    security(("session_cookie" = [])),
    description = "Rejects the request as an approver of the current step (`approval.rejected`, requester notified). `comment` is required.",
    params(("request_id" = String, Path, description = "Approval request id")),
    request_body = ApprovalDecisionRequest,
    responses(
        (status = 200, description = "Approval request after the decision", body = Data<ApprovalRequestResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Not an approver of the current step or already decided (`approval_not_approver`)", body = ErrorEnvelope),
        (status = 404, description = "Approval request not found (`approval_request_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The request is not pending (`approval_not_pending`)", body = ErrorEnvelope),
        (status = 422, description = "Missing or too long comment (`validation_error` with `details.fields`)", body = ErrorEnvelope),
    )
)]
async fn reject_approval_request(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(request_id): Path<String>,
    JsonBody(request): JsonBody<ApprovalDecisionRequest>,
) -> Result<Json<Data<ApprovalRequestResponse>>, ApiError> {
    let view = service::reject(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &request_id,
        request.into_command(client.0.ip),
    )
    .await?;
    Ok(Json(Data::new(view.into())))
}

#[utoipa::path(
    post,
    path = "/approval-requests/{request_id}/cancel",
    tag = TAG,
    operation_id = "approval_requests_cancel",
    security(("session_cookie" = [])),
    description = "Cancels a pending request (`approval.cancelled`). Allowed to the requester and to `approvals.manage`.",
    params(("request_id" = String, Path, description = "Approval request id")),
    request_body = ApprovalDecisionRequest,
    responses(
        (status = 200, description = "Cancelled approval request", body = Data<ApprovalRequestResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Neither the requester nor `approvals.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Approval request not found (`approval_request_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The request is not pending (`approval_not_pending`)", body = ErrorEnvelope),
    )
)]
async fn cancel_approval_request(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(request_id): Path<String>,
    JsonBody(request): JsonBody<ApprovalDecisionRequest>,
) -> Result<Json<Data<ApprovalRequestResponse>>, ApiError> {
    let view = service::cancel(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &request_id,
        request.into_command(client.0.ip),
    )
    .await?;
    Ok(Json(Data::new(view.into())))
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(transparent)]
#[schema(as = ApprovalMoment, value_type = String, example = "on_demand")]
pub(crate) struct ApprovalMomentSchema(String);

impl From<ApprovalMoment> for ApprovalMomentSchema {
    fn from(value: ApprovalMoment) -> Self {
        Self(value.as_str().to_owned())
    }
}

impl ApprovalMomentSchema {
    fn into_key(self) -> String {
        self.0
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = ApprovalConditionKind)]
pub(crate) enum ApprovalConditionKindSchema {
    Always,
    Filter,
    Formula,
}

impl From<ConditionKind> for ApprovalConditionKindSchema {
    fn from(value: ConditionKind) -> Self {
        match value {
            ConditionKind::Always => Self::Always,
            ConditionKind::Filter => Self::Filter,
            ConditionKind::Formula => Self::Formula,
        }
    }
}

impl From<ApprovalConditionKindSchema> for ConditionKind {
    fn from(value: ApprovalConditionKindSchema) -> Self {
        match value {
            ApprovalConditionKindSchema::Always => Self::Always,
            ApprovalConditionKindSchema::Filter => Self::Filter,
            ApprovalConditionKindSchema::Formula => Self::Formula,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = ApproverKind)]
pub(crate) enum ApproverKindSchema {
    User,
    Role,
    OwnerTeamManager,
}

impl From<ApproverKind> for ApproverKindSchema {
    fn from(value: ApproverKind) -> Self {
        match value {
            ApproverKind::User => Self::User,
            ApproverKind::Role => Self::Role,
            ApproverKind::OwnerTeamManager => Self::OwnerTeamManager,
        }
    }
}

impl From<ApproverKindSchema> for ApproverKind {
    fn from(value: ApproverKindSchema) -> Self {
        match value {
            ApproverKindSchema::User => Self::User,
            ApproverKindSchema::Role => Self::Role,
            ApproverKindSchema::OwnerTeamManager => Self::OwnerTeamManager,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = ApprovalRule)]
pub(crate) enum ApprovalRuleSchema {
    Any,
    All,
}

impl From<ApprovalRule> for ApprovalRuleSchema {
    fn from(value: ApprovalRule) -> Self {
        match value {
            ApprovalRule::Any => Self::Any,
            ApprovalRule::All => Self::All,
        }
    }
}

impl From<ApprovalRuleSchema> for ApprovalRule {
    fn from(value: ApprovalRuleSchema) -> Self {
        match value {
            ApprovalRuleSchema::Any => Self::Any,
            ApprovalRuleSchema::All => Self::All,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = ApprovalStatus)]
pub(crate) enum ApprovalStatusSchema {
    Pending,
    Approved,
    Rejected,
    Cancelled,
}

impl From<ApprovalStatus> for ApprovalStatusSchema {
    fn from(value: ApprovalStatus) -> Self {
        match value {
            ApprovalStatus::Pending => Self::Pending,
            ApprovalStatus::Approved => Self::Approved,
            ApprovalStatus::Rejected => Self::Rejected,
            ApprovalStatus::Cancelled => Self::Cancelled,
        }
    }
}

impl From<ApprovalStatusSchema> for ApprovalStatus {
    fn from(value: ApprovalStatusSchema) -> Self {
        match value {
            ApprovalStatusSchema::Pending => Self::Pending,
            ApprovalStatusSchema::Approved => Self::Approved,
            ApprovalStatusSchema::Rejected => Self::Rejected,
            ApprovalStatusSchema::Cancelled => Self::Cancelled,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = ApprovalStepState)]
pub(crate) enum ApprovalStepStateSchema {
    Waiting,
    Active,
    Approved,
    Rejected,
    Skipped,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct ApprovalPolicyStepResponse {
    pub(crate) id: String,
    pub(crate) position: u32,
    pub(crate) name: Option<String>,
    pub(crate) approver_kind: ApproverKindSchema,
    pub(crate) approver_user_id: Option<String>,
    pub(crate) approver_role_id: Option<String>,
    pub(crate) rule: ApprovalRuleSchema,
}

impl From<PolicyStep> for ApprovalPolicyStepResponse {
    fn from(step: PolicyStep) -> Self {
        Self {
            id: step.id.to_string(),
            position: step.position,
            name: step.name,
            approver_kind: step.approver_kind.into(),
            approver_user_id: step.approver_user_id.map(UserId::into_inner),
            approver_role_id: step.approver_role_id.map(|id| id.to_string()),
            rule: step.rule.into(),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct ApprovalPolicyResponse {
    pub(crate) id: String,
    #[schema(example = "large_discount")]
    pub(crate) key: String,
    #[schema(example = "Discount above 20%")]
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    #[schema(example = "deals.deal")]
    pub(crate) record_type: String,
    pub(crate) moment: ApprovalMomentSchema,
    pub(crate) target_id: Option<String>,
    pub(crate) condition_kind: ApprovalConditionKindSchema,
    #[schema(value_type = Option<Object>)]
    pub(crate) condition_filter: Option<Value>,
    pub(crate) condition_formula: Option<String>,
    pub(crate) locked_fields: Vec<String>,
    pub(crate) due_hours: Option<u32>,
    pub(crate) is_active: bool,
    pub(crate) position: i32,
    pub(crate) steps: Vec<ApprovalPolicyStepResponse>,
    pub(crate) created_by: Option<String>,
    #[schema(example = "2026-10-20T13:45:00Z")]
    pub(crate) created_at: String,
    #[schema(example = "2026-10-20T13:45:00Z")]
    pub(crate) updated_at: String,
}

impl From<ApprovalPolicy> for ApprovalPolicyResponse {
    fn from(policy: ApprovalPolicy) -> Self {
        Self {
            id: policy.id.to_string(),
            key: policy.key,
            name: policy.name,
            description: policy.description,
            record_type: policy.record_type,
            moment: policy.moment.into(),
            target_id: policy.target_id.map(|id| id.to_string()),
            condition_kind: policy.condition.kind().into(),
            condition_filter: policy
                .condition
                .filter()
                .and_then(|filter| serde_json::to_value(filter).ok()),
            condition_formula: policy.condition.formula().map(str::to_owned),
            locked_fields: policy.locked_fields,
            due_hours: policy.due_hours,
            is_active: policy.is_active,
            position: policy.position,
            steps: policy.steps.into_iter().map(Into::into).collect(),
            created_by: policy.created_by.map(UserId::into_inner),
            created_at: timestamp(policy.created_at),
            updated_at: timestamp(policy.updated_at),
        }
    }
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ApprovalPolicyStepRequest {
    pub(crate) name: Option<String>,
    pub(crate) approver_kind: ApproverKindSchema,
    pub(crate) approver_user_id: Option<String>,
    pub(crate) approver_role_id: Option<String>,
    pub(crate) rule: Option<ApprovalRuleSchema>,
}

#[derive(Debug, Default, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ApprovalPolicyRequest {
    #[schema(example = "large_discount")]
    pub(crate) key: Option<String>,
    pub(crate) name: Option<String>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub(crate) description: Option<Option<String>>,
    pub(crate) record_type: Option<String>,
    pub(crate) moment: Option<ApprovalMomentSchema>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub(crate) target_id: Option<Option<String>>,
    pub(crate) condition_kind: Option<ApprovalConditionKindSchema>,
    #[schema(value_type = Option<Object>)]
    pub(crate) condition_filter: Option<Value>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub(crate) condition_formula: Option<Option<String>>,
    pub(crate) locked_fields: Option<Vec<String>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<i64>, minimum = 1, maximum = 2160)]
    pub(crate) due_hours: Option<Option<i64>>,
    pub(crate) is_active: Option<bool>,
    pub(crate) steps: Option<Vec<ApprovalPolicyStepRequest>>,
}

impl ApprovalPolicyRequest {
    pub(crate) fn into_command(
        self,
        ip: Option<String>,
        expected_updated_at: Option<DateTime<Utc>>,
    ) -> PolicyCommand {
        PolicyCommand {
            key: self.key,
            name: self.name,
            description: self.description,
            record_type: self.record_type,
            moment: self.moment.map(ApprovalMomentSchema::into_key),
            target_id: self.target_id,
            condition_kind: self
                .condition_kind
                .map(|kind| ConditionKind::from(kind).as_str().to_owned()),
            condition_filter: self.condition_filter,
            condition_formula: self.condition_formula,
            locked_fields: self.locked_fields,
            due_hours: self.due_hours,
            is_active: self.is_active,
            steps: self.steps.map(|steps| {
                steps
                    .into_iter()
                    .map(|step| StepInput {
                        name: step.name,
                        approver_kind: ApproverKind::from(step.approver_kind).as_str().to_owned(),
                        approver_user_id: step.approver_user_id,
                        approver_role_id: step.approver_role_id,
                        rule: step
                            .rule
                            .map(|rule| ApprovalRule::from(rule).as_str().to_owned()),
                    })
                    .collect()
            }),
            ip,
            expected_updated_at,
        }
    }
}

#[derive(Debug, Default, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ApprovalPolicyOrderRequest {
    pub(crate) ids: Vec<String>,
}

#[derive(Debug, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct ApprovalPolicyListParams {
    #[param(example = "deals.deal")]
    pub(crate) record_type: Option<String>,
    pub(crate) active: Option<bool>,
}

impl ApprovalPolicyListParams {
    pub(crate) fn into_filter(self) -> PolicyListFilter {
        PolicyListFilter {
            record_type: self
                .record_type
                .map(|value| value.trim().to_owned())
                .filter(|value| !value.is_empty()),
            active: self.active,
        }
    }
}

#[derive(Debug, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct ApprovalRequestListParams {
    pub(crate) record_type: Option<String>,
    pub(crate) record_id: Option<String>,
    #[serde(rename = "box")]
    #[param(example = "pending_for_me")]
    pub(crate) box_name: Option<String>,
    pub(crate) status: Option<ApprovalStatusSchema>,
}

impl ApprovalRequestListParams {
    pub(crate) fn into_query(self) -> RequestListQuery {
        RequestListQuery {
            record_type: self.record_type,
            record_id: self.record_id,
            box_name: self.box_name,
            status: self.status.map(Into::into),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct ApprovalUserResponse {
    pub(crate) id: String,
    pub(crate) display_name: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct ApprovalApproverResponse {
    pub(crate) user_id: String,
    pub(crate) display_name: String,
    #[schema(value_type = Option<String>, example = "approved")]
    pub(crate) decision: Option<String>,
    pub(crate) comment: Option<String>,
    pub(crate) decided_at: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct ApprovalStepResponse {
    pub(crate) position: u32,
    pub(crate) name: Option<String>,
    #[schema(value_type = ApproverKindSchema)]
    pub(crate) approver_kind: String,
    #[schema(value_type = ApprovalRuleSchema)]
    pub(crate) rule: String,
    pub(crate) state: ApprovalStepStateSchema,
    pub(crate) approvers: Vec<ApprovalApproverResponse>,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct ApprovalRequestResponse {
    pub(crate) id: String,
    pub(crate) policy_id: Option<String>,
    pub(crate) policy_key: Option<String>,
    pub(crate) policy_name: String,
    pub(crate) record_type: String,
    pub(crate) record_id: String,
    pub(crate) record_title: Option<String>,
    pub(crate) moment: ApprovalMomentSchema,
    pub(crate) target_id: Option<String>,
    pub(crate) target_name: Option<String>,
    pub(crate) status: ApprovalStatusSchema,
    pub(crate) current_step: u32,
    pub(crate) steps: Vec<ApprovalStepResponse>,
    pub(crate) locked_fields: Vec<String>,
    pub(crate) comment: Option<String>,
    pub(crate) requested_by: Option<ApprovalUserResponse>,
    pub(crate) requested_at: String,
    pub(crate) due_at: Option<String>,
    pub(crate) is_overdue: bool,
    pub(crate) resolved_at: Option<String>,
    pub(crate) resolved_by: Option<ApprovalUserResponse>,
    pub(crate) resolution_comment: Option<String>,
    pub(crate) consumed_at: Option<String>,
    pub(crate) can_decide: bool,
    pub(crate) can_cancel: bool,
}

fn user_response(
    names: &std::collections::HashMap<String, String>,
    user: Option<UserId>,
) -> Option<ApprovalUserResponse> {
    user.map(|user| ApprovalUserResponse {
        display_name: names.get(user.as_str()).cloned().unwrap_or_default(),
        id: user.into_inner(),
    })
}

impl From<RequestView> for ApprovalRequestResponse {
    fn from(view: RequestView) -> Self {
        let names = view.names;
        let record = view.record;
        let request = record.request;
        let current = request.current_step;
        let status = request.status;
        let steps = request
            .steps
            .iter()
            .map(|step| {
                let approvers: Vec<ApprovalApproverResponse> = record
                    .approvers
                    .iter()
                    .filter(|approver| approver.step_position == step.position)
                    .map(|approver| ApprovalApproverResponse {
                        user_id: approver.user_id.as_str().to_owned(),
                        display_name: names
                            .get(approver.user_id.as_str())
                            .cloned()
                            .unwrap_or_default(),
                        decision: approver
                            .decision
                            .map(|decision| decision.as_str().to_owned()),
                        comment: approver.comment.clone(),
                        decided_at: approver.decided_at.map(timestamp),
                    })
                    .collect();
                let rejected = approvers.iter().any(|approver| {
                    approver.decision.as_deref() == Some(ApprovalDecision::Rejected.as_str())
                });
                let state = if step.position < current {
                    ApprovalStepStateSchema::Approved
                } else if step.position == current {
                    match status {
                        ApprovalStatus::Pending => ApprovalStepStateSchema::Active,
                        ApprovalStatus::Approved => ApprovalStepStateSchema::Approved,
                        ApprovalStatus::Rejected if rejected => ApprovalStepStateSchema::Rejected,
                        ApprovalStatus::Rejected | ApprovalStatus::Cancelled => {
                            ApprovalStepStateSchema::Skipped
                        }
                    }
                } else if status == ApprovalStatus::Pending {
                    ApprovalStepStateSchema::Waiting
                } else {
                    ApprovalStepStateSchema::Skipped
                };
                ApprovalStepResponse {
                    position: step.position,
                    name: step.name.clone(),
                    approver_kind: step.approver_kind.clone(),
                    rule: step.rule.clone(),
                    state,
                    approvers,
                }
            })
            .collect();
        Self {
            id: request.id.to_string(),
            policy_id: request.policy_id.map(|id| id.to_string()),
            policy_key: request.policy_key,
            policy_name: request.policy_name,
            record_type: request.record.record_type.as_str().to_owned(),
            record_id: request.record.record_id.to_string(),
            record_title: view.record_title,
            moment: request.moment.into(),
            target_id: request.target_id.map(|id| id.to_string()),
            target_name: view.target_name,
            status: status.into(),
            current_step: current,
            steps,
            locked_fields: request.locked_fields,
            comment: request.comment,
            requested_by: user_response(&names, request.requested_by),
            requested_at: timestamp(request.requested_at),
            due_at: request.due_at.map(timestamp),
            is_overdue: view.is_overdue,
            resolved_at: request.resolved_at.map(timestamp),
            resolved_by: user_response(&names, request.resolved_by),
            resolution_comment: request.resolution_comment,
            consumed_at: request.consumed_at.map(timestamp),
            can_decide: view.can_decide,
            can_cancel: view.can_cancel,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct ApprovalSubmissionResponse {
    #[schema(example = "pending")]
    pub(crate) outcome: String,
    pub(crate) requests: Vec<ApprovalRequestResponse>,
}

impl From<SubmissionView> for ApprovalSubmissionResponse {
    fn from(view: SubmissionView) -> Self {
        Self {
            outcome: view.outcome.to_owned(),
            requests: view.requests.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ApprovalSubmitRequest {
    #[schema(example = "deals.deal")]
    pub(crate) record_type: String,
    pub(crate) record_id: String,
    pub(crate) moment: ApprovalMomentSchema,
    pub(crate) target_id: Option<String>,
    pub(crate) comment: Option<String>,
    pub(crate) policy_key: Option<String>,
}

impl ApprovalSubmitRequest {
    pub(crate) fn into_command(
        self,
        ip: Option<String>,
    ) -> (SubmitApprovalCommand, Option<String>) {
        (
            SubmitApprovalCommand {
                record_type: self.record_type,
                record_id: self.record_id,
                moment: self.moment.into_key(),
                target_id: self.target_id,
                comment: self.comment,
                ip,
            },
            self.policy_key
                .map(|key| key.trim().to_owned())
                .filter(|key| !key.is_empty()),
        )
    }
}

#[derive(Debug, Default, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ApprovalDecisionRequest {
    pub(crate) comment: Option<String>,
}

impl ApprovalDecisionRequest {
    pub(crate) fn into_command(self, ip: Option<String>) -> DecideCommand {
        DecideCommand {
            comment: self.comment,
            ip,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct ApprovalPendingCountResponse {
    pub(crate) count: u64,
}
