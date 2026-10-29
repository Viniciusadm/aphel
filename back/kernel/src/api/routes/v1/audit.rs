use axum::Json;
use axum::extract::rejection::QueryRejection;
use axum::extract::{Query, State};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::api::dto::audit::{AuditLogQueryParams, AuditLogResponse};
use crate::api::dto::common::{ErrorEnvelope, Page};
use crate::api::error::ApiError;
use crate::api::extractors::{AuthContext, Pagination, PaginationQuery};
use crate::api::routes::v1::field_permissions::field_policies;
use crate::api::state::AppState;
use crate::application::audit::AuditUseCases;
use crate::application::organization_admin::ListWindow;
use crate::application::shared::RecordType;
use crate::errors::AppError;
use crate::infrastructure::audit::MySqlAuditRepository;

pub const TAG: &str = "audit";

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(list_audit_logs))
}

fn audit_use_cases(state: &AppState) -> Result<AuditUseCases<MySqlAuditRepository>, AppError> {
    Ok(AuditUseCases::new(MySqlAuditRepository::new(state.db()?)))
}

#[utoipa::path(
    get,
    path = "/audit-logs",
    tag = TAG,
    operation_id = "audit_logs_list",
    security(("session_cookie" = [])),
    params(AuditLogQueryParams, PaginationQuery),
    responses(
        (status = 200, description = "Audit log entries of the organization, most recent first. `caused_by` is set when the change was made by an automation run (automation, run, depth and the chain of automations that led to it)", body = Page<AuditLogResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `audit.read` (`permission_denied`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid filters or pagination", body = ErrorEnvelope),
    )
)]
async fn list_audit_logs(
    State(state): State<AppState>,
    tenant: AuthContext,
    pagination: Pagination,
    query: Result<Query<AuditLogQueryParams>, QueryRejection>,
) -> Result<Json<Page<AuditLogResponse>>, ApiError> {
    let Query(params) = query.map_err(|rejection| AppError::Validation(rejection.body_text()))?;
    let listing = audit_use_cases(&state)?
        .list(&tenant.scope, params.into(), window(pagination))
        .await?;
    let mut items = listing.items;
    let record_types: Vec<RecordType> = items
        .iter()
        .filter_map(|item| {
            item.record
                .as_ref()
                .map(|record| record.record_type.clone())
        })
        .collect();
    let policies = field_policies(&state, &tenant.scope, record_types).await?;
    for item in &mut items {
        if let Some(policy) = item
            .record
            .as_ref()
            .and_then(|record| policies.get(&record.record_type))
        {
            policy.mask_changes(&mut item.changes);
        }
    }
    Ok(Json(Page::new(
        items.into_iter().map(Into::into).collect(),
        listing.total,
        pagination,
    )))
}

fn window(pagination: Pagination) -> ListWindow {
    ListWindow {
        offset: pagination.offset(),
        limit: pagination.limit(),
    }
}
