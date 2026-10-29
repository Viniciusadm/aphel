use axum::Json;
use axum::extract::multipart::{Field, MultipartError, MultipartRejection};
use axum::extract::{DefaultBodyLimit, Multipart, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use bytes::{Bytes, BytesMut};
use utoipa_axum::router::{OpenApiRouter, UtoipaMethodRouterExt};
use utoipa_axum::routes;

use crate::dto::{
    FILE_FIELD, MAPS_FIELD, MigrationHistoryResponse, MigrationImportResponse,
    MigrationPreviewResponse, MigrationSourceResponse, MigrationUploadForm, SOURCE_FIELD,
    STEP_FIELD, ValueMapsInput,
};
use crate::{MigrationFileCommand, SOURCE_TOOLS, service};
use aphel_kernel::api::dto::common::{Data, ErrorEnvelope};
use aphel_kernel::api::dto::files::FormFile;
use aphel_kernel::api::error::app_error;
use aphel_kernel::api::extractors::{AuthContext, ClientInfo};
use aphel_kernel::api::state::AppState;
use aphel_kernel::application::files::{FILE_TOO_LARGE, file_too_large};
use aphel_kernel::context::Ctx;
use aphel_kernel::errors::{AppError, FieldViolation};

pub(crate) const TAG: &str = "migration-assistant";

const MAX_TEXT_FIELD_BYTES: u64 = 64;
const MAX_MAPS_BYTES: u64 = 256 * 1024;

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_sources))
        .routes(routes!(preview_migration_file).layer(DefaultBodyLimit::disable()))
        .routes(routes!(create_migration_import).layer(DefaultBodyLimit::disable()))
        .routes(routes!(migrate_history).layer(DefaultBodyLimit::disable()))
}

#[utoipa::path(
    get,
    path = "/migration-assistant/sources",
    tag = TAG,
    operation_id = "migration_assistant_sources",
    security(("session_cookie" = [])),
    responses(
        (status = 200, description = "Supported source CRMs (Pipedrive, HubSpot, RD Station CRM, Agendor), the suggested import order (companies, contacts, deals, then activities and notes, whose `kind` sends them to `/migration-assistant/history`) and the columns recognized in each export, with the Aphel field and the conversion applied. Headers were inferred from public documentation and may vary by account language and version", body = Data<Vec<MigrationSourceResponse>>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
    )
)]
async fn list_sources() -> Json<Data<Vec<MigrationSourceResponse>>> {
    Json(Data::new(SOURCE_TOOLS.iter().map(Into::into).collect()))
}

#[utoipa::path(
    post,
    path = "/migration-assistant/preview",
    tag = TAG,
    operation_id = "migration_assistant_preview",
    security(("session_cookie" = [])),
    request_body(content = MigrationUploadForm, content_type = "multipart/form-data"),
    responses(
        (status = 200, description = "Reads the CSV exported by the source tool without storing it: recognized and unrecognized columns, cells converted (dates, money, phones, owners by name, deal status), conversion issues per field, distinct values of stage, pipeline, owner and status columns (deal status is read from a status column or from won/lost stage names and clears the stage, so the deal goes to the first won or lost stage of its pipeline) (to build `maps`) and the first normalized rows. Requires `<type>.create`", body = Data<MigrationPreviewResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `<type>.create` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown source or step (`migration_source_not_found`)", body = ErrorEnvelope),
        (status = 413, description = "The file exceeds the upload limit of the organization settings (`file_too_large`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid form or `maps`, empty or invalid CSV (`empty_import`, `invalid_csv`, `too_many_rows`, `too_many_columns`)", body = ErrorEnvelope),
    )
)]
async fn preview_migration_file(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    multipart: Result<Multipart, MultipartRejection>,
) -> Response {
    let result = async {
        let command = read_form(&state, multipart, client.0.ip).await?;
        let ctx = Ctx::from_state(&state, &tenant.scope).await?;
        service::preview(&ctx, &command).await
    }
    .await;
    match result {
        Ok(preview) => Json(Data::new(MigrationPreviewResponse::from(preview))).into_response(),
        Err(error) => error_response(error),
    }
}

#[utoipa::path(
    post,
    path = "/migration-assistant/imports",
    tag = TAG,
    operation_id = "migration_assistant_create_import",
    security(("session_cookie" = [])),
    request_body(content = MigrationUploadForm, content_type = "multipart/form-data"),
    responses(
        (status = 201, description = "Converts the CSV of the source tool (dates, money, phones, owners, deal status and the `maps` of stages, pipelines and owners), stores it as a draft import (`POST /imports`) and pre-fills its mapping and options (duplicates by the step key, missing companies created by name). Review and start it with the import endpoints (`/imports/{import_id}`). Requires `<type>.create`", body = Data<MigrationImportResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `<type>.create` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown source or step (`migration_source_not_found`)", body = ErrorEnvelope),
        (status = 413, description = "The file exceeds the upload limit of the organization settings (`file_too_large`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid form or `maps`, empty or invalid CSV (`empty_import`, `invalid_csv`, `too_many_rows`, `too_many_columns`)", body = ErrorEnvelope),
    )
)]
async fn create_migration_import(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    multipart: Result<Multipart, MultipartRejection>,
) -> Response {
    let result = async {
        let command = read_form(&state, multipart, client.0.ip).await?;
        let ctx = Ctx::from_state(&state, &tenant.scope).await?;
        service::create_import(&ctx, command).await
    }
    .await;
    match result {
        Ok(created) => (
            StatusCode::CREATED,
            Json(Data::new(MigrationImportResponse::from(created))),
        )
            .into_response(),
        Err(error) => error_response(error),
    }
}

#[utoipa::path(
    post,
    path = "/migration-assistant/history",
    tag = TAG,
    operation_id = "migration_assistant_history",
    security(("session_cookie" = [])),
    request_body(content = MigrationUploadForm, content_type = "multipart/form-data"),
    responses(
        (status = 200, description = "Migrates the activities or notes exported by the source tool (steps `activities` and `notes`, up to 5 000 rows per file) right away through the activities and notes use cases (validation, permission, event and audit per row). Each row is linked to the deal (by title), contact (by e-mail or name) and company (by name) already in Aphel, so import companies, contacts and deals first; activity types are matched by name and owners by name or e-mail, done tasks keep their due date as completion. Notes need a linked record. Attachments are not part of the CSV exports and are not migrated. Rows that fail are listed in `report.errors` (first 100). Requires `activities.create` or `notes.create`", body = Data<MigrationHistoryResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `activities.create` or `notes.create` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown source or step, or the activities or notes module is disabled (`migration_source_not_found`)", body = ErrorEnvelope),
        (status = 413, description = "The file exceeds the upload limit of the organization settings (`file_too_large`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid form, a companies/contacts/deals step (`migration_history_step`), empty or invalid file or more than 5 000 rows (`empty_import`, `invalid_csv`, `too_many_rows`, `too_many_columns`)", body = ErrorEnvelope),
    )
)]
async fn migrate_history(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    multipart: Result<Multipart, MultipartRejection>,
) -> Response {
    let result = async {
        let command = read_form(&state, multipart, client.0.ip).await?;
        let ctx = Ctx::from_state(&state, &tenant.scope).await?;
        service::migrate_history(&ctx, command).await
    }
    .await;
    match result {
        Ok(migrated) => Json(Data::new(MigrationHistoryResponse::from(migrated))).into_response(),
        Err(error) => error_response(error),
    }
}

fn error_response(error: AppError) -> Response {
    let too_large = matches!(&error, AppError::Coded { code, .. } if *code == FILE_TOO_LARGE);
    let mut response = app_error(error);
    if too_large {
        *response.status_mut() = StatusCode::PAYLOAD_TOO_LARGE;
    }
    response
}

async fn read_form(
    state: &AppState,
    multipart: Result<Multipart, MultipartRejection>,
    ip: Option<String>,
) -> Result<MigrationFileCommand, AppError> {
    let form = read_upload_form(state, multipart).await?;
    let maps: ValueMapsInput = match form.maps.as_deref().map(str::trim) {
        None | Some("") => ValueMapsInput::default(),
        Some(text) => serde_json::from_str(text).map_err(|_| {
            AppError::invalid_fields(
                "invalid migration form",
                vec![FieldViolation::new(
                    MAPS_FIELD,
                    "must be a JSON object with stage_id, pipeline_id and owner_id maps",
                )],
            )
        })?,
    };
    Ok(MigrationFileCommand {
        source: form.source,
        step: form.step,
        file_name: form.file.name,
        content: form.file.content,
        maps: maps.into(),
        ip,
    })
}

async fn read_upload_form(
    state: &AppState,
    multipart: Result<Multipart, MultipartRejection>,
) -> Result<MigrationUploadForm, AppError> {
    let mut multipart =
        multipart.map_err(|rejection| AppError::Validation(rejection.body_text()))?;
    let mut file: Option<FormFile> = None;
    let mut source: Option<String> = None;
    let mut step: Option<String> = None;
    let mut maps: Option<String> = None;
    while let Some(field) = multipart.next_field().await.map_err(invalid_form)? {
        match field.name() {
            Some(FILE_FIELD) if file.is_none() => {
                let name = field.file_name().unwrap_or("export.csv").to_owned();
                let content = read_limited(field, state.max_upload_bytes()).await?;
                file = Some(FormFile { name, content });
            }
            Some(SOURCE_FIELD) if source.is_none() => {
                source = Some(read_text(field, MAX_TEXT_FIELD_BYTES).await?);
            }
            Some(STEP_FIELD) if step.is_none() => {
                step = Some(read_text(field, MAX_TEXT_FIELD_BYTES).await?);
            }
            Some(MAPS_FIELD) if maps.is_none() => {
                maps = Some(read_text(field, MAX_MAPS_BYTES).await?);
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
    let source = source.filter(|value| !value.is_empty());
    if source.is_none() {
        violations.push(FieldViolation::new(SOURCE_FIELD, "is required"));
    }
    let step = step.filter(|value| !value.is_empty());
    if step.is_none() {
        violations.push(FieldViolation::new(STEP_FIELD, "is required"));
    }
    match (file, source, step) {
        (Some(file), Some(source), Some(step)) => Ok(MigrationUploadForm {
            file,
            source,
            step,
            maps,
        }),
        _ => Err(AppError::invalid_fields(
            "invalid migration form",
            violations,
        )),
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

async fn read_text(field: Field<'_>, max_bytes: u64) -> Result<String, AppError> {
    let bytes = read_limited(field, max_bytes)
        .await
        .map_err(|_| AppError::Validation("form field is too long".to_owned()))?;
    String::from_utf8(bytes.to_vec())
        .map(|value| value.trim().to_owned())
        .map_err(|_| AppError::Validation("form fields must be UTF-8 text".to_owned()))
}

fn invalid_form(error: MultipartError) -> AppError {
    AppError::Validation(format!("invalid multipart form: {}", error.body_text()))
}
