use axum::Json;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde_json::json;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
use uuid::Uuid;

use crate::api::dto::common::{Data, ErrorEnvelope};
use crate::api::error::ApiError;
use crate::api::extractors::{ClientInfo, JsonBody};
use crate::errors::AppError;
use crate::tenancy::audit::{OperatorActor, record_operator_action};
use crate::tenancy::console::dto::{
    AcceptInvitationRequest, ChangePasswordRequest, InvitationDetailsResponse, InvitationResponse,
    InviteOperatorRequest, OperatorResponse, SignInRequest,
};
use crate::tenancy::console::operators::{self, Operator};
use crate::tenancy::console::{
    OperatorSession, Shared, read_cookie, request_origin, session_cookie,
};

const TAG: &str = "console";

pub(super) fn router() -> OpenApiRouter<Shared> {
    OpenApiRouter::new()
        .routes(routes!(current_session, sign_in, sign_out))
        .routes(routes!(change_password))
        .routes(routes!(list_operators, invite_operator))
        .routes(routes!(renew_invitation))
        .routes(routes!(deactivate_operator))
        .routes(routes!(invitation_details, accept_invitation))
}

pub(super) fn actor(session: &OperatorSession, client: &ClientInfo) -> OperatorActor {
    OperatorActor::operator(session.0.operator.id, client.0.ip.clone())
}

async fn signed_in(
    state: &Shared,
    operator: Operator,
    client: &ClientInfo,
    status: StatusCode,
) -> Result<Response, ApiError> {
    let issued = operators::issue_session(
        state.control.pool(),
        operator.id,
        client.0.ip.as_deref(),
        client.0.user_agent.as_deref(),
    )
    .await?;
    let cookie = session_cookie(
        issued.token.expose(),
        Some(issued.expires_at),
        state.secure_cookies,
    )?;
    let mut response = (status, Json(Data::new(OperatorResponse::from(operator)))).into_response();
    response.headers_mut().insert(header::SET_COOKIE, cookie);
    Ok(response)
}

#[utoipa::path(
    get,
    path = "/session",
    tag = TAG,
    operation_id = "console_session",
    responses(
        (status = 200, description = "The signed-in operator", body = Data<OperatorResponse>),
        (status = 401, description = "No console session (`console_session_required`)", body = ErrorEnvelope),
    )
)]
async fn current_session(session: OperatorSession) -> Json<Data<OperatorResponse>> {
    Json(Data::new(session.0.operator.into()))
}

#[utoipa::path(
    post,
    path = "/session",
    tag = TAG,
    operation_id = "console_sign_in",
    request_body = SignInRequest,
    responses(
        (status = 200, description = "Signed in; sets the `aphel_console_session` cookie", body = Data<OperatorResponse>),
        (status = 401, description = "Wrong email or password (`console_invalid_credentials`)", body = ErrorEnvelope),
        (status = 429, description = "Too many attempts (`rate_limited`) or the account is locked for a few minutes (`console_sign_in_locked`)", body = ErrorEnvelope),
    )
)]
async fn sign_in(
    State(state): State<Shared>,
    client: ClientInfo,
    JsonBody(request): JsonBody<SignInRequest>,
) -> Result<Response, ApiError> {
    state.sign_in_limiter.check(
        client.ip_or_unknown(),
        "too many sign-in attempts; wait a minute",
    )?;
    let operator =
        operators::authenticate(state.control.pool(), &request.email, &request.password).await?;
    record_operator_action(
        state.control.pool(),
        &OperatorActor::operator(operator.id, client.0.ip.clone()),
        "operator.signed_in",
        None,
        json!({}),
    )
    .await;
    signed_in(&state, operator, &client, StatusCode::OK).await
}

#[utoipa::path(
    delete,
    path = "/session",
    tag = TAG,
    operation_id = "console_sign_out",
    responses(
        (status = 204, description = "Signed out; clears the cookie"),
    )
)]
async fn sign_out(State(state): State<Shared>, headers: HeaderMap) -> Result<Response, ApiError> {
    if let Some(token) = read_cookie(&headers)
        && let Some(session) =
            operators::resolve_session(state.control.pool(), token.expose()).await?
    {
        operators::revoke_session(state.control.pool(), session.id).await?;
    }
    let mut response = StatusCode::NO_CONTENT.into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        session_cookie("", None, state.secure_cookies)?,
    );
    Ok(response)
}

#[utoipa::path(
    post,
    path = "/session/password",
    tag = TAG,
    operation_id = "console_change_password",
    request_body = ChangePasswordRequest,
    responses(
        (status = 204, description = "Password changed; the operator's other sessions are revoked"),
        (status = 401, description = "No console session", body = ErrorEnvelope),
        (status = 422, description = "Wrong current password (`operator_password_wrong`) or invalid new password (`operator_password_invalid`)", body = ErrorEnvelope),
    )
)]
async fn change_password(
    State(state): State<Shared>,
    session: OperatorSession,
    client: ClientInfo,
    JsonBody(request): JsonBody<ChangePasswordRequest>,
) -> Result<StatusCode, ApiError> {
    operators::change_password(
        state.control.pool(),
        &session.0,
        &request.current_password,
        &request.new_password,
    )
    .await?;
    record_operator_action(
        state.control.pool(),
        &actor(&session, &client),
        "operator.password_changed",
        None,
        json!({}),
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get,
    path = "/operators",
    tag = TAG,
    operation_id = "console_list_operators",
    responses(
        (status = 200, description = "Every operator, deactivated ones last", body = Data<Vec<OperatorResponse>>),
        (status = 401, description = "No console session", body = ErrorEnvelope),
    )
)]
async fn list_operators(
    State(state): State<Shared>,
    _session: OperatorSession,
) -> Result<Json<Data<Vec<OperatorResponse>>>, ApiError> {
    let operators = operators::list(state.control.pool()).await?;
    Ok(Json(Data::new(
        operators.into_iter().map(OperatorResponse::from).collect(),
    )))
}

fn invitation_response(
    state: &Shared,
    headers: &HeaderMap,
    operator: Operator,
    token: &str,
) -> InvitationResponse {
    InvitationResponse {
        operator: operator.into(),
        invite_url: format!(
            "{}/invite/{token}",
            request_origin(headers, state.secure_cookies)
        ),
    }
}

#[utoipa::path(
    post,
    path = "/operators",
    tag = TAG,
    operation_id = "console_invite_operator",
    request_body = InviteOperatorRequest,
    responses(
        (status = 201, description = "Operator invited; `invite_url` is shown once and valid for 7 days", body = Data<InvitationResponse>),
        (status = 401, description = "No console session", body = ErrorEnvelope),
        (status = 409, description = "Email already used (`operator_email_taken`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid email (`operator_email_invalid`) or name (`operator_name_invalid`)", body = ErrorEnvelope),
    )
)]
async fn invite_operator(
    State(state): State<Shared>,
    session: OperatorSession,
    client: ClientInfo,
    headers: HeaderMap,
    JsonBody(request): JsonBody<InviteOperatorRequest>,
) -> Result<(StatusCode, Json<Data<InvitationResponse>>), ApiError> {
    let (operator, token) =
        operators::invite(state.control.pool(), &request.email, &request.display_name).await?;
    record_operator_action(
        state.control.pool(),
        &actor(&session, &client),
        "operator.invited",
        None,
        json!({ "operator_id": operator.id, "email": operator.email }),
    )
    .await;
    Ok((
        StatusCode::CREATED,
        Json(Data::new(invitation_response(
            &state,
            &headers,
            operator,
            token.expose(),
        ))),
    ))
}

#[utoipa::path(
    post,
    path = "/operators/{id}/invitation",
    tag = TAG,
    operation_id = "console_renew_invitation",
    params(("id" = String, Path, description = "Operator id")),
    responses(
        (status = 200, description = "A new invitation link; the previous one stops working", body = Data<InvitationResponse>),
        (status = 401, description = "No console session", body = ErrorEnvelope),
        (status = 409, description = "The operator is not invited (`operator_not_invited`)", body = ErrorEnvelope),
    )
)]
async fn renew_invitation(
    State(state): State<Shared>,
    session: OperatorSession,
    client: ClientInfo,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<Data<InvitationResponse>>, ApiError> {
    let id = operator_id(&id)?;
    let (operator, token) = operators::renew_invitation(state.control.pool(), id).await?;
    record_operator_action(
        state.control.pool(),
        &actor(&session, &client),
        "operator.invitation_renewed",
        None,
        json!({ "operator_id": operator.id, "email": operator.email }),
    )
    .await;
    Ok(Json(Data::new(invitation_response(
        &state,
        &headers,
        operator,
        token.expose(),
    ))))
}

#[utoipa::path(
    post,
    path = "/operators/{id}/deactivate",
    tag = TAG,
    operation_id = "console_deactivate_operator",
    params(("id" = String, Path, description = "Operator id")),
    responses(
        (status = 200, description = "Operator deactivated and signed out everywhere", body = Data<OperatorResponse>),
        (status = 401, description = "No console session", body = ErrorEnvelope),
        (status = 404, description = "Unknown or already deactivated operator (`operator_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "Operators cannot deactivate themselves (`operator_self_deactivation`)", body = ErrorEnvelope),
    )
)]
async fn deactivate_operator(
    State(state): State<Shared>,
    session: OperatorSession,
    client: ClientInfo,
    Path(id): Path<String>,
) -> Result<Json<Data<OperatorResponse>>, ApiError> {
    let id = operator_id(&id)?;
    let operator = operators::deactivate(state.control.pool(), &session.0.operator, id).await?;
    record_operator_action(
        state.control.pool(),
        &actor(&session, &client),
        "operator.deactivated",
        None,
        json!({ "operator_id": operator.id, "email": operator.email }),
    )
    .await;
    Ok(Json(Data::new(operator.into())))
}

#[utoipa::path(
    get,
    path = "/invitations/{token}",
    tag = TAG,
    operation_id = "console_invitation",
    params(("token" = String, Path, description = "Token from the invitation link")),
    responses(
        (status = 200, description = "Who the invitation is for", body = Data<InvitationDetailsResponse>),
        (status = 404, description = "Invalid or expired link (`operator_invitation_invalid`)", body = ErrorEnvelope),
    )
)]
async fn invitation_details(
    State(state): State<Shared>,
    Path(token): Path<String>,
) -> Result<Json<Data<InvitationDetailsResponse>>, ApiError> {
    let operator = operators::invitation(state.control.pool(), &token).await?;
    Ok(Json(Data::new(InvitationDetailsResponse {
        email: operator.email,
        display_name: operator.display_name,
    })))
}

#[utoipa::path(
    post,
    path = "/invitations/{token}",
    tag = TAG,
    operation_id = "console_accept_invitation",
    params(("token" = String, Path, description = "Token from the invitation link")),
    request_body = AcceptInvitationRequest,
    responses(
        (status = 201, description = "Password chosen, operator active and signed in", body = Data<OperatorResponse>),
        (status = 404, description = "Invalid or expired link (`operator_invitation_invalid`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid password (`operator_password_invalid`)", body = ErrorEnvelope),
    )
)]
async fn accept_invitation(
    State(state): State<Shared>,
    client: ClientInfo,
    Path(token): Path<String>,
    JsonBody(request): JsonBody<AcceptInvitationRequest>,
) -> Result<Response, ApiError> {
    let operator =
        operators::accept_invitation(state.control.pool(), &token, &request.password).await?;
    record_operator_action(
        state.control.pool(),
        &OperatorActor::operator(operator.id, client.0.ip.clone()),
        "operator.invitation_accepted",
        None,
        json!({ "email": operator.email }),
    )
    .await;
    signed_in(&state, operator, &client, StatusCode::CREATED).await
}

fn operator_id(raw: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(raw)
        .map_err(|_| AppError::not_found_code("operator_not_found", "the operator was not found"))
}
