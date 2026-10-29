use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::{HeaderMap, header};
use uuid::Uuid;

use crate::api::error::ApiError;
use crate::api::routes::v1::api_tokens::api_token_use_cases;
use crate::api::session_cookie;
use crate::api::state::AppState;
use crate::application::auth::two_factor::two_factor_setup_required;
use crate::application::auth::{AuthenticatedRequest, ClientMetadata};
use crate::domain::SessionId;
use crate::domain::access::AccessScope;
use crate::domain::users::User;
use crate::errors::AppError;

pub const API_TOKEN_NOT_ALLOWED: &str = "api_token_not_allowed";
const MAX_USER_AGENT_LENGTH: usize = 255;

#[derive(Debug, Clone)]
pub struct SessionContext(pub AuthenticatedRequest);

impl SessionContext {
    pub fn user(&self) -> &User {
        &self.0.user
    }

    pub fn session_id(&self) -> &SessionId {
        &self.0.session_id
    }

    pub fn ensure_two_factor_compliant(&self) -> Result<(), AppError> {
        if self.0.two_factor.blocks_organization() {
            return Err(two_factor_setup_required());
        }
        Ok(())
    }
}

impl FromRequestParts<AppState> for SessionContext {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        if let Some(context) = parts.extensions.get::<SessionContext>() {
            return Ok(context.clone());
        }
        if bearer_token(&parts.headers).is_some() {
            return Err(AppError::forbidden_code(
                API_TOKEN_NOT_ALLOWED,
                "this route only accepts a browser session, not an API token",
            )
            .into());
        }

        let token = session_cookie::read(&parts.headers)
            .ok_or_else(|| AppError::Unauthorized("missing session".to_owned()))?;
        let request = state.auth_use_cases()?.authenticate(token.expose()).await?;
        let context = SessionContext(request);
        parts.extensions.insert(context.clone());
        Ok(context)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Credential {
    Session(SessionId),
    ApiToken(Uuid),
}

#[derive(Debug, Clone)]
pub struct AuthContext {
    pub user: User,
    pub credential: Credential,
    pub scope: AccessScope,
}

impl AuthContext {
    pub fn user(&self) -> &User {
        &self.user
    }
}

impl FromRequestParts<AppState> for AuthContext {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        if let Some(context) = parts.extensions.get::<AuthContext>() {
            return Ok(context.clone());
        }

        let context = match bearer_token(&parts.headers) {
            Some(token) => {
                let ip = crate::api::client_address::client_ip(parts);
                let authentication = api_token_use_cases(state)?
                    .authenticate(&token, ip.as_deref())
                    .await?;
                AuthContext {
                    user: authentication.user,
                    credential: Credential::ApiToken(authentication.token_id),
                    scope: authentication.scope,
                }
            }
            None => {
                let SessionContext(request) =
                    SessionContext::from_request_parts(parts, state).await?;
                if request.two_factor.blocks_organization() {
                    return Err(two_factor_setup_required().into());
                }
                AuthContext {
                    user: request.user,
                    credential: Credential::Session(request.session_id),
                    scope: request.scope,
                }
            }
        };
        if !context.user.is_active() {
            return Err(AppError::Unauthorized("the account is not active".to_owned()).into());
        }
        parts.extensions.insert(context.clone());
        Ok(context)
    }
}

pub fn bearer_token(headers: &HeaderMap) -> Option<String> {
    let value = headers.get(header::AUTHORIZATION)?.to_str().ok()?;
    let (scheme, token) = value.trim().split_once(' ')?;
    scheme
        .eq_ignore_ascii_case("bearer")
        .then(|| token.trim().to_owned())
}

#[derive(Debug, Clone, Default)]
pub struct ClientInfo(pub ClientMetadata);

impl ClientInfo {
    pub fn ip_or_unknown(&self) -> &str {
        self.0.ip.as_deref().unwrap_or("unknown")
    }
}

impl<S> FromRequestParts<S> for ClientInfo
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let ip = crate::api::client_address::client_ip(parts);
        let user_agent = parts
            .headers
            .get(header::USER_AGENT)
            .and_then(|value| value.to_str().ok())
            .map(|value| value.chars().take(MAX_USER_AGENT_LENGTH).collect());
        Ok(Self(ClientMetadata { ip, user_agent }))
    }
}
