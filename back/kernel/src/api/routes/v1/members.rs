use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde::Serialize;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::api::dto::common::{Data, ErrorEnvelope, Page};
use crate::api::dto::members::{ChangeMemberRoleRequest, InviteMemberRequest, MemberResponse};
use crate::api::error::ApiError;
use crate::api::extractors::{
    AuthContext, ClientInfo, IfMatch, JsonBody, Pagination, PaginationQuery,
};
use crate::api::state::AppState;
use crate::application::mail::{SharedMailer, UndeliveredMailCapture};
use crate::application::organization_admin::{ListWindow, OrganizationAdminUseCases};
use crate::errors::AppError;
use crate::infrastructure::repositories::mysql_organization_admin::MySqlOrganizationAdminPorts;

pub const TAG: &str = "members";

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_members))
        .routes(routes!(invite_member))
        .routes(routes!(get_member, change_member_role, remove_member))
        .routes(routes!(resend_invitation))
        .routes(routes!(password_reset_link))
        .routes(routes!(cancel_invitation))
        .routes(routes!(suspend_member))
        .routes(routes!(reactivate_member))
}

pub fn admin_use_cases(
    state: &AppState,
) -> Result<OrganizationAdminUseCases<MySqlOrganizationAdminPorts>, AppError> {
    Ok(OrganizationAdminUseCases::new(
        MySqlOrganizationAdminPorts::new(
            state.db()?,
            state.mailer(),
            state.events.clone(),
            state.audit.clone(),
        ),
        state.auth_settings.as_ref().clone(),
    )
    .with_security_events(state.security_events.clone()))
}

#[derive(Debug, Serialize, ToSchema)]
pub struct InvitationResponse {
    #[serde(flatten)]
    pub member: MemberResponse,
    #[schema(example = "https://app.example.com/accept-invitation?token=...")]
    pub invitation_link: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PasswordResetLinkResponse {
    pub emailed: bool,
    pub link: Option<String>,
}

fn invitation_use_cases(
    state: &AppState,
    mailer: SharedMailer,
) -> Result<OrganizationAdminUseCases<MySqlOrganizationAdminPorts>, AppError> {
    Ok(OrganizationAdminUseCases::new(
        MySqlOrganizationAdminPorts::new(
            state.db()?,
            mailer,
            state.events.clone(),
            state.audit.clone(),
        ),
        state.auth_settings.as_ref().clone(),
    ))
}

pub fn window(pagination: Pagination) -> ListWindow {
    ListWindow {
        offset: pagination.offset(),
        limit: pagination.limit(),
    }
}

#[utoipa::path(
    get,
    path = "/members",
    tag = TAG,
    operation_id = "members_list",
    security(("session_cookie" = [])),
    params(PaginationQuery),
    responses(
        (status = 200, description = "Members and pending invitations", body = Page<MemberResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `users.manage` and `settings.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid pagination (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn list_members(
    State(state): State<AppState>,
    tenant: AuthContext,
    pagination: Pagination,
) -> Result<Json<Page<MemberResponse>>, ApiError> {
    let listing = admin_use_cases(&state)?
        .list_members(&tenant.scope, window(pagination))
        .await?;
    Ok(Json(Page::new(
        listing.items.into_iter().map(Into::into).collect(),
        listing.total,
        pagination,
    )))
}

#[utoipa::path(
    post,
    path = "/members/invitations",
    tag = TAG,
    operation_id = "members_invite",
    security(("session_cookie" = [])),
    request_body = InviteMemberRequest,
    responses(
        (status = 201, description = "Invitation created and email sent. When no outgoing email is configured (`mail_not_configured`), `invitation_link` carries the link for the administrator to copy", body = Data<InvitationResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `users.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 409, description = "The user already belongs to the organization (`already_member`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid fields (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn invite_member(
    State(state): State<AppState>,
    tenant: AuthContext,
    JsonBody(request): JsonBody<InviteMemberRequest>,
) -> Result<(StatusCode, Json<Data<InvitationResponse>>), ApiError> {
    let capture = UndeliveredMailCapture::new(state.mailer());
    let member = invitation_use_cases(&state, capture.shared())?
        .invite_member(&tenant.scope, request.into())
        .await?;
    Ok((
        StatusCode::CREATED,
        Json(Data::new(InvitationResponse {
            member: member.into(),
            invitation_link: invitation_link(&state, &capture),
        })),
    ))
}

fn invitation_link(state: &AppState, capture: &UndeliveredMailCapture) -> Option<String> {
    capture.undelivered_link(
        &state
            .auth_settings
            .app_base_url
            .link("/accept-invitation?token="),
    )
}

#[utoipa::path(
    get,
    path = "/members/{user_id}",
    tag = TAG,
    operation_id = "members_get",
    security(("session_cookie" = [])),
    params(("user_id" = String, Path, description = "User id of the member")),
    responses(
        (status = 200, description = "Member", body = Data<MemberResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `users.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Member not found (`member_not_found`)", body = ErrorEnvelope),
    )
)]
async fn get_member(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(user_id): Path<String>,
) -> Result<Json<Data<MemberResponse>>, ApiError> {
    let member = admin_use_cases(&state)?
        .get_member(&tenant.scope, &user_id)
        .await?;
    Ok(Json(Data::new(member.into())))
}

#[utoipa::path(
    patch,
    path = "/members/{user_id}",
    tag = TAG,
    operation_id = "members_change_role",
    security(("session_cookie" = [])),
    params(
        ("user_id" = String, Path, description = "User id of the member"),
        ("If-Match" = Option<String>, Header, description = "`updated_at` of the member as last read; when it changed since, answers 409 `version_conflict`"),
    ),
    request_body = ChangeMemberRoleRequest,
    responses(
        (status = 200, description = "Role changed", body = Data<MemberResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `users.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Member not found (`member_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The organization would be left without an administrator (`last_admin`) or the member changed since `If-Match` (`version_conflict`)", body = ErrorEnvelope),
        (status = 422, description = "Unknown or non-assignable `role_id` (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn change_member_role(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(user_id): Path<String>,
    client: ClientInfo,
    IfMatch(expected_updated_at): IfMatch,
    JsonBody(request): JsonBody<ChangeMemberRoleRequest>,
) -> Result<Json<Data<MemberResponse>>, ApiError> {
    let mut command = request.into_command(expected_updated_at);
    command.client = client.0;
    let member = admin_use_cases(&state)?
        .change_member_role(&tenant.scope, &user_id, command)
        .await?;
    Ok(Json(Data::new(member.into())))
}

#[utoipa::path(
    delete,
    path = "/members/{user_id}",
    tag = TAG,
    operation_id = "members_remove",
    security(("session_cookie" = [])),
    params(("user_id" = String, Path, description = "User id of the member")),
    responses(
        (status = 204, description = "Member deactivated (removed from teams) or pending invitation deleted, and signed out"),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `users.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Member not found (`member_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The organization would be left without an administrator (`last_admin`)", body = ErrorEnvelope),
    )
)]
async fn remove_member(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(user_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    admin_use_cases(&state)?
        .remove_member(&tenant.scope, &user_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/members/{user_id}/resend-invitation",
    tag = TAG,
    operation_id = "members_resend_invitation",
    security(("session_cookie" = [])),
    params(("user_id" = String, Path, description = "User id of the member")),
    responses(
        (status = 200, description = "Previous invitation links invalidated and a new email sent. Without outgoing email (`mail_not_configured`), `invitation_link` carries the new link", body = Data<InvitationResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `users.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Member not found (`member_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The member has no pending invitation (`not_invited`)", body = ErrorEnvelope),
    )
)]
async fn resend_invitation(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(user_id): Path<String>,
) -> Result<Json<Data<InvitationResponse>>, ApiError> {
    let capture = UndeliveredMailCapture::new(state.mailer());
    let member = invitation_use_cases(&state, capture.shared())?
        .resend_invitation(&tenant.scope, &user_id)
        .await?;
    Ok(Json(Data::new(InvitationResponse {
        member: member.into(),
        invitation_link: invitation_link(&state, &capture),
    })))
}

#[utoipa::path(
    post,
    path = "/members/{user_id}/password-reset",
    tag = TAG,
    operation_id = "members_password_reset",
    description = "Starts a password reset for an active member (permission `users.manage`): previous reset links are invalidated and the email is sent. When no outgoing email is configured, nothing is emailed (`emailed = false`) and `link` carries the reset link for the administrator to hand over. Members without a password (single sign-on only) or not active get neither.",
    security(("session_cookie" = [])),
    params(("user_id" = String, Path, description = "User id of the member")),
    responses(
        (status = 200, description = "Reset started", body = Data<PasswordResetLinkResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `users.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Member not found (`member_not_found`)", body = ErrorEnvelope),
    )
)]
async fn password_reset_link(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(user_id): Path<String>,
) -> Result<Json<Data<PasswordResetLinkResponse>>, ApiError> {
    let member = admin_use_cases(&state)?
        .get_member(&tenant.scope, &user_id)
        .await?;
    let capture = UndeliveredMailCapture::new(state.mailer());
    let state = state.clone().with_mailer(capture.shared());
    state
        .auth_use_cases()?
        .request_password_reset(&member.email)
        .await?;
    let link = capture.undelivered_link(
        &state
            .auth_settings
            .app_base_url
            .link("/reset-password?token="),
    );
    Ok(Json(Data::new(PasswordResetLinkResponse {
        emailed: link.is_none() && capture.undelivered().is_empty(),
        link,
    })))
}

#[utoipa::path(
    delete,
    path = "/members/{user_id}/invitation",
    tag = TAG,
    operation_id = "members_cancel_invitation",
    security(("session_cookie" = [])),
    params(("user_id" = String, Path, description = "User id of the member")),
    responses(
        (status = 204, description = "Invitation cancelled"),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `users.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Member not found (`member_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The member has no pending invitation (`not_invited`)", body = ErrorEnvelope),
    )
)]
async fn cancel_invitation(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(user_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    admin_use_cases(&state)?
        .cancel_invitation(&tenant.scope, &user_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/members/{user_id}/suspend",
    tag = TAG,
    operation_id = "members_suspend",
    security(("session_cookie" = [])),
    params(("user_id" = String, Path, description = "User id of the member")),
    responses(
        (status = 200, description = "Member suspended and signed out", body = Data<MemberResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `users.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Member not found (`member_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "Member is not active (`invalid_member_status`) or is the last administrator (`last_admin`)", body = ErrorEnvelope),
    )
)]
async fn suspend_member(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(user_id): Path<String>,
) -> Result<Json<Data<MemberResponse>>, ApiError> {
    let member = admin_use_cases(&state)?
        .suspend_member(&tenant.scope, &user_id)
        .await?;
    Ok(Json(Data::new(member.into())))
}

#[utoipa::path(
    post,
    path = "/members/{user_id}/reactivate",
    tag = TAG,
    operation_id = "members_reactivate",
    security(("session_cookie" = [])),
    params(("user_id" = String, Path, description = "User id of the member")),
    responses(
        (status = 200, description = "Member reactivated", body = Data<MemberResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `users.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Member not found (`member_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "Member is not suspended (`invalid_member_status`)", body = ErrorEnvelope),
    )
)]
async fn reactivate_member(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(user_id): Path<String>,
) -> Result<Json<Data<MemberResponse>>, ApiError> {
    let member = admin_use_cases(&state)?
        .reactivate_member(&tenant.scope, &user_id)
        .await?;
    Ok(Json(Data::new(member.into())))
}
