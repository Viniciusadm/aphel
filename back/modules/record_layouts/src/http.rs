use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::application::LayoutUseCases;
use crate::dto::{FormLayoutResponse, SaveLayoutRequest};
use crate::infrastructure::MySqlLayoutPorts;
use aphel_kernel::api::dto::common::{Data, ErrorEnvelope};
use aphel_kernel::api::error::ApiError;
use aphel_kernel::api::extractors::{AuthContext, ClientInfo, JsonBody};
use aphel_kernel::api::state::AppState;
use aphel_kernel::errors::AppError;

pub const TAG: &str = "layouts";

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(get_layout, save_layout, reset_layout))
}

async fn layout_use_cases(state: &AppState) -> Result<LayoutUseCases<MySqlLayoutPorts>, AppError> {
    Ok(LayoutUseCases::new(MySqlLayoutPorts::new(
        state.db()?,
        state.registry().await?,
        state.events.clone(),
        state.audit.clone(),
    )))
}

#[utoipa::path(
    get,
    path = "/layouts/{record_type}/{context}",
    tag = TAG,
    operation_id = "layouts_get",
    security(("session_cookie" = [])),
    params(
        ("record_type" = String, Path, description = "Record type, such as `contact`"),
        ("context" = String, Path, description = "`create`, `edit` or `view`"),
    ),
    responses(
        (status = 200, description = "Saved layout of the context; `layout` is `null` (and `is_custom` false) when the organization uses the default layout generated from the fields. Any member can read layouts", body = Data<FormLayoutResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown record type (`record_type_not_found`) or context (`layout_context_not_found`)", body = ErrorEnvelope),
    )
)]
async fn get_layout(
    State(state): State<AppState>,
    Path((record_type, context)): Path<(String, String)>,
) -> Result<Json<Data<FormLayoutResponse>>, ApiError> {
    let entry = layout_use_cases(&state)
        .await?
        .get_layout(&record_type, &context)
        .await?;
    Ok(Json(Data::new(entry.into())))
}

#[utoipa::path(
    put,
    path = "/layouts/{record_type}/{context}",
    tag = TAG,
    operation_id = "layouts_save",
    security(("session_cookie" = [])),
    params(
        ("record_type" = String, Path, description = "Record type, such as `contact`"),
        ("context" = String, Path, description = "`create`, `edit` or `view`"),
    ),
    request_body = SaveLayoutRequest,
    responses(
        (status = 200, description = "Layout saved (tabs → sections → columns → fields, with `visible_when`/`required_when` conditions in the filter AST). Required fields missing from a `create` layout are appended to its first column. Replaces the whole layout of the context. Publishes `form_layout.updated`", body = Data<FormLayoutResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown record type (`record_type_not_found`) or context (`layout_context_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid structure, unknown, inactive or duplicated field, hidden required field, `required_when` in a view layout or invalid condition (paths such as `layout.tabs.0.sections.0.columns.0.fields.1.required_when.conditions.0.value`; `validation_error`)", body = ErrorEnvelope),
    )
)]
async fn save_layout(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path((record_type, context)): Path<(String, String)>,
    JsonBody(request): JsonBody<SaveLayoutRequest>,
) -> Result<Json<Data<FormLayoutResponse>>, ApiError> {
    let entry = layout_use_cases(&state)
        .await?
        .save_layout(
            &tenant.scope,
            &record_type,
            &context,
            request.into_command(client.0.ip),
        )
        .await?;
    Ok(Json(Data::new(entry.into())))
}

#[utoipa::path(
    delete,
    path = "/layouts/{record_type}/{context}",
    tag = TAG,
    operation_id = "layouts_reset",
    security(("session_cookie" = [])),
    params(
        ("record_type" = String, Path, description = "Record type, such as `contact`"),
        ("context" = String, Path, description = "`create`, `edit` or `view`"),
    ),
    responses(
        (status = 204, description = "The saved layout was removed and the default layout applies again (no change when none was saved). Publishes `form_layout.reset`"),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown record type (`record_type_not_found`) or context (`layout_context_not_found`)", body = ErrorEnvelope),
    )
)]
async fn reset_layout(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path((record_type, context)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    layout_use_cases(&state)
        .await?
        .reset_layout(&tenant.scope, &record_type, &context, client.0.ip)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
