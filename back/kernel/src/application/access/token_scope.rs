use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::application::access::{AccessRepository, build_access_scope};
use crate::application::api_tokens::ApiTokenRepository;
use crate::domain::UserId;
use crate::domain::access::AccessScope;
use crate::errors::AppError;

pub async fn token_scope<T: ApiTokenRepository>(
    tokens: &T,
    access: &dyn AccessRepository,
    token_id: Uuid,
    now: DateTime<Utc>,
) -> Result<Option<AccessScope>, AppError> {
    let Some(token) = tokens
        .find(token_id)
        .await?
        .filter(|token| token.is_usable_at(now))
    else {
        return Ok(None);
    };
    let Some(grants) = access.load_access(&token.created_by).await? else {
        return Ok(None);
    };
    Ok(Some(
        build_access_scope(token.created_by.clone(), grants)
            .through_api_token(token.id.to_string(), token.scopes.clone()),
    ))
}

pub async fn requester_scope<T: ApiTokenRepository>(
    tokens: &T,
    access: &dyn AccessRepository,
    user_id: &UserId,
    api_token_id: Option<Uuid>,
    now: DateTime<Utc>,
) -> Result<Option<AccessScope>, AppError> {
    let scope = match api_token_id {
        Some(token_id) => token_scope(tokens, access, token_id, now).await?,
        None => access
            .load_access(user_id)
            .await?
            .map(|grants| build_access_scope(user_id.clone(), grants)),
    };
    Ok(scope.filter(|scope| scope.user_id() == user_id))
}
