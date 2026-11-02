use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::application::{RULE_PREVIEW_RESULT_TYPE, ValidationRuleUseCases, rule_preview};
use crate::dto::{
    CreateValidationRuleRequest, ReorderValidationRulesRequest, UpdateValidationRuleRequest,
    ValidationRuleListParams, ValidationRulePreviewRequest, ValidationRulePreviewResponse,
    ValidationRuleResponse,
};
use crate::infrastructure::MySqlValidationRulePorts;
use ::formulas::application::{FormulaUseCases, PreviewFormulaCommand};
use ::formulas::infrastructure::MySqlFormulaPorts;
use aphel_kernel::api::dto::common::{Data, ErrorEnvelope};
use aphel_kernel::api::error::ApiError;
use aphel_kernel::api::extractors::{AuthContext, ClientInfo, IfMatch, JsonBody};
use aphel_kernel::api::state::AppState;
use aphel_kernel::errors::AppError;

pub const TAG: &str = "validation-rules";

const UNAUTHORIZED: &str = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)";
const FORBIDDEN_MANAGE: &str =
    "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)";
const NOT_FOUND: &str = "Rule not found (`validation_rule_not_found`)";

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_validation_rules, create_validation_rule))
        .routes(routes!(reorder_validation_rules))
        .routes(routes!(preview_validation_rule))
        .routes(routes!(
            get_validation_rule,
            update_validation_rule,
            delete_validation_rule
        ))
}

async fn validation_rule_use_cases(
    state: &AppState,
) -> Result<ValidationRuleUseCases<MySqlValidationRulePorts>, AppError> {
    Ok(ValidationRuleUseCases::new(MySqlValidationRulePorts::new(
        state.db()?,
        state.registry().await?,
        state.events.clone(),
        state.audit.clone(),
    )))
}

#[utoipa::path(
    get,
    path = "/validation-rules",
    tag = TAG,
    operation_id = "validation_rules_list",
    security(("session_cookie" = [])),
    description = "Validation rules of the organization, ordered by record type and position. Needs `settings.manage`.",
    params(ValidationRuleListParams),
    responses(
        (status = 200, description = "Validation rules", body = Data<Vec<ValidationRuleResponse>>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = FORBIDDEN_MANAGE, body = ErrorEnvelope),
    )
)]
async fn list_validation_rules(
    State(state): State<AppState>,
    tenant: AuthContext,
    Query(params): Query<ValidationRuleListParams>,
) -> Result<Json<Data<Vec<ValidationRuleResponse>>>, ApiError> {
    let rules = validation_rule_use_cases(&state)
        .await?
        .list(&tenant.scope, params.record_type.as_deref())
        .await?;
    Ok(Json(Data::new(rules.into_iter().map(Into::into).collect())))
}

#[utoipa::path(
    post,
    path = "/validation-rules",
    tag = TAG,
    operation_id = "validation_rules_create",
    security(("session_cookie" = [])),
    description = "Creates a validation rule and publishes `validation_rule.created`. `expression` is a boolean formula (same language as formula fields) describing the **error condition**: when it evaluates to `true` over the proposed state of the record, the write is refused with 422 `validation_rule_failed` and `message` in `details.fields` (path of `field`, `custom_fields.<key>` for custom fields, or `record`). Applies to every write of the record type (forms, inline editing, API, imports, automations, bulk actions) on create, update or both (`applies_on`). `key` is generated from the name when left out. Needs `settings.manage`.",
    request_body = CreateValidationRuleRequest,
    responses(
        (status = 201, description = "Rule created", body = Data<ValidationRuleResponse>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = FORBIDDEN_MANAGE, body = ErrorEnvelope),
        (status = 404, description = "Unknown record type (`record_type_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid name, key, message, field, timing or condition (`validation_error` with `details.fields`)", body = ErrorEnvelope),
    )
)]
async fn create_validation_rule(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    JsonBody(request): JsonBody<CreateValidationRuleRequest>,
) -> Result<(StatusCode, Json<Data<ValidationRuleResponse>>), ApiError> {
    let rule = validation_rule_use_cases(&state)
        .await?
        .create(&tenant.scope, request.into_command(client.0.ip))
        .await?;
    Ok((StatusCode::CREATED, Json(Data::new(rule.into()))))
}

#[utoipa::path(
    put,
    path = "/validation-rules/order",
    tag = TAG,
    operation_id = "validation_rules_reorder",
    security(("session_cookie" = [])),
    description = "Sets the order of every rule of a record type (`validation_rule.reordered`). Needs `settings.manage`.",
    request_body = ReorderValidationRulesRequest,
    responses(
        (status = 200, description = "Rules of the record type in the new order", body = Data<Vec<ValidationRuleResponse>>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = FORBIDDEN_MANAGE, body = ErrorEnvelope),
        (status = 404, description = "Unknown record type (`record_type_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "`ids` must list every rule of the record type once (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn reorder_validation_rules(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    JsonBody(request): JsonBody<ReorderValidationRulesRequest>,
) -> Result<Json<Data<Vec<ValidationRuleResponse>>>, ApiError> {
    let rules = validation_rule_use_cases(&state)
        .await?
        .reorder(&tenant.scope, request.into_command(client.0.ip))
        .await?;
    Ok(Json(Data::new(rules.into_iter().map(Into::into).collect())))
}

#[utoipa::path(
    post,
    path = "/validation-rules/preview",
    tag = TAG,
    operation_id = "validation_rules_preview",
    security(("session_cookie" = [])),
    description = "Checks a rule condition (syntax, types, boolean result) and, with `record_id`, evaluates it over the saved state of that record: `fails = true` means the rule would block a write leaving the record as it is. Needs `settings.manage`.",
    request_body = ValidationRulePreviewRequest,
    responses(
        (status = 200, description = "Errors with character positions, dependencies and, with `record_id`, whether the record breaks the rule", body = Data<ValidationRulePreviewResponse>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = FORBIDDEN_MANAGE, body = ErrorEnvelope),
        (status = 404, description = "Unknown record type (`record_type_not_found`) or record not visible (`record_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid record id (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn preview_validation_rule(
    State(state): State<AppState>,
    tenant: AuthContext,
    JsonBody(request): JsonBody<ValidationRulePreviewRequest>,
) -> Result<Json<Data<ValidationRulePreviewResponse>>, ApiError> {
    let formulas =
        FormulaUseCases::new(MySqlFormulaPorts::new(state.db()?, state.registry().await?));
    let formula = formulas
        .preview(
            &tenant.scope,
            PreviewFormulaCommand {
                record_type: request.record_type,
                expression: request.expression.clone(),
                result_type: RULE_PREVIEW_RESULT_TYPE.to_owned(),
                decimals: None,
                record_id: request.record_id,
                field_key: None,
            },
        )
        .await?;
    Ok(Json(Data::new(
        rule_preview(&request.expression, formula).into(),
    )))
}

#[utoipa::path(
    get,
    path = "/validation-rules/{rule_id}",
    tag = TAG,
    operation_id = "validation_rules_get",
    security(("session_cookie" = [])),
    params(("rule_id" = String, Path, description = "Validation rule id")),
    responses(
        (status = 200, description = "Validation rule", body = Data<ValidationRuleResponse>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = FORBIDDEN_MANAGE, body = ErrorEnvelope),
        (status = 404, description = NOT_FOUND, body = ErrorEnvelope),
    )
)]
async fn get_validation_rule(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(rule_id): Path<String>,
) -> Result<Json<Data<ValidationRuleResponse>>, ApiError> {
    let rule = validation_rule_use_cases(&state)
        .await?
        .get(&tenant.scope, &rule_id)
        .await?;
    Ok(Json(Data::new(rule.into())))
}

#[utoipa::path(
    patch,
    path = "/validation-rules/{rule_id}",
    tag = TAG,
    operation_id = "validation_rules_update",
    security(("session_cookie" = [])),
    description = "Updates a validation rule (`validation_rule.updated`). `is_active = false` turns the rule off without removing it. The record type and the key never change. Fields left out keep their value; `field: null` clears the highlighted field. Needs `settings.manage`.",
    params(
        ("rule_id" = String, Path, description = "Validation rule id"),
        ("If-Match" = Option<String>, Header, description = "`updated_at` of the rule as read (ISO-8601)"),
    ),
    request_body = UpdateValidationRuleRequest,
    responses(
        (status = 200, description = "Rule updated", body = Data<ValidationRuleResponse>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = FORBIDDEN_MANAGE, body = ErrorEnvelope),
        (status = 404, description = NOT_FOUND, body = ErrorEnvelope),
        (status = 409, description = "The rule changed since the `If-Match` version (`version_conflict`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid name, message, field, timing or condition (`validation_error` with `details.fields`)", body = ErrorEnvelope),
    )
)]
async fn update_validation_rule(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    IfMatch(expected): IfMatch,
    Path(rule_id): Path<String>,
    JsonBody(request): JsonBody<UpdateValidationRuleRequest>,
) -> Result<Json<Data<ValidationRuleResponse>>, ApiError> {
    let rule = validation_rule_use_cases(&state)
        .await?
        .update(
            &tenant.scope,
            &rule_id,
            request.into_command(client.0.ip, expected),
        )
        .await?;
    Ok(Json(Data::new(rule.into())))
}

#[utoipa::path(
    delete,
    path = "/validation-rules/{rule_id}",
    tag = TAG,
    operation_id = "validation_rules_delete",
    security(("session_cookie" = [])),
    params(("rule_id" = String, Path, description = "Validation rule id")),
    responses(
        (status = 204, description = "Rule removed (`validation_rule.deleted`)"),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = FORBIDDEN_MANAGE, body = ErrorEnvelope),
        (status = 404, description = NOT_FOUND, body = ErrorEnvelope),
    )
)]
async fn delete_validation_rule(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(rule_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    validation_rule_use_cases(&state)
        .await?
        .delete(&tenant.scope, &rule_id, client.0.ip)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
