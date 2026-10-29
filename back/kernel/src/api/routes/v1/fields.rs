use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::api::dto::common::{Data, ErrorEnvelope, Page};
use crate::api::dto::fields::{
    CreateFieldRequest, FieldListParams, FieldOptionRequest, FieldResponse, FieldSchemaResponse,
    ReorderRequest, UpdateFieldRequest, UpdateOptionRequest,
};
use crate::api::error::ApiError;
use crate::api::extractors::{
    AuthContext, ClientInfo, IfMatch, JsonBody, Pagination, PaginationQuery,
};
use crate::api::routes::v1::members::window;
use crate::api::state::AppState;
use crate::application::custom_objects::CUSTOM_OBJECT_FIELDS_MAX;
use crate::application::fields::FieldUseCases;
use crate::errors::AppError;
use crate::infrastructure::fields::MySqlFieldPorts;
use crate::records::filter_schema;

pub const TAG: &str = "fields";

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_fields, create_field))
        .routes(routes!(field_schema))
        .routes(routes!(reorder_fields))
        .routes(routes!(update_field))
        .routes(routes!(deactivate_field))
        .routes(routes!(activate_field))
        .routes(routes!(add_option))
        .routes(routes!(reorder_options))
        .routes(routes!(update_option))
}

pub async fn field_use_cases(state: &AppState) -> Result<FieldUseCases<MySqlFieldPorts>, AppError> {
    Ok(FieldUseCases::new(MySqlFieldPorts::new(
        state.db()?,
        state.registry().await?,
        state.events.clone(),
        state.audit.clone(),
    ))
    .with_custom_object_field_limit(CUSTOM_OBJECT_FIELDS_MAX))
}

#[utoipa::path(
    get,
    path = "/fields/{record_type}",
    tag = TAG,
    operation_id = "fields_list",
    security(("session_cookie" = [])),
    params(
        ("record_type" = String, Path, description = "Record type, such as `contact`"),
        FieldListParams,
        PaginationQuery,
    ),
    responses(
        (status = 200, description = "System and custom fields ordered by position; inactive fields only with `include_inactive=true`", body = Page<FieldResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` and the read permission of the record type (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown record type (`record_type_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid pagination (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn list_fields(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(record_type): Path<String>,
    Query(params): Query<FieldListParams>,
    pagination: Pagination,
) -> Result<Json<Page<FieldResponse>>, ApiError> {
    let listing = field_use_cases(&state)
        .await?
        .list_fields(
            &tenant.scope,
            &record_type,
            params.include_inactive.unwrap_or(false),
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
    path = "/fields/{record_type}/schema",
    tag = TAG,
    operation_id = "fields_schema",
    security(("session_cookie" = [])),
    params(("record_type" = String, Path, description = "Record type, such as `contact`")),
    responses(
        (status = 200, description = "Active fields with their active options and filter operators, plus the operand kind of every operator", body = Data<FieldSchemaResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` and the read permission of the record type (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown record type (`record_type_not_found`)", body = ErrorEnvelope),
    )
)]
async fn field_schema(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(record_type): Path<String>,
) -> Result<Json<Data<FieldSchemaResponse>>, ApiError> {
    let schema = field_use_cases(&state)
        .await?
        .schema(&tenant.scope, &record_type)
        .await?;
    let schema = filter_schema(
        &state.db()?,
        &tenant.scope,
        &crate::application::shared::RecordType::parse(&record_type)?,
        schema,
    )
    .await?;
    Ok(Json(Data::new(schema.into())))
}

#[utoipa::path(
    post,
    path = "/fields/{record_type}",
    tag = TAG,
    operation_id = "fields_create",
    security(("session_cookie" = [])),
    params(("record_type" = String, Path, description = "Record type, such as `contact`")),
    request_body = CreateFieldRequest,
    responses(
        (status = 201, description = "Custom field created at the end of the list", body = Data<FieldResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown record type (`record_type_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "Key already used (`field_key_taken`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid fields, configuration or options (`validation_error`); too many fields (`too_many_fields`)", body = ErrorEnvelope),
    )
)]
async fn create_field(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(record_type): Path<String>,
    JsonBody(request): JsonBody<CreateFieldRequest>,
) -> Result<(StatusCode, Json<Data<FieldResponse>>), ApiError> {
    let field = field_use_cases(&state)
        .await?
        .create_field(
            &tenant.scope,
            &record_type,
            request.into_command(client.0.ip),
        )
        .await?;
    Ok((StatusCode::CREATED, Json(Data::new(field.into()))))
}

#[utoipa::path(
    put,
    path = "/fields/{record_type}/order",
    tag = TAG,
    operation_id = "fields_reorder",
    security(("session_cookie" = [])),
    params(("record_type" = String, Path, description = "Record type, such as `contact`")),
    request_body = ReorderRequest,
    responses(
        (status = 200, description = "Every field, including inactive ones, in the new order", body = Data<Vec<FieldResponse>>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown record type (`record_type_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "`ids` must list every field id exactly once (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn reorder_fields(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(record_type): Path<String>,
    JsonBody(request): JsonBody<ReorderRequest>,
) -> Result<Json<Data<Vec<FieldResponse>>>, ApiError> {
    let fields = field_use_cases(&state)
        .await?
        .reorder_fields(
            &tenant.scope,
            &record_type,
            request.into_command(client.0.ip),
        )
        .await?;
    Ok(Json(Data::new(
        fields.into_iter().map(Into::into).collect(),
    )))
}

#[utoipa::path(
    patch,
    path = "/fields/{record_type}/{field_id}",
    tag = TAG,
    operation_id = "fields_update",
    security(("session_cookie" = [])),
    params(
        ("record_type" = String, Path, description = "Record type, such as `contact`"),
        ("field_id" = String, Path, description = "Field id"),
        ("If-Match" = Option<String>, Header, description = "`updated_at` of the field as last read; when it changed since, answers 409 `version_conflict`"),
    ),
    request_body = UpdateFieldRequest,
    responses(
        (status = 200, description = "Field updated; `config` replaces the whole configuration. System fields accept only `label`", body = Data<FieldResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown record type or field (`record_type_not_found`, `field_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "Existing values are duplicated, so the field cannot become unique (`duplicate_values`), the field changed since `If-Match` (`version_conflict`) or the field is managed by a module (`field_managed_by_module`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid fields (`validation_error`), incompatible type change (`incompatible_field_type`) or system field restriction (`system_field_locked`)", body = ErrorEnvelope),
    )
)]
async fn update_field(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path((record_type, field_id)): Path<(String, String)>,
    IfMatch(expected_updated_at): IfMatch,
    JsonBody(request): JsonBody<UpdateFieldRequest>,
) -> Result<Json<Data<FieldResponse>>, ApiError> {
    let field = field_use_cases(&state)
        .await?
        .update_field(
            &tenant.scope,
            &record_type,
            &field_id,
            request.into_command(expected_updated_at, client.0.ip),
        )
        .await?;
    Ok(Json(Data::new(field.into())))
}

#[utoipa::path(
    post,
    path = "/fields/{record_type}/{field_id}/deactivate",
    tag = TAG,
    operation_id = "fields_deactivate",
    security(("session_cookie" = [])),
    params(
        ("record_type" = String, Path, description = "Record type, such as `contact`"),
        ("field_id" = String, Path, description = "Field id"),
    ),
    responses(
        (status = 200, description = "Field deactivated; its values are kept and come back when it is activated", body = Data<FieldResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown record type or field (`record_type_not_found`, `field_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The field is managed by a module (`field_managed_by_module`)", body = ErrorEnvelope),
        (status = 422, description = "System fields cannot be deactivated (`system_field_locked`)", body = ErrorEnvelope),
    )
)]
async fn deactivate_field(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path((record_type, field_id)): Path<(String, String)>,
) -> Result<Json<Data<FieldResponse>>, ApiError> {
    let field = field_use_cases(&state)
        .await?
        .set_field_active(&tenant.scope, &record_type, &field_id, false, client.0.ip)
        .await?;
    Ok(Json(Data::new(field.into())))
}

#[utoipa::path(
    post,
    path = "/fields/{record_type}/{field_id}/activate",
    tag = TAG,
    operation_id = "fields_activate",
    security(("session_cookie" = [])),
    params(
        ("record_type" = String, Path, description = "Record type, such as `contact`"),
        ("field_id" = String, Path, description = "Field id"),
    ),
    responses(
        (status = 200, description = "Field active again", body = Data<FieldResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown record type or field (`record_type_not_found`, `field_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The field is managed by a module (`field_managed_by_module`)", body = ErrorEnvelope),
        (status = 422, description = "System fields are always active (`system_field_locked`)", body = ErrorEnvelope),
    )
)]
async fn activate_field(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path((record_type, field_id)): Path<(String, String)>,
) -> Result<Json<Data<FieldResponse>>, ApiError> {
    let field = field_use_cases(&state)
        .await?
        .set_field_active(&tenant.scope, &record_type, &field_id, true, client.0.ip)
        .await?;
    Ok(Json(Data::new(field.into())))
}

#[utoipa::path(
    post,
    path = "/fields/{record_type}/{field_id}/options",
    tag = TAG,
    operation_id = "fields_add_option",
    security(("session_cookie" = [])),
    params(
        ("record_type" = String, Path, description = "Record type, such as `contact`"),
        ("field_id" = String, Path, description = "Field id"),
    ),
    request_body = FieldOptionRequest,
    responses(
        (status = 201, description = "Option added at the end; without `value`, one is derived from the label", body = Data<FieldResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown record type or field (`record_type_not_found`, `field_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "Value already used (`option_value_taken`) or the field is managed by a module (`field_managed_by_module`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid option (`validation_error`), field without options (`field_without_options`) or system field (`system_field_locked`)", body = ErrorEnvelope),
    )
)]
async fn add_option(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path((record_type, field_id)): Path<(String, String)>,
    JsonBody(request): JsonBody<FieldOptionRequest>,
) -> Result<(StatusCode, Json<Data<FieldResponse>>), ApiError> {
    let field = field_use_cases(&state)
        .await?
        .add_option(
            &tenant.scope,
            &record_type,
            &field_id,
            request.into(),
            client.0.ip,
        )
        .await?;
    Ok((StatusCode::CREATED, Json(Data::new(field.into()))))
}

#[utoipa::path(
    put,
    path = "/fields/{record_type}/{field_id}/options/order",
    tag = TAG,
    operation_id = "fields_reorder_options",
    security(("session_cookie" = [])),
    params(
        ("record_type" = String, Path, description = "Record type, such as `contact`"),
        ("field_id" = String, Path, description = "Field id"),
    ),
    request_body = ReorderRequest,
    responses(
        (status = 200, description = "Options in the new order", body = Data<FieldResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown record type or field (`record_type_not_found`, `field_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "`ids` must list every option id exactly once (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn reorder_options(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path((record_type, field_id)): Path<(String, String)>,
    JsonBody(request): JsonBody<ReorderRequest>,
) -> Result<Json<Data<FieldResponse>>, ApiError> {
    let field = field_use_cases(&state)
        .await?
        .reorder_options(
            &tenant.scope,
            &record_type,
            &field_id,
            request.into_command(client.0.ip),
        )
        .await?;
    Ok(Json(Data::new(field.into())))
}

#[utoipa::path(
    patch,
    path = "/fields/{record_type}/{field_id}/options/{option_id}",
    tag = TAG,
    operation_id = "fields_update_option",
    security(("session_cookie" = [])),
    params(
        ("record_type" = String, Path, description = "Record type, such as `contact`"),
        ("field_id" = String, Path, description = "Field id"),
        ("option_id" = String, Path, description = "Option id"),
        ("If-Match" = Option<String>, Header, description = "`updated_at` of the field as last read (option changes also move it); when it changed since, answers 409 `version_conflict`"),
    ),
    request_body = UpdateOptionRequest,
    responses(
        (status = 200, description = "Option updated; the value never changes, `color: null` removes the color", body = Data<FieldResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown record type, field or option (`record_type_not_found`, `field_not_found`, `field_option_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The field changed since `If-Match` (`version_conflict`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid label or color (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn update_option(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path((record_type, field_id, option_id)): Path<(String, String, String)>,
    IfMatch(expected_updated_at): IfMatch,
    JsonBody(request): JsonBody<UpdateOptionRequest>,
) -> Result<Json<Data<FieldResponse>>, ApiError> {
    let field = field_use_cases(&state)
        .await?
        .update_option(
            &tenant.scope,
            &record_type,
            &field_id,
            &option_id,
            request.into_command(expected_updated_at, client.0.ip),
        )
        .await?;
    Ok(Json(Data::new(field.into())))
}
