use serde_json::Value;
use uuid::Uuid;

use crate::store::StoredChannel;
use crate::{KEY, SETTING_ACTING_USER, SETTING_CREATE_CONTACTS, settings};
use aphel_kernel::Ctx;
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::access::{AccessRepository, build_access_scope};
use aphel_sdk::platform::application::auth::UserRepository;
use aphel_sdk::platform::domain::access::AccessScope;
use aphel_sdk::platform::infrastructure::repositories::mysql_access::MySqlAccessRepository;
use aphel_sdk::platform::infrastructure::repositories::mysql_users::MySqlUserRepository;

pub(crate) async fn scope_for_user(
    ctx: &Ctx<'_>,
    user_id: Uuid,
) -> Result<Option<AccessScope>, AppError> {
    let user = UserId::new(user_id.to_string())?;
    let Some(access) = MySqlAccessRepository::new(ctx.pool().clone())
        .load_access(&user)
        .await?
    else {
        return Ok(None);
    };
    let scope = build_access_scope(user, access);
    Ok(scope.module_enabled(KEY).then_some(scope))
}

async fn active_user_by_email(ctx: &Ctx<'_>, email: &str) -> Result<Option<Uuid>, AppError> {
    let user = MySqlUserRepository::new(ctx.pool().clone())
        .find_by_email(&email.trim().to_lowercase())
        .await?;
    Ok(user
        .filter(|user| user.is_active())
        .and_then(|user| Uuid::parse_str(user.id.as_str()).ok()))
}

pub(crate) async fn acting_scope(
    ctx: &Ctx<'_>,
    channel: &StoredChannel,
) -> Result<Option<AccessScope>, AppError> {
    if let Some(user_id) = channel.acting_user_id
        && let Some(scope) = scope_for_user(ctx, user_id).await?
    {
        return Ok(Some(scope));
    }
    let settings = settings(ctx).await?;
    if let Some(email) = settings
        .get(SETTING_ACTING_USER)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|email| !email.is_empty())
        && let Some(user_id) = active_user_by_email(ctx, email).await?
        && let Some(scope) = scope_for_user(ctx, user_id).await?
    {
        return Ok(Some(scope));
    }
    scope_for_user(ctx, channel.created_by).await
}

pub(crate) async fn user_scope_or_actor(
    ctx: &Ctx<'_>,
    channel: &StoredChannel,
    user_id: Option<Uuid>,
) -> Result<Option<AccessScope>, AppError> {
    if let Some(user_id) = user_id
        && let Some(scope) = scope_for_user(ctx, user_id).await?
    {
        return Ok(Some(scope));
    }
    acting_scope(ctx, channel).await
}

pub(crate) async fn creates_unknown_contacts(ctx: &Ctx<'_>) -> Result<bool, AppError> {
    let settings = settings(ctx).await?;
    Ok(settings
        .get(SETTING_CREATE_CONTACTS)
        .and_then(Value::as_bool)
        .unwrap_or(true))
}
