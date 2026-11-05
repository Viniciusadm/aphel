use chrono::Utc;
use uuid::Uuid;

use crate::support;
use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::api::session_cookie;
use aphel_sdk::platform::application::access::AccessRepository;
use aphel_sdk::platform::application::auth::{ClientMetadata, LoginOutcome};
use aphel_sdk::platform::infrastructure::repositories::mysql_access::MySqlAccessRepository;

pub(crate) const NOT_A_MEMBER: &str = "not_a_member";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SignInRequest {
    pub(crate) user_id: Uuid,
    pub(crate) method: String,
    pub(crate) ip: Option<String>,
    pub(crate) user_agent: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SignedIn {
    pub(crate) set_cookie: String,
    pub(crate) two_factor_required: bool,
}

pub(crate) async fn sign_in(
    ctx: &Ctx<'_>,
    key: &str,
    request: SignInRequest,
) -> Result<SignedIn, AppError> {
    let user = support::user_id(request.user_id)?;
    if MySqlAccessRepository::new(ctx.pool().clone())
        .load_access(&user)
        .await?
        .is_none()
    {
        return Err(AppError::forbidden_code(
            NOT_A_MEMBER,
            "the user is not an active member of the organization",
        ));
    }
    tracing::info!(
        module = key,
        method = %request.method,
        user_id = %request.user_id,
        "external sign-in"
    );
    let state = ctx.state();
    let outcome = state
        .auth_use_cases()?
        .sign_in_external(
            &user,
            ClientMetadata {
                ip: request.ip,
                user_agent: request.user_agent,
            },
        )
        .await?;
    let (token, expires_at, two_factor_required) = match outcome {
        LoginOutcome::Session(session) => (session.token, session.expires_at, false),
        LoginOutcome::TwoFactorRequired(pending) => (pending.token, pending.expires_at, true),
    };
    let cookie = session_cookie::issue(&token, expires_at, Utc::now(), state.secure_cookies())?;
    Ok(SignedIn {
        set_cookie: cookie
            .to_str()
            .map_err(|error| AppError::Internal(error.to_string()))?
            .to_owned(),
        two_factor_required,
    })
}
