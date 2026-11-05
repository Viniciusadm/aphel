use axum::Json;
use axum::body::Body;
use axum::extract::multipart::{Field, MultipartRejection};
use axum::extract::rejection::QueryRejection;
use axum::extract::{DefaultBodyLimit, Multipart, Path, Query, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use bytes::{Bytes, BytesMut};
use utoipa_axum::router::{OpenApiRouter, UtoipaMethodRouterExt};
use utoipa_axum::routes;

use crate::api::dto::common::{Data, ErrorEnvelope, Page};
use crate::api::dto::files::{
    AttachFileRequest, AttachmentResponse, AttachmentsQueryParams, FILE_FIELD, FormFile,
    RECORD_ID_FIELD, RECORD_TYPE_FIELD, UploadFileForm, UploadFileResponse,
};
use crate::api::error::{ApiError, app_error};
use crate::api::extractors::{AuthContext, ClientInfo, JsonBody, Pagination, PaginationQuery};
use crate::api::state::AppState;
use crate::application::files::{
    AttachFileCommand, FileSettings, FileUseCases, UploadFileCommand, file_too_large,
};
use crate::application::organization_admin::ListWindow;
use crate::application::shared::RecordRef;
use crate::errors::AppError;
use crate::infrastructure::records::MySqlRecordAccessChecker;
use crate::infrastructure::storage::MySqlFilePorts;

pub const TAG: &str = "files";

const MAX_TEXT_FIELD_BYTES: usize = 256;
const DOWNLOAD_CONTENT_SECURITY_POLICY: &str = "default-src 'none'; sandbox";

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(upload_file).layer(DefaultBodyLimit::disable()))
        .routes(routes!(download_file))
        .routes(routes!(list_attachments, attach_file))
        .routes(routes!(remove_attachment))
}

pub async fn file_use_cases(
    state: &impl AsRef<AppState>,
) -> Result<FileUseCases<MySqlFilePorts>, AppError> {
    let state = state.as_ref();
    Ok(FileUseCases::new(
        MySqlFilePorts::new(
            state.db()?,
            state.storage.clone(),
            state.events.clone(),
            state.audit.clone(),
        )
        .with_records(MySqlRecordAccessChecker::new(
            state.db()?,
            state.registry().await?,
        )),
        FileSettings {
            max_upload_bytes: state.max_upload_bytes(),
        },
    ))
}

#[utoipa::path(
    post,
    path = "/files",
    tag = TAG,
    operation_id = "files_upload",
    security(("session_cookie" = [])),
    request_body(content = UploadFileForm, content_type = "multipart/form-data"),
    responses(
        (status = 201, description = "File stored, and attached when `record_type` and `record_id` were sent", body = Data<UploadFileResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing update permission on the record (`permission_denied`)", body = ErrorEnvelope),
        (status = 413, description = "The file exceeds the upload limit of the organization settings (`file_too_large`)", body = ErrorEnvelope),
        (status = 422, description = "Empty file (`empty_file`), forbidden type (`file_type_not_allowed`), unknown record type (`unsupported_record_type`) or invalid form", body = ErrorEnvelope),
    )
)]
async fn upload_file(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    multipart: Result<Multipart, MultipartRejection>,
) -> Response {
    match upload(&state, &tenant, client, multipart).await {
        Ok(uploaded) => (StatusCode::CREATED, Json(Data::new(uploaded))).into_response(),
        Err(error) => upload_error(error),
    }
}

async fn upload(
    state: &AppState,
    tenant: &AuthContext,
    client: ClientInfo,
    multipart: Result<Multipart, MultipartRejection>,
) -> Result<UploadFileResponse, AppError> {
    let use_cases = file_use_cases(state).await?;
    let max_upload_bytes = use_cases.settings().max_upload_bytes;
    let form = read_upload_form(multipart, max_upload_bytes).await?;
    let attach_to = match (form.record_type, form.record_id) {
        (Some(record_type), Some(record_id)) => Some(RecordRef::parse(&record_type, &record_id)?),
        (None, None) => None,
        _ => {
            return Err(AppError::Validation(
                "record_type and record_id must be sent together".to_owned(),
            ));
        }
    };

    let uploaded = use_cases
        .upload(
            &tenant.scope,
            UploadFileCommand {
                file_name: form.file.name,
                content: form.file.content,
                attach_to,
                ip: client.0.ip,
            },
        )
        .await?;
    Ok(uploaded.into())
}

async fn read_upload_form(
    multipart: Result<Multipart, MultipartRejection>,
    max_upload_bytes: u64,
) -> Result<UploadFileForm, AppError> {
    let mut multipart =
        multipart.map_err(|rejection| AppError::Validation(rejection.body_text()))?;
    let mut file: Option<FormFile> = None;
    let mut record_type: Option<String> = None;
    let mut record_id: Option<String> = None;
    while let Some(field) = multipart.next_field().await.map_err(invalid_form)? {
        match field.name() {
            Some(FILE_FIELD) if file.is_none() => {
                let name = field.file_name().unwrap_or_default().to_owned();
                let content = read_limited(field, max_upload_bytes).await?;
                file = Some(FormFile { name, content });
            }
            Some(RECORD_TYPE_FIELD) if record_type.is_none() => {
                record_type = Some(read_text(field).await?);
            }
            Some(RECORD_ID_FIELD) if record_id.is_none() => {
                record_id = Some(read_text(field).await?);
            }
            _ => {
                return Err(AppError::Validation(
                    "unexpected or repeated form field".to_owned(),
                ));
            }
        }
    }
    let file =
        file.ok_or_else(|| AppError::Validation("the form must include a file".to_owned()))?;
    Ok(UploadFileForm {
        file,
        record_type,
        record_id,
    })
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
    let bytes = read_limited(field, MAX_TEXT_FIELD_BYTES as u64)
        .await
        .map_err(|_| AppError::Validation("form field is too long".to_owned()))?;
    String::from_utf8(bytes.to_vec())
        .map(|value| value.trim().to_owned())
        .map_err(|_| AppError::Validation("form fields must be UTF-8 text".to_owned()))
}

fn invalid_form(error: axum::extract::multipart::MultipartError) -> AppError {
    AppError::Validation(format!("invalid multipart form: {}", error.body_text()))
}

fn upload_error(error: AppError) -> Response {
    app_error(error)
}

#[utoipa::path(
    get,
    path = "/files/{file_id}",
    tag = TAG,
    operation_id = "files_download",
    security(("session_cookie" = [])),
    params(("file_id" = String, Path, description = "File id")),
    responses(
        (status = 200, description = "File content, served as an attachment with the detected `Content-Type` and `X-Content-Type-Options: nosniff`", content_type = "application/octet-stream"),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 404, description = "The file does not exist or is not readable by the user (`file_not_found`). Organization exports and privacy packages always answer 404 here (they are downloaded only through their dedicated endpoints), and list exports only reach the user who requested them", body = ErrorEnvelope),
    )
)]
async fn download_file(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(file_id): Path<String>,
) -> Result<Response, ApiError> {
    let download = file_use_cases(&state)
        .await?
        .download(&tenant.scope, &file_id)
        .await?;
    let content_type = HeaderValue::from_str(&download.file.mime_type)
        .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream"));
    let disposition = HeaderValue::from_str(&content_disposition(&download.file.original_name))
        .map_err(|_| AppError::Internal("invalid content disposition".to_owned()))?;
    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CONTENT_LENGTH, download.file.size)
        .header(header::CONTENT_DISPOSITION, disposition)
        .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff")
        .header(header::CACHE_CONTROL, "private, no-store")
        .header(
            header::CONTENT_SECURITY_POLICY,
            DOWNLOAD_CONTENT_SECURITY_POLICY,
        )
        .body(Body::from_stream(download.content))
        .map_err(|error| AppError::Internal(format!("cannot build download response: {error}")))?;
    Ok(response)
}

pub fn content_disposition(file_name: &str) -> String {
    let fallback: String = file_name
        .chars()
        .map(|character| {
            if character == ' '
                || (character.is_ascii_graphic() && !matches!(character, '"' | '\\' | '%' | ';'))
            {
                character
            } else {
                '_'
            }
        })
        .collect();
    format!(
        "attachment; filename=\"{fallback}\"; filename*=UTF-8''{}",
        percent_encode(file_name)
    )
}

fn percent_encode(value: &str) -> String {
    value
        .bytes()
        .map(|byte| {
            if byte.is_ascii_alphanumeric()
                || matches!(
                    byte,
                    b'!' | b'#'
                        | b'$'
                        | b'&'
                        | b'+'
                        | b'-'
                        | b'.'
                        | b'^'
                        | b'_'
                        | b'`'
                        | b'|'
                        | b'~'
                )
            {
                (byte as char).to_string()
            } else {
                format!("%{byte:02X}")
            }
        })
        .collect()
}

#[utoipa::path(
    get,
    path = "/attachments",
    tag = TAG,
    operation_id = "attachments_list",
    security(("session_cookie" = [])),
    params(AttachmentsQueryParams, PaginationQuery),
    responses(
        (status = 200, description = "Attachments of the record, newest first", body = Page<AttachmentResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing read permission on the record (`permission_denied`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid record reference or pagination, or unknown record type (`unsupported_record_type`)", body = ErrorEnvelope),
    )
)]
async fn list_attachments(
    State(state): State<AppState>,
    tenant: AuthContext,
    pagination: Pagination,
    query: Result<Query<AttachmentsQueryParams>, QueryRejection>,
) -> Result<Json<Page<AttachmentResponse>>, ApiError> {
    let Query(params) = query.map_err(|rejection| AppError::Validation(rejection.body_text()))?;
    let listing = file_use_cases(&state)
        .await?
        .list_attachments(
            &tenant.scope,
            params.into(),
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
    path = "/attachments",
    tag = TAG,
    operation_id = "attachments_create",
    security(("session_cookie" = [])),
    request_body = AttachFileRequest,
    responses(
        (status = 201, description = "File attached to the record", body = Data<AttachmentResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing update permission on the record (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "File not found or not readable (`file_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The file is already attached to the record (`already_attached`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid record reference or unknown record type (`unsupported_record_type`)", body = ErrorEnvelope),
    )
)]
async fn attach_file(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    JsonBody(request): JsonBody<AttachFileRequest>,
) -> Result<(StatusCode, Json<Data<AttachmentResponse>>), ApiError> {
    let attachment = file_use_cases(&state)
        .await?
        .attach(
            &tenant.scope,
            AttachFileCommand {
                file_id: request.file_id,
                record_type: request.record_type,
                record_id: request.record_id,
                ip: client.0.ip,
            },
        )
        .await?;
    Ok((StatusCode::CREATED, Json(Data::new(attachment.into()))))
}

#[utoipa::path(
    delete,
    path = "/attachments/{attachment_id}",
    tag = TAG,
    operation_id = "attachments_remove",
    security(("session_cookie" = [])),
    params(("attachment_id" = String, Path, description = "Attachment id")),
    responses(
        (status = 204, description = "Attachment removed; the file itself is kept"),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing update permission on the record (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Attachment not found (`attachment_not_found`)", body = ErrorEnvelope),
    )
)]
async fn remove_attachment(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(attachment_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    file_use_cases(&state)
        .await?
        .remove_attachment(&tenant.scope, &attachment_id, client.0.ip)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
