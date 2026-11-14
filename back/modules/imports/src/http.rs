use axum::Json;
use axum::extract::multipart::{Field, MultipartRejection};
use axum::extract::{DefaultBodyLimit, Multipart, Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use bytes::{Bytes, BytesMut};
use utoipa_axum::router::{OpenApiRouter, UtoipaMethodRouterExt};
use utoipa_axum::routes;

use crate::dto::{
    ConfigureImportRequest, FILE_FIELD, ImportJobResponse, ImportListParams, ImportRowResponse,
    ImportRowsParams, ImportUploadForm, ImportValidationResponse, RECORD_TYPE_FIELD,
    RETRY_OF_FIELD,
};
use crate::{CreateImportCommand, RowStatus, service};
use aphel_kernel::context::Ctx;
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_sdk::platform::api::dto::common::{Data, ErrorEnvelope, Page};
use aphel_sdk::platform::api::dto::files::FormFile;
use aphel_sdk::platform::api::error::{ApiError, app_error};
use aphel_sdk::platform::api::extractors::{
    AuthContext, ClientInfo, JsonBody, Pagination, PaginationQuery,
};
use aphel_sdk::platform::api::routes::v1::members::window;
use aphel_sdk::platform::api::state::AppState;
use aphel_sdk::platform::application::files::{FILE_TOO_LARGE, file_too_large};

pub(crate) const TAG: &str = "imports";

const MAX_TEXT_FIELD_BYTES: u64 = 256;

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(create_import).layer(DefaultBodyLimit::disable()))
        .routes(routes!(list_imports))
        .routes(routes!(get_import, delete_import))
        .routes(routes!(configure_import))
        .routes(routes!(validate_import))
        .routes(routes!(start_import))
        .routes(routes!(retry_failed_rows))
        .routes(routes!(list_import_rows))
}

#[utoipa::path(
    post,
    path = "/imports",
    tag = TAG,
    operation_id = "imports_create",
    security(("session_cookie" = [])),
    request_body(content = ImportUploadForm, content_type = "multipart/form-data"),
    responses(
        (status = 201, description = "CSV or spreadsheet (XLSX, XLS, ODS; first sheet) stored and parsed: for CSV, detected encoding (UTF-8 with or without BOM, otherwise Windows-1252) and delimiter (`,` `;` tab `|`), headers, the first 20 rows, the suggested mapping and the importable fields. With `retry_of`, the mapping and options of that import are reused by column name", body = Data<ImportJobResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `<type>.create` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "`retry_of` import not found (`import_not_found`)", body = ErrorEnvelope),
        (status = 413, description = "The file exceeds the upload limit of the organization settings (`file_too_large`)", body = ErrorEnvelope),
        (status = 422, description = "Unsupported record type (`import_not_supported`), empty or invalid file (`empty_import`, `invalid_csv`, `invalid_spreadsheet`, `too_many_rows`, `too_many_columns`) or invalid form", body = ErrorEnvelope),
    )
)]
async fn create_import(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    multipart: Result<Multipart, MultipartRejection>,
) -> Response {
    let result = async {
        let command = read_upload(&state, multipart, client.0.ip).await?;
        let ctx = Ctx::from_state(&state, &tenant.scope).await?;
        service::create(&ctx, command).await
    }
    .await;
    match result {
        Ok(details) => (
            StatusCode::CREATED,
            Json(Data::new(ImportJobResponse::from(details))),
        )
            .into_response(),
        Err(error) => {
            let too_large =
                matches!(&error, AppError::Coded { code, .. } if *code == FILE_TOO_LARGE);
            let mut response = app_error(error);
            if too_large {
                *response.status_mut() = StatusCode::PAYLOAD_TOO_LARGE;
            }
            response
        }
    }
}

async fn read_upload(
    state: &AppState,
    multipart: Result<Multipart, MultipartRejection>,
    ip: Option<String>,
) -> Result<CreateImportCommand, AppError> {
    let form = read_upload_form(state, multipart).await?;
    Ok(CreateImportCommand {
        record_type: form.record_type,
        file_name: form.file.name,
        content: form.file.content,
        retry_of: form.retry_of,
        ip,
    })
}

async fn read_upload_form(
    state: &AppState,
    multipart: Result<Multipart, MultipartRejection>,
) -> Result<ImportUploadForm, AppError> {
    let mut multipart =
        multipart.map_err(|rejection| AppError::Validation(rejection.body_text()))?;
    let mut file: Option<FormFile> = None;
    let mut record_type: Option<String> = None;
    let mut retry_of: Option<String> = None;
    while let Some(field) = multipart.next_field().await.map_err(invalid_form)? {
        match field.name() {
            Some(FILE_FIELD) if file.is_none() => {
                let name = field.file_name().unwrap_or("import.csv").to_owned();
                let content = read_limited(field, state.max_upload_bytes()).await?;
                file = Some(FormFile { name, content });
            }
            Some(RECORD_TYPE_FIELD) if record_type.is_none() => {
                record_type = Some(read_text(field).await?);
            }
            Some(RETRY_OF_FIELD) if retry_of.is_none() => {
                retry_of = Some(read_text(field).await?);
            }
            _ => {
                return Err(AppError::Validation(
                    "unexpected or repeated form field".to_owned(),
                ));
            }
        }
    }
    let mut violations = Vec::new();
    if file.is_none() {
        violations.push(FieldViolation::new(FILE_FIELD, "is required"));
    }
    let record_type = record_type.filter(|value| !value.is_empty());
    if record_type.is_none() {
        violations.push(FieldViolation::new(RECORD_TYPE_FIELD, "is required"));
    }
    match (file, record_type) {
        (Some(file), Some(record_type)) => Ok(ImportUploadForm {
            file,
            record_type,
            retry_of,
        }),
        _ => Err(AppError::invalid_fields("invalid import form", violations)),
    }
}

async fn read_limited(mut field: Field<'_>, max_bytes: u64) -> Result<Bytes, AppError> {
    let mut buffer = BytesMut::new();
    while let Some(chunk) = field.chunk().await.map_err(invalid_form)? {
        if (buffer.len() + chunk.len()) as u64 > max_bytes {
            return Err(file_too_large(max_bytes));
        }
        buffer.extend_from_slice(&chunk);
    }
    Ok(buffer.freeze())
}

async fn read_text(field: Field<'_>) -> Result<String, AppError> {
    let bytes = read_limited(field, MAX_TEXT_FIELD_BYTES)
        .await
        .map_err(|_| AppError::Validation("form field is too long".to_owned()))?;
    String::from_utf8(bytes.to_vec())
        .map(|value| value.trim().to_owned())
        .map_err(|_| AppError::Validation("form fields must be UTF-8 text".to_owned()))
}

fn invalid_form(error: axum::extract::multipart::MultipartError) -> AppError {
    AppError::Validation(format!("invalid multipart form: {}", error.body_text()))
}

#[utoipa::path(
    get,
    path = "/imports",
    tag = TAG,
    operation_id = "imports_list",
    security(("session_cookie" = [])),
    params(ImportListParams, PaginationQuery),
    responses(
        (status = 200, description = "Imports created by the user (every import of the organization with `settings.manage`), most recent first", body = Page<ImportJobResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 422, description = "Invalid pagination (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn list_imports(
    State(state): State<AppState>,
    tenant: AuthContext,
    Query(params): Query<ImportListParams>,
    pagination: Pagination,
) -> Result<Json<Page<ImportJobResponse>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let listing = service::list(&ctx, params.record_type, window(pagination)).await?;
    Ok(Json(Page::new(
        listing.items.into_iter().map(Into::into).collect(),
        listing.total,
        pagination,
    )))
}

#[utoipa::path(
    get,
    path = "/imports/{import_id}",
    tag = TAG,
    operation_id = "imports_get",
    security(("session_cookie" = [])),
    params(("import_id" = String, Path, description = "Import id")),
    responses(
        (status = 200, description = "Import with its mapping, options, progress (`totals`) and importable fields; `error_file_id` points to the CSV of failed rows (download with `GET /files/{id}`)", body = Data<ImportJobResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 404, description = "Import not found or not visible (`import_not_found`)", body = ErrorEnvelope),
    )
)]
async fn get_import(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(import_id): Path<String>,
) -> Result<Json<Data<ImportJobResponse>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let details = service::get(&ctx, &import_id).await?;
    Ok(Json(Data::new(details.into())))
}

#[utoipa::path(
    delete,
    path = "/imports/{import_id}",
    tag = TAG,
    operation_id = "imports_delete",
    security(("session_cookie" = [])),
    params(("import_id" = String, Path, description = "Import id")),
    responses(
        (status = 204, description = "Import, its rows and its files deleted (rows already written to records stay). Only the creator can delete"),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Not the creator (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Import not found (`import_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The import is queued or running (`import_in_progress`)", body = ErrorEnvelope),
    )
)]
async fn delete_import(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(import_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    service::delete(&ctx, state.storage.as_ref(), &import_id, client.0.ip).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    put,
    path = "/imports/{import_id}/mapping",
    tag = TAG,
    operation_id = "imports_configure",
    security(("session_cookie" = [])),
    params(("import_id" = String, Path, description = "Import id")),
    request_body = ConfigureImportRequest,
    responses(
        (status = 200, description = "Mapping (one field key or `null` per column) and duplicate options saved", body = Data<ImportJobResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Only the creator changes an import (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Import not found (`import_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The import was already started (`import_not_editable`)", body = ErrorEnvelope),
        (status = 422, description = "Unknown or repeated field (`mapping.<index>`) or unsupported duplicate key", body = ErrorEnvelope),
    )
)]
async fn configure_import(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(import_id): Path<String>,
    JsonBody(request): JsonBody<ConfigureImportRequest>,
) -> Result<Json<Data<ImportJobResponse>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let details = service::configure(&ctx, &import_id, request.into()).await?;
    Ok(Json(Data::new(details.into())))
}

#[utoipa::path(
    post,
    path = "/imports/{import_id}/validate",
    tag = TAG,
    operation_id = "imports_validate",
    security(("session_cookie" = [])),
    params(("import_id" = String, Path, description = "Import id")),
    responses(
        (status = 200, description = "Dry run of the saved mapping: counts of valid and invalid rows and the errors of the first 200 invalid rows. Nothing is written; duplicates and the module rules not covered by the field types are checked only when the import runs", body = Data<ImportValidationResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `<type>.create` or not the creator (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Import not found (`import_not_found`)", body = ErrorEnvelope),
    )
)]
async fn validate_import(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(import_id): Path<String>,
) -> Result<Json<Data<ImportValidationResponse>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let report = service::validate(&ctx, &import_id).await?;
    Ok(Json(Data::new(report.into())))
}

#[utoipa::path(
    post,
    path = "/imports/{import_id}/start",
    tag = TAG,
    operation_id = "imports_start",
    security(("session_cookie" = [])),
    params(("import_id" = String, Path, description = "Import id")),
    responses(
        (status = 202, description = "Rows stored and the `import.run` job queued; the worker imports them in batches of 200 and updates `totals`. Ends with `import.completed` or `import.failed`", body = Data<ImportJobResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `<type>.create` (or `<type>.update` to update duplicates), or not the creator (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Import not found (`import_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The import was already started (`import_not_editable`)", body = ErrorEnvelope),
        (status = 422, description = "A required field is not mapped (`mapping`)", body = ErrorEnvelope),
    )
)]
async fn start_import(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(import_id): Path<String>,
) -> Result<(StatusCode, Json<Data<ImportJobResponse>>), ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let details = service::start(&ctx, &import_id, client.0.ip).await?;
    Ok((StatusCode::ACCEPTED, Json(Data::new(details.into()))))
}

#[utoipa::path(
    post,
    path = "/imports/{import_id}/retry-failed",
    tag = TAG,
    operation_id = "imports_retry_failed",
    security(("session_cookie" = [])),
    params(("import_id" = String, Path, description = "Import id")),
    responses(
        (status = 202, description = "Failed rows go back to `pending` with the same content and the job is queued again (useful when the cause was outside the file, such as a missing member). To fix the data, upload the corrected error CSV with `retry_of`", body = Data<ImportJobResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Not the creator or missing `<type>.create` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Import not found (`import_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The import is not finished or has no failed rows (`import_not_retryable`)", body = ErrorEnvelope),
    )
)]
async fn retry_failed_rows(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(import_id): Path<String>,
) -> Result<(StatusCode, Json<Data<ImportJobResponse>>), ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let details = service::retry_failed(&ctx, &import_id, client.0.ip).await?;
    Ok((StatusCode::ACCEPTED, Json(Data::new(details.into()))))
}

#[utoipa::path(
    get,
    path = "/imports/{import_id}/rows",
    tag = TAG,
    operation_id = "imports_rows",
    security(("session_cookie" = [])),
    params(("import_id" = String, Path, description = "Import id"), ImportRowsParams, PaginationQuery),
    responses(
        (status = 200, description = "Rows of a started import in file order, with status (`pending`, `imported`, `skipped`, `failed`), action (`created`, `updated`, `skipped`), record id and errors", body = Page<ImportRowResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 404, description = "Import not found (`import_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid status or pagination", body = ErrorEnvelope),
    )
)]
async fn list_import_rows(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(import_id): Path<String>,
    Query(params): Query<ImportRowsParams>,
    pagination: Pagination,
) -> Result<Json<Page<ImportRowResponse>>, ApiError> {
    let status = match params.status.as_deref().map(str::trim) {
        None | Some("") => None,
        Some(value) => Some(RowStatus::parse(value).ok_or_else(|| {
            AppError::invalid_fields(
                "invalid query",
                vec![FieldViolation::new(
                    "status",
                    "must be pending, imported, skipped or failed",
                )],
            )
        })?),
    };
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let listing = service::rows(&ctx, &import_id, status, window(pagination)).await?;
    Ok(Json(Page::new(
        listing.items.into_iter().map(Into::into).collect(),
        listing.total,
        pagination,
    )))
}
