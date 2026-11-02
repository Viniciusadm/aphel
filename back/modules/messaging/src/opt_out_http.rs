use axum::Json;
use axum::extract::rejection::QueryRejection;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
use uuid::Uuid;

use crate::opt_out_store as store;
use crate::opt_outs::{
    MAX_REASON_LENGTH, OPT_OUT_NOT_FOUND, OPT_OUT_REMOVED_EVENT, OPTED_OUT_EVENT, OptOut,
    OptOutScope, normalize_address,
};
use aphel_kernel::api::dto::common::{Data, ErrorEnvelope, Page, timestamp};
use aphel_kernel::api::error::ApiError;
use aphel_kernel::api::extractors::{
    AuthContext, ClientInfo, JsonBody, Pagination, PaginationQuery,
};
use aphel_kernel::api::state::AppState;
use aphel_kernel::application::access::Authorizer;
use aphel_kernel::application::organization_admin::ListWindow;
use aphel_kernel::application::shared::{Actor, AuditEntry, DomainEvent, FieldChange};
use aphel_kernel::domain::access::Permission;
use aphel_kernel::errors::{AppError, FieldViolation};

pub const TAG: &str = "communication";

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_opt_outs, create_opt_out))
        .routes(routes!(delete_opt_out))
}

#[derive(Debug, Serialize, ToSchema)]
pub struct OptOutResponse {
    id: String,
    address: String,
    #[schema(example = "all")]
    scope: String,
    reason: Option<String>,
    source: String,
    created_by: Option<String>,
    created_at: String,
}

impl From<OptOut> for OptOutResponse {
    fn from(opt_out: OptOut) -> Self {
        Self {
            id: opt_out.id.to_string(),
            address: opt_out.address,
            scope: opt_out.scope.as_str().to_owned(),
            reason: opt_out.reason,
            source: opt_out.source,
            created_by: opt_out.created_by,
            created_at: timestamp(opt_out.created_at),
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct OptOutRequest {
    address: String,
    #[schema(example = "all")]
    scope: String,
    #[serde(default)]
    reason: Option<String>,
}

#[derive(Debug, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct OptOutParams {
    search: Option<String>,
}

fn snapshot(opt_out: &OptOut) -> Value {
    json!({ "address": opt_out.address, "scope": opt_out.scope.as_str(), "reason": opt_out.reason })
}

async fn record_change(
    state: &AppState,
    tenant: &AuthContext,
    ip: Option<String>,
    event_type: &'static str,
    before: Value,
    after: Value,
    mut tx: aphel_kernel::database::transaction::MySqlTx,
) -> Result<(), AppError> {
    let now = Utc::now();
    let actor = Actor::from_scope(&tenant.scope);
    let payload = if after.is_null() {
        before.clone()
    } else {
        after.clone()
    };
    state
        .events
        .publish(
            &mut tx,
            DomainEvent::new(actor.clone(), event_type, now).with_payload(payload),
        )
        .await?;
    state
        .audit
        .record(
            &mut tx,
            AuditEntry::new(actor, event_type, now)
                .with_changes(vec![FieldChange::new("opt_out", before, after)])
                .with_ip(ip),
        )
        .await?;
    tx.commit().await?;
    Ok(())
}

#[utoipa::path(
    get,
    path = "/communication/opt-outs",
    tag = TAG,
    operation_id = "communication_opt_outs_list",
    security(("session_cookie" = [])),
    description = "Addresses that asked not to be contacted (permission `privacy.manage`). Scope `all` blocks every message sent by the platform on behalf of plugins and modules that check consent; `marketing` blocks only marketing messages.",
    params(OptOutParams, PaginationQuery),
    responses(
        (status = 200, description = "Opt-outs, newest first", body = Page<OptOutResponse>),
        (status = 403, description = "Missing `privacy.manage` (`permission_denied`)", body = ErrorEnvelope),
    )
)]
async fn list_opt_outs(
    State(state): State<AppState>,
    tenant: AuthContext,
    pagination: Pagination,
    query: Result<Query<OptOutParams>, QueryRejection>,
) -> Result<Json<Page<OptOutResponse>>, ApiError> {
    tenant.scope.require(&Permission::PRIVACY_MANAGE)?;
    let Query(params) = query.map_err(|rejection| AppError::Validation(rejection.body_text()))?;
    let listing = store::list(
        &state.db()?,
        params.search.as_deref(),
        ListWindow {
            offset: pagination.offset(),
            limit: pagination.limit(),
        },
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
    path = "/communication/opt-outs",
    tag = TAG,
    operation_id = "communication_opt_outs_create",
    security(("session_cookie" = [])),
    request_body = OptOutRequest,
    description = "Records that an address does not want to be contacted (permission `privacy.manage`). Publishes `communication.opted_out` with audit.",
    responses(
        (status = 201, description = "Opt-out recorded", body = Data<OptOutResponse>),
        (status = 403, description = "Missing `privacy.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 409, description = "The address already has an opt-out (`communication_opt_out_exists`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid address, scope or reason (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn create_opt_out(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    JsonBody(request): JsonBody<OptOutRequest>,
) -> Result<(StatusCode, Json<Data<OptOutResponse>>), ApiError> {
    tenant.scope.require(&Permission::PRIVACY_MANAGE)?;
    let address = normalize_address(&request.address)?;
    let mut violations = Vec::new();
    let scope = OptOutScope::parse(request.scope.trim());
    if scope.is_none() {
        violations.push(FieldViolation::new("scope", "must be all or marketing"));
    }
    let reason = request
        .reason
        .map(|reason| reason.trim().to_owned())
        .filter(|reason| !reason.is_empty());
    if reason
        .as_ref()
        .is_some_and(|reason| reason.chars().count() > MAX_REASON_LENGTH)
    {
        violations.push(FieldViolation::new(
            "reason",
            format!("must have at most {MAX_REASON_LENGTH} characters"),
        ));
    }
    let Some(scope) = scope.filter(|_| violations.is_empty()) else {
        return Err(AppError::invalid_fields("invalid opt-out", violations).into());
    };
    let opt_out = OptOut {
        id: Uuid::now_v7(),
        address,
        scope,
        reason,
        source: "manual".to_owned(),
        created_by: Some(tenant.scope.user_id().as_str().to_owned()),
        created_at: Utc::now(),
    };
    let mut tx = aphel_kernel::database::receipt::begin(&state.db()?).await?;
    store::insert(&mut tx, &opt_out).await?;
    record_change(
        &state,
        &tenant,
        client.0.ip,
        OPTED_OUT_EVENT,
        Value::Null,
        snapshot(&opt_out),
        tx,
    )
    .await?;
    Ok((StatusCode::CREATED, Json(Data::new(opt_out.into()))))
}

#[utoipa::path(
    delete,
    path = "/communication/opt-outs/{id}",
    tag = TAG,
    operation_id = "communication_opt_outs_delete",
    security(("session_cookie" = [])),
    params(("id" = String, Path, description = "Opt-out id")),
    description = "Removes an opt-out, for example after the person consents again (permission `privacy.manage`). Publishes `communication.opt_out_removed` with audit.",
    responses(
        (status = 204, description = "Opt-out removed"),
        (status = 403, description = "Missing `privacy.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown opt-out (`communication_opt_out_not_found`)", body = ErrorEnvelope),
    )
)]
async fn delete_opt_out(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    tenant.scope.require(&Permission::PRIVACY_MANAGE)?;
    let not_found = || AppError::not_found_code(OPT_OUT_NOT_FOUND, "the opt-out does not exist");
    let id = Uuid::parse_str(&id).map_err(|_| not_found())?;
    let pool = state.db()?;
    let opt_out = store::find(&pool, id).await?.ok_or_else(not_found)?;
    let mut tx = aphel_kernel::database::receipt::begin(&pool).await?;
    store::delete(&mut tx, id).await?;
    record_change(
        &state,
        &tenant,
        client.0.ip,
        OPT_OUT_REMOVED_EVENT,
        snapshot(&opt_out),
        Value::Null,
        tx,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}
