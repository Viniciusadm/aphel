use axum::Json;
use axum::extract::{Query, State};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::api::dto::common::{Data, ErrorEnvelope};
use crate::api::dto::formulas::{
    FormulaCatalogParams, FormulaCatalogResponse, FormulaPreviewRequest, FormulaPreviewResponse,
};
use crate::api::error::ApiError;
use crate::api::extractors::{AuthContext, JsonBody};
use crate::api::state::AppState;
use crate::application::fields::formulas::FormulaUseCases;
use crate::errors::AppError;
use crate::infrastructure::fields::formulas::MySqlFormulaPorts;

pub const TAG: &str = "fields";

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(preview_formula))
        .routes(routes!(formula_catalog))
}

async fn formula_use_cases(
    state: &AppState,
) -> Result<FormulaUseCases<MySqlFormulaPorts>, AppError> {
    Ok(FormulaUseCases::new(MySqlFormulaPorts::new(
        state.db()?,
        state.registry().await?,
    )))
}

#[utoipa::path(
    post,
    path = "/fields/formula/preview",
    tag = TAG,
    operation_id = "fields_formula_preview",
    security(("session_cookie" = [])),
    request_body = FormulaPreviewRequest,
    responses(
        (status = 200, description = "Syntax and type errors with their character positions, detected dependencies and, with `record_id`, the value computed over that record under the caller's read scope", body = Data<FormulaPreviewResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` or the read permission of the record type (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown record type (`record_type_not_found`) or record not visible (`record_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid result type or record id (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn preview_formula(
    State(state): State<AppState>,
    tenant: AuthContext,
    JsonBody(request): JsonBody<FormulaPreviewRequest>,
) -> Result<Json<Data<FormulaPreviewResponse>>, ApiError> {
    let preview = formula_use_cases(&state)
        .await?
        .preview(&tenant.scope, request.into())
        .await?;
    Ok(Json(Data::new(preview.into())))
}

#[utoipa::path(
    get,
    path = "/fields/formula/catalog",
    tag = TAG,
    operation_id = "fields_formula_catalog",
    security(("session_cookie" = [])),
    params(FormulaCatalogParams),
    responses(
        (status = 200, description = "Fields, aggregatable relations and functions a formula of the record type may use", body = Data<FormulaCatalogResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown record type (`record_type_not_found`)", body = ErrorEnvelope),
    )
)]
async fn formula_catalog(
    State(state): State<AppState>,
    tenant: AuthContext,
    Query(params): Query<FormulaCatalogParams>,
) -> Result<Json<Data<FormulaCatalogResponse>>, ApiError> {
    let catalog = formula_use_cases(&state)
        .await?
        .catalog(&tenant.scope, &params.record_type)
        .await?;
    Ok(Json(Data::new(catalog.into())))
}
