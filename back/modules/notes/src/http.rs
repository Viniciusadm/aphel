use axum::Json;
use axum::extract::rejection::QueryRejection;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::domain::{
    CreateNoteCommand, ListNotesQuery, NoteView, NoteVisibility, UpdateNoteCommand,
};
use crate::service;
use aphel_kernel::Ctx;
use aphel_kernel::api::dto::common::{Data, ErrorEnvelope, Page, timestamp as instant};
use aphel_kernel::api::dto::files::UserReferenceResponse;
use aphel_kernel::api::error::ApiError;
use aphel_kernel::api::extractors::{
    AuthContext, ClientInfo, IfMatch, JsonBody, Pagination, PaginationQuery,
};
use aphel_kernel::api::routes::v1::members::window;
use aphel_kernel::api::state::AppState;
use aphel_kernel::errors::AppError;

pub(crate) const TAG: &str = "notes";

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_notes, create_note))
        .routes(routes!(get_note, update_note, delete_note))
        .routes(routes!(pin_note))
        .routes(routes!(unpin_note))
}

#[utoipa::path(
    get,
    path = "/notes",
    tag = TAG,
    operation_id = "notes_list",
    security(("session_cookie" = [])),
    params(NotesQueryParams, PaginationQuery),
    responses(
        (status = 200, description = "Notes of the record visible to the user (team notes and the user's own private notes), pinned first and then newest first", body = Page<NoteResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `notes.read` or read access to the record (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid record reference or pagination, or unknown record type (`unsupported_record_type`); other invalid input answers `validation_error` with `details.fields`", body = ErrorEnvelope),
    )
)]
async fn list_notes(
    State(state): State<AppState>,
    tenant: AuthContext,
    pagination: Pagination,
    query: Result<Query<NotesQueryParams>, QueryRejection>,
) -> Result<Json<Page<NoteResponse>>, ApiError> {
    let Query(params) = query.map_err(|rejection| AppError::Validation(rejection.body_text()))?;
    let listing = service::list(
        &Ctx::from_state(&state, &tenant.scope).await?,
        params.into(),
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
    post,
    path = "/notes",
    tag = TAG,
    operation_id = "notes_create",
    security(("session_cookie" = [])),
    request_body = CreateNoteRequest,
    responses(
        (status = 201, description = "Note created with the user as author. `private` notes are visible only to their author, administrators included", body = Data<NoteResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `notes.create` or read access to the record (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 422, description = "Empty or too long body, invalid visibility or record reference (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn create_note(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    JsonBody(request): JsonBody<CreateNoteRequest>,
) -> Result<(StatusCode, Json<Data<NoteResponse>>), ApiError> {
    let note = service::create(
        &Ctx::from_state(&state, &tenant.scope).await?,
        request.into_command(client.0.ip),
    )
    .await?;
    Ok((StatusCode::CREATED, Json(Data::new(note.into()))))
}

#[utoipa::path(
    get,
    path = "/notes/{note_id}",
    tag = TAG,
    operation_id = "notes_get",
    security(("session_cookie" = [])),
    params(("note_id" = String, Path, description = "Note id")),
    responses(
        (status = 200, description = "The note", body = Data<NoteResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `notes.read` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Note not found, private note of another user or unreadable record (`note_not_found`)", body = ErrorEnvelope),
    )
)]
async fn get_note(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(note_id): Path<String>,
) -> Result<Json<Data<NoteResponse>>, ApiError> {
    let note = service::get(&Ctx::from_state(&state, &tenant.scope).await?, &note_id).await?;
    Ok(Json(Data::new(note.into())))
}

#[utoipa::path(
    patch,
    path = "/notes/{note_id}",
    tag = TAG,
    operation_id = "notes_update",
    security(("session_cookie" = [])),
    params(
        ("note_id" = String, Path, description = "Note id"),
        ("If-Match" = Option<String>, Header, description = "`updated_at` of the note as last read (ISO-8601); when it changed since, the update is refused with 409 `version_conflict`"),
    ),
    request_body = UpdateNoteRequest,
    responses(
        (status = 200, description = "Note updated. The author edits with `notes.update`; others need `notes.update` with a scope that includes the author, and only for team notes. Only the author changes the visibility", body = Data<NoteResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Not allowed to edit this note (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Note not found (`note_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The note changed after the `If-Match` version (`version_conflict`)", body = ErrorEnvelope),
        (status = 422, description = "Empty or too long body, or invalid visibility (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn update_note(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    IfMatch(expected_updated_at): IfMatch,
    Path(note_id): Path<String>,
    JsonBody(request): JsonBody<UpdateNoteRequest>,
) -> Result<Json<Data<NoteResponse>>, ApiError> {
    let note = service::update(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &note_id,
        request.into_command(client.0.ip, expected_updated_at),
    )
    .await?;
    Ok(Json(Data::new(note.into())))
}

#[utoipa::path(
    delete,
    path = "/notes/{note_id}",
    tag = TAG,
    operation_id = "notes_delete",
    security(("session_cookie" = [])),
    params(("note_id" = String, Path, description = "Note id")),
    responses(
        (status = 204, description = "Note deleted"),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Not allowed to delete this note (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Note not found (`note_not_found`)", body = ErrorEnvelope),
    )
)]
async fn delete_note(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(note_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    service::delete(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &note_id,
        client.0.ip,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/notes/{note_id}/pin",
    tag = TAG,
    operation_id = "notes_pin",
    security(("session_cookie" = [])),
    params(("note_id" = String, Path, description = "Note id")),
    responses(
        (status = 200, description = "Note pinned to the top of the record's notes", body = Data<NoteResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Not allowed to edit this note (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Note not found (`note_not_found`)", body = ErrorEnvelope),
    )
)]
async fn pin_note(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(note_id): Path<String>,
) -> Result<Json<Data<NoteResponse>>, ApiError> {
    let note = service::set_pinned(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &note_id,
        true,
        client.0.ip,
    )
    .await?;
    Ok(Json(Data::new(note.into())))
}

#[utoipa::path(
    post,
    path = "/notes/{note_id}/unpin",
    tag = TAG,
    operation_id = "notes_unpin",
    security(("session_cookie" = [])),
    params(("note_id" = String, Path, description = "Note id")),
    responses(
        (status = 200, description = "Note no longer pinned", body = Data<NoteResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Not allowed to edit this note (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Note not found (`note_not_found`)", body = ErrorEnvelope),
    )
)]
async fn unpin_note(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(note_id): Path<String>,
) -> Result<Json<Data<NoteResponse>>, ApiError> {
    let note = service::set_pinned(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &note_id,
        false,
        client.0.ip,
    )
    .await?;
    Ok(Json(Data::new(note.into())))
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = NoteVisibility)]
pub(crate) enum NoteVisibilitySchema {
    Team,
    Private,
}

impl From<NoteVisibility> for NoteVisibilitySchema {
    fn from(visibility: NoteVisibility) -> Self {
        match visibility {
            NoteVisibility::Team => Self::Team,
            NoteVisibility::Private => Self::Private,
        }
    }
}

impl From<NoteVisibilitySchema> for NoteVisibility {
    fn from(visibility: NoteVisibilitySchema) -> Self {
        match visibility {
            NoteVisibilitySchema::Team => Self::Team,
            NoteVisibilitySchema::Private => Self::Private,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct NoteResponse {
    pub(crate) id: String,
    #[schema(example = "contacts.contact")]
    pub(crate) record_type: String,
    pub(crate) record_id: String,
    pub(crate) author_id: Option<String>,
    pub(crate) author: Option<UserReferenceResponse>,
    #[schema(example = "**Meeting** scheduled for Friday.")]
    pub(crate) body: String,
    pub(crate) visibility: NoteVisibilitySchema,
    pub(crate) pinned: bool,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

impl From<NoteView> for NoteResponse {
    fn from(note: NoteView) -> Self {
        Self {
            id: note.id.to_string(),
            record_type: note.record.record_type.as_str().to_owned(),
            record_id: note.record.record_id.to_string(),
            author_id: note.author_id().map(|author| author.as_str().to_owned()),
            author: note.author.map(Into::into),
            body: note.body,
            visibility: note.visibility.into(),
            pinned: note.pinned,
            created_at: instant(note.created_at),
            updated_at: instant(note.updated_at),
        }
    }
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct NotesQueryParams {
    #[param(example = "contacts.contact")]
    pub(crate) record_type: String,
    pub(crate) record_id: String,
}

impl From<NotesQueryParams> for ListNotesQuery {
    fn from(params: NotesQueryParams) -> Self {
        Self {
            record_type: params.record_type,
            record_id: params.record_id,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreateNoteRequest {
    #[schema(example = "contacts.contact")]
    pub(crate) record_type: String,
    pub(crate) record_id: String,
    pub(crate) body: String,
    pub(crate) visibility: Option<NoteVisibilitySchema>,
    pub(crate) pinned: Option<bool>,
}

impl CreateNoteRequest {
    pub(crate) fn into_command(self, ip: Option<String>) -> CreateNoteCommand {
        CreateNoteCommand {
            record_type: self.record_type,
            record_id: self.record_id,
            body: self.body,
            visibility: self
                .visibility
                .map(|visibility| NoteVisibility::from(visibility).as_str().to_owned()),
            pinned: self.pinned,
            ip,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct UpdateNoteRequest {
    pub(crate) body: Option<String>,
    pub(crate) visibility: Option<NoteVisibilitySchema>,
    pub(crate) pinned: Option<bool>,
}

impl UpdateNoteRequest {
    pub(crate) fn into_command(
        self,
        ip: Option<String>,
        expected_updated_at: Option<DateTime<Utc>>,
    ) -> UpdateNoteCommand {
        UpdateNoteCommand {
            body: self.body,
            visibility: self.visibility.map(Into::into),
            pinned: self.pinned,
            expected_updated_at,
            ip,
        }
    }
}
