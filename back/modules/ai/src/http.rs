use std::collections::HashMap;

use axum::Json;
use axum::extract::{Query, State};
use chrono::Utc;
use utoipa_axum::router::OpenApiRouter;

use crate::config::{self, AiSettings, Feature};
use crate::features;
use crate::model::{
    AiCallList, AiCallListMeta, AiComposeRequest, AiComposition, AiFeatureFlags, AiNextActions,
    AiQueryRequest, AiQueryResult, AiRecordRequest, AiStatus, AiSummary,
};
use crate::query::{self, MAX_QUESTION_CHARS, QUERY_INVALID};
use crate::service::{AiSession, organization};
use crate::store::{self, CallFilter};
use crate::{TAG, USE, VIEW_USAGE};
use aphel_kernel::api::dto::common::{Data, ErrorEnvelope};
use aphel_kernel::api::error::ApiError;
use aphel_kernel::api::extractors::{AuthContext, JsonBody};
use aphel_kernel::application::access::Authorizer;
use aphel_kernel::context::Ctx;
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_kernel::state::AppState;

pub(crate) const INVALID_CALLS_QUERY: &str = "ai_invalid_calls_query";
const DEFAULT_PER_PAGE: u32 = 25;
const MAX_PER_PAGE: u32 = 100;
const RECORD_TYPE_MAX_CHARS: usize = 64;

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(utoipa_axum::routes!(status))
        .routes(utoipa_axum::routes!(summary))
        .routes(utoipa_axum::routes!(next_actions))
        .routes(utoipa_axum::routes!(compose))
        .routes(utoipa_axum::routes!(natural_query))
        .routes(utoipa_axum::routes!(calls))
}

#[utoipa::path(
    get,
    path = "/ai/status",
    tag = TAG,
    operation_id = "ai_status",
    security(("session_cookie" = [])),
    responses(
        (status = 200, description = "Whether the AI provider is configured, which provider, URL and model receive the data, which features are on, the daily limit and today's usage. Never returns the API key, only `api_key_configured`", body = Data<AiStatus>),
        (status = 403, description = "Missing `ai.use` and `ai.view_usage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Module disabled (`module_disabled`)", body = ErrorEnvelope),
    )
)]
async fn status(
    State(state): State<AppState>,
    tenant: AuthContext,
) -> Result<Json<Data<AiStatus>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let scope = ctx.scope;
    if scope.scope_for(&VIEW_USAGE).is_none() {
        scope.require(&USE)?;
    }
    let settings = AiSettings::load(&ctx).await?;
    let (configured, api_key_configured) = config::is_configured(&ctx, &settings).await?;
    let organization = organization(&ctx).await?;
    let today = Utc::now()
        .with_timezone(&organization.timezone)
        .date_naive();
    let used_today = store::used_on(ctx.pool(), today).await?;
    Ok(Json(Data {
        data: AiStatus {
            configured,
            api_key_configured,
            provider: settings.provider.clone(),
            base_url: settings.base_url.clone(),
            model: settings.model.clone(),
            daily_limit: settings.daily_limit,
            used_today,
            remaining_today: settings.daily_limit.saturating_sub(used_today),
            user_hourly_limit: settings.user_hourly_limit,
            send_personal_data: settings.send_personal_data,
            features: AiFeatureFlags {
                summary: settings.feature_enabled(Feature::Summary),
                next_actions: settings.feature_enabled(Feature::NextActions),
                compose: settings.feature_enabled(Feature::Compose),
                query: settings.feature_enabled(Feature::Query),
            },
        },
    }))
}

#[utoipa::path(
    post,
    path = "/ai/summary",
    tag = TAG,
    operation_id = "ai_summary",
    security(("session_cookie" = [])),
    request_body = AiRecordRequest,
    responses(
        (status = 200, description = "Summary of a contact, company or deal written by the configured provider from data the user can read (sensitive fields never leave), with the records used as sources", body = Data<AiSummary>),
        (status = 403, description = "Missing `ai.use` or read access (`permission_denied`), or feature turned off (`ai_feature_disabled`)", body = ErrorEnvelope),
        (status = 404, description = "Record not found or not visible (`ai_record_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "Unsupported record type (`ai_unsupported_record_type`) or the model declined (`ai_refused`)", body = ErrorEnvelope),
        (status = 429, description = "Organization daily limit (`ai_limit_reached`), user hourly limit (`ai_user_limit_reached`) or provider rate limit (`ai_provider_rate_limited`)", body = ErrorEnvelope),
        (status = 503, description = "Provider not configured (`ai_not_configured`) or failed (`ai_provider_error`, `ai_provider_auth_failed`, `ai_provider_timeout`, `ai_empty_response`)", body = ErrorEnvelope),
    )
)]
async fn summary(
    State(state): State<AppState>,
    tenant: AuthContext,
    JsonBody(request): JsonBody<AiRecordRequest>,
) -> Result<Json<Data<AiSummary>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let scope = ctx.scope;
    scope.require(&USE)?;
    crate::context::ensure_context_type(&ctx, &request.record_type)?;
    let session = AiSession::open(&ctx, Feature::Summary).await?;
    let data = features::summary(&ctx, &session, &request.record_type, &request.record_id).await?;
    Ok(Json(Data { data }))
}

#[utoipa::path(
    post,
    path = "/ai/next-actions",
    tag = TAG,
    operation_id = "ai_next_actions",
    security(("session_cookie" = [])),
    request_body = AiRecordRequest,
    responses(
        (status = 200, description = "Up to 5 suggested next actions (`title`, `due_in_days`, `reason`). Nothing is created: the client turns a suggestion into a task only when the user asks, through `POST /api/v1/activities`", body = Data<AiNextActions>),
        (status = 403, description = "Missing `ai.use` or read access (`permission_denied`), or feature turned off (`ai_feature_disabled`)", body = ErrorEnvelope),
        (status = 404, description = "Record not found or not visible (`ai_record_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "Unsupported record type (`ai_unsupported_record_type`) or the model declined (`ai_refused`)", body = ErrorEnvelope),
        (status = 429, description = "`ai_limit_reached`, `ai_user_limit_reached` or `ai_provider_rate_limited`", body = ErrorEnvelope),
        (status = 503, description = "`ai_not_configured`, `ai_provider_error`, `ai_provider_auth_failed`, `ai_provider_timeout`, `ai_empty_response` or `ai_invalid_response` (the answer was not the expected JSON)", body = ErrorEnvelope),
    )
)]
async fn next_actions(
    State(state): State<AppState>,
    tenant: AuthContext,
    JsonBody(request): JsonBody<AiRecordRequest>,
) -> Result<Json<Data<AiNextActions>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let scope = ctx.scope;
    scope.require(&USE)?;
    crate::context::ensure_context_type(&ctx, &request.record_type)?;
    let session = AiSession::open(&ctx, Feature::NextActions).await?;
    let data =
        features::next_actions(&ctx, &session, &request.record_type, &request.record_id).await?;
    Ok(Json(Data { data }))
}

#[utoipa::path(
    post,
    path = "/ai/compose",
    tag = TAG,
    operation_id = "ai_compose",
    security(("session_cookie" = [])),
    request_body = AiComposeRequest,
    responses(
        (status = 200, description = "Suggested text for a note, e-mail or message following the instruction (rewrite, summarize, change tone, reply…). With `record_type` + `record_id` the record (without sensitive fields) goes as context. Nothing is saved", body = Data<AiComposition>),
        (status = 403, description = "Missing `ai.use` (`permission_denied`) or feature turned off (`ai_feature_disabled`)", body = ErrorEnvelope),
        (status = 404, description = "Record not found or not visible (`ai_record_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid body (`ai_invalid_instruction` with `details.fields`, `invalid_json`), unsupported record type (`ai_unsupported_record_type`) or the model declined (`ai_refused`)", body = ErrorEnvelope),
        (status = 429, description = "`ai_limit_reached`, `ai_user_limit_reached` or `ai_provider_rate_limited`", body = ErrorEnvelope),
        (status = 503, description = "`ai_not_configured`, `ai_provider_error`, `ai_provider_auth_failed`, `ai_provider_timeout` or `ai_empty_response`", body = ErrorEnvelope),
    )
)]
async fn compose(
    State(state): State<AppState>,
    tenant: AuthContext,
    JsonBody(request): JsonBody<AiComposeRequest>,
) -> Result<Json<Data<AiComposition>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let scope = ctx.scope;
    scope.require(&USE)?;
    features::validate_compose(&request)?;
    if let Some(record_type) = &request.record_type {
        crate::context::ensure_context_type(&ctx, record_type)?;
    }
    let session = AiSession::open(&ctx, Feature::Compose).await?;
    let data = features::compose(&ctx, &session, request).await?;
    Ok(Json(Data { data }))
}

fn validate_query(request: &AiQueryRequest) -> Result<(), AppError> {
    let record_type = request.record_type.trim();
    let valid_type = !record_type.is_empty()
        && record_type.len() <= RECORD_TYPE_MAX_CHARS
        && record_type.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
        });
    let question = request.question.trim();
    let mut violations = Vec::new();
    if !valid_type {
        violations.push(FieldViolation::new(
            "record_type",
            "is not a valid record type",
        ));
    }
    if question.is_empty() {
        violations.push(FieldViolation::new("question", "is required"));
    } else if question.chars().count() > MAX_QUESTION_CHARS {
        violations.push(FieldViolation::new(
            "question",
            format!("must have at most {MAX_QUESTION_CHARS} characters"),
        ));
    }
    if violations.is_empty() {
        Ok(())
    } else {
        Err(AppError::invalid_fields_code(
            QUERY_INVALID,
            "invalid question",
            violations,
        ))
    }
}

#[utoipa::path(
    post,
    path = "/ai/query",
    tag = TAG,
    operation_id = "ai_query",
    security(("session_cookie" = [])),
    request_body = AiQueryRequest,
    responses(
        (status = 200, description = "A filter AST (the same format as `filter` in the record lists) generated from the question, validated against the field registry and executed read-only to count matches, plus a short explanation. The AI never produces SQL and nothing runs besides the validated filter; the client shows the filter and applies it only when the user confirms. The provider receives the question and the filterable non-sensitive field definitions (key, label, type, options), never record data", body = Data<AiQueryResult>),
        (status = 403, description = "Missing `ai.use` or read access to the record type (`permission_denied`), or feature turned off (`ai_feature_disabled`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid body (`ai_query_invalid` with `details.fields`), no usable fields (`ai_query_no_fields`), the question cannot be a filter (`ai_query_unanswerable`, with the AI's reason) or the AI produced an invalid filter twice (`ai_query_invalid`)", body = ErrorEnvelope),
        (status = 429, description = "`ai_limit_reached`, `ai_user_limit_reached` or `ai_provider_rate_limited`", body = ErrorEnvelope),
        (status = 503, description = "`ai_not_configured`, `ai_provider_error`, `ai_provider_auth_failed`, `ai_provider_timeout` or `ai_empty_response`", body = ErrorEnvelope),
    )
)]
async fn natural_query(
    State(state): State<AppState>,
    tenant: AuthContext,
    JsonBody(request): JsonBody<AiQueryRequest>,
) -> Result<Json<Data<AiQueryResult>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let scope = ctx.scope;
    scope.require(&USE)?;
    validate_query(&request)?;
    let session = AiSession::open(&ctx, Feature::Query).await?;
    let generated = query::generate(
        &ctx,
        &session,
        request.record_type.trim(),
        request.question.trim(),
    )
    .await?;
    Ok(Json(Data {
        data: AiQueryResult {
            filter: generated.filter,
            total: generated.total,
            explanation: generated.explanation,
            provider: session.provider_name(),
            model: session.model(),
        },
    }))
}

fn page_param(params: &HashMap<String, String>, key: &str, default: u32) -> Result<u32, AppError> {
    match params.get(key) {
        None => Ok(default),
        Some(value) => value
            .parse::<u32>()
            .ok()
            .filter(|value| *value >= 1)
            .ok_or_else(|| {
                AppError::validation_code(
                    INVALID_CALLS_QUERY,
                    format!("{key} must be a positive integer"),
                )
            }),
    }
}

#[utoipa::path(
    get,
    path = "/ai/calls",
    tag = TAG,
    operation_id = "ai_calls",
    security(("session_cookie" = [])),
    params(
        ("feature" = Option<String>, Query, description = "summary, next_actions, compose, query or plugin"),
        ("status" = Option<String>, Query, description = "success or error"),
        ("page" = Option<u32>, Query, description = "Page (from 1)"),
        ("per_page" = Option<u32>, Query, description = "Items per page (1 to 100, default 25)"),
    ),
    responses(
        (status = 200, description = "Log of AI calls, newest first: who, feature, provider, model, status, tokens, duration and error code. Prompts and answers are never stored. `meta` adds today's usage, the daily limit and totals of the last 30 days", body = AiCallList),
        (status = 403, description = "Missing `ai.view_usage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid feature, status or pagination (`ai_invalid_calls_query`)", body = ErrorEnvelope),
    )
)]
async fn calls(
    State(state): State<AppState>,
    tenant: AuthContext,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<AiCallList>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let scope = ctx.scope;
    scope.require(&VIEW_USAGE)?;
    let feature = match params.get("feature").filter(|value| !value.is_empty()) {
        None => None,
        Some(value) => Some(
            Feature::parse(value)
                .ok_or_else(|| {
                    AppError::validation_code(
                        INVALID_CALLS_QUERY,
                        "feature must be summary, next_actions, compose, query or plugin",
                    )
                })?
                .as_str()
                .to_owned(),
        ),
    };
    let status = match params.get("status").filter(|value| !value.is_empty()) {
        None => None,
        Some(value) if value == "success" || value == "error" => Some(value.clone()),
        Some(_) => {
            return Err(AppError::validation_code(
                INVALID_CALLS_QUERY,
                "status must be success or error",
            )
            .into());
        }
    };
    let page = page_param(&params, "page", 1)?;
    let per_page = page_param(&params, "per_page", DEFAULT_PER_PAGE)?.min(MAX_PER_PAGE);
    let pool = ctx.pool();
    let (data, total) = store::list_calls(
        pool,
        &CallFilter { feature, status },
        u64::from(page - 1) * u64::from(per_page),
        u64::from(per_page),
    )
    .await?;
    let settings = AiSettings::load(&ctx).await?;
    let organization = organization(&ctx).await?;
    let today = Utc::now()
        .with_timezone(&organization.timezone)
        .date_naive();
    let used_today = store::used_on(pool, today).await?;
    let last_30_days = store::totals_since(pool, store::thirty_days_ago()).await?;
    Ok(Json(AiCallList {
        data,
        meta: AiCallListMeta {
            total,
            page,
            per_page,
            used_today,
            daily_limit: settings.daily_limit,
            last_30_days,
        },
    }))
}
