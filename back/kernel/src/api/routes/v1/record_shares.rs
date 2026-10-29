use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::api::dto::common::{Data, ErrorEnvelope};
use crate::api::dto::record_shares::{
    CreateRecordShareRequest, RecordShareResponse, RecordSharesResponse, UpdateRecordShareRequest,
};
use crate::api::error::ApiError;
use crate::api::extractors::{AuthContext, ClientInfo, IfMatch, JsonBody};
use crate::api::state::AppState;
use crate::application::record_sharing::RecordSharingUseCases;
use crate::errors::AppError;
use crate::infrastructure::record_sharing::{MySqlRecordSharingAdapters, MySqlRecordSharingPorts};

pub const TAG: &str = "record-sharing";

const UNAUTHORIZED: &str = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)";

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_record_shares, create_record_share))
        .routes(routes!(update_record_share, delete_record_share))
}

pub async fn record_sharing_use_cases(
    state: &AppState,
) -> Result<RecordSharingUseCases<MySqlRecordSharingPorts>, AppError> {
    Ok(RecordSharingUseCases::new(MySqlRecordSharingPorts::new(
        state.db()?,
        state.registry().await?,
        MySqlRecordSharingAdapters {
            events: state.events.clone(),
            audit: state.audit.clone(),
            security_events: state.security_events.clone(),
        },
    )))
}

#[utoipa::path(
    get,
    path = "/records/{record_type}/{record_id}/shares",
    tag = TAG,
    operation_id = "record_shares_list",
    security(("session_cookie" = [])),
    description = "Users and teams the record is shared with. Anyone who reads the record (read permission of its type, shares included) sees the list; `can_manage` tells whether the caller can change it (owner of the record or `records.share`).",
    params(
        ("record_type" = String, Path, description = "Record type, such as `deal` or `co_contract`"),
        ("record_id" = String, Path, description = "Record id"),
    ),
    responses(
        (status = 200, description = "Shares of the record", body = Data<RecordSharesResponse>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = "Missing the read permission of the type (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown type or record not visible (`record_type_not_found`, `record_not_found`)", body = ErrorEnvelope),
    )
)]
async fn list_record_shares(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path((record_type, record_id)): Path<(String, String)>,
) -> Result<Json<Data<RecordSharesResponse>>, ApiError> {
    let shares = record_sharing_use_cases(&state)
        .await?
        .list(&tenant.scope, &record_type, &record_id)
        .await?;
    Ok(Json(Data::new(shares.into())))
}

#[utoipa::path(
    post,
    path = "/records/{record_type}/{record_id}/shares",
    tag = TAG,
    operation_id = "record_shares_create",
    security(("session_cookie" = [])),
    description = "Shares a record with a user (`grantee_type: user`, an active member) or a team (`team`) with `read` or `edit` access, and publishes `record.shared` (audited and recorded in the security log). Only the owner of the record or someone with `records.share` who reads the record can share it; a share never allows deleting nor sharing again. `read` adds the record to every read and export of the grantee (lists, search, reports, dashboards); `edit` also allows updating, moving and archiving it when the grantee has the update permission of the type. At most 100 shares per record.",
    params(
        ("record_type" = String, Path, description = "Record type"),
        ("record_id" = String, Path, description = "Record id"),
    ),
    request_body = CreateRecordShareRequest,
    responses(
        (status = 201, description = "Record shared", body = Data<RecordShareResponse>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = "Not the owner nor `records.share` (`record_share_not_allowed`) or missing the read permission (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown type or record not visible (`record_type_not_found`, `record_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "Already shared with this user or team (`record_share_exists`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid grantee or access (`validation_error`) or share limit reached (`record_shares_limit_reached`)", body = ErrorEnvelope),
    )
)]
async fn create_record_share(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path((record_type, record_id)): Path<(String, String)>,
    JsonBody(request): JsonBody<CreateRecordShareRequest>,
) -> Result<(StatusCode, Json<Data<RecordShareResponse>>), ApiError> {
    let share = record_sharing_use_cases(&state)
        .await?
        .create(
            &tenant.scope,
            &record_type,
            &record_id,
            request.into_command(client.0),
        )
        .await?;
    Ok((StatusCode::CREATED, Json(Data::new(share.into()))))
}

#[utoipa::path(
    patch,
    path = "/records/{record_type}/{record_id}/shares/{share_id}",
    tag = TAG,
    operation_id = "record_shares_update",
    security(("session_cookie" = [])),
    description = "Changes the access (`read` or `edit`) of a share and publishes `record.share_updated`. Same rule as sharing: the owner of the record or `records.share`.",
    params(
        ("record_type" = String, Path, description = "Record type"),
        ("record_id" = String, Path, description = "Record id"),
        ("share_id" = String, Path, description = "Share id"),
        ("If-Match" = Option<String>, Header, description = "`updated_at` of the share as read (ISO-8601); the update fails with 409 `version_conflict` when it changed since"),
    ),
    request_body = UpdateRecordShareRequest,
    responses(
        (status = 200, description = "Share updated", body = Data<RecordShareResponse>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = "Not the owner nor `records.share` (`record_share_not_allowed`)", body = ErrorEnvelope),
        (status = 404, description = "Record or share not found (`record_not_found`, `record_share_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The share changed since the `If-Match` version (`version_conflict`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid access (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn update_record_share(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    IfMatch(expected): IfMatch,
    Path((record_type, record_id, share_id)): Path<(String, String, String)>,
    JsonBody(request): JsonBody<UpdateRecordShareRequest>,
) -> Result<Json<Data<RecordShareResponse>>, ApiError> {
    let share = record_sharing_use_cases(&state)
        .await?
        .update(
            &tenant.scope,
            &record_type,
            &record_id,
            &share_id,
            request.into_command(client.0, expected),
        )
        .await?;
    Ok(Json(Data::new(share.into())))
}

#[utoipa::path(
    delete,
    path = "/records/{record_type}/{record_id}/shares/{share_id}",
    tag = TAG,
    operation_id = "record_shares_delete",
    security(("session_cookie" = [])),
    description = "Removes a share and publishes `record.unshared`. Same rule as sharing: the owner of the record or `records.share`.",
    params(
        ("record_type" = String, Path, description = "Record type"),
        ("record_id" = String, Path, description = "Record id"),
        ("share_id" = String, Path, description = "Share id"),
    ),
    responses(
        (status = 204, description = "Share removed"),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = "Not the owner nor `records.share` (`record_share_not_allowed`)", body = ErrorEnvelope),
        (status = 404, description = "Record or share not found (`record_not_found`, `record_share_not_found`)", body = ErrorEnvelope),
    )
)]
async fn delete_record_share(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path((record_type, record_id, share_id)): Path<(String, String, String)>,
) -> Result<StatusCode, ApiError> {
    record_sharing_use_cases(&state)
        .await?
        .delete(&tenant.scope, &record_type, &record_id, &share_id, client.0)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
