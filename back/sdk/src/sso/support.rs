use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use serde_json::Value;
use uuid::Uuid;

use aphel_kernel::Ctx;
use aphel_kernel::api::rate_limit::{SlidingWindowLimiter, shared_hit};
use aphel_kernel::application::access::{AccessRepository, build_access_scope};
use aphel_kernel::application::auth::UserRepository;
use aphel_kernel::database::transaction::Tx;
use aphel_kernel::domain::access::{AccessScope, Permission};
use aphel_kernel::domain::{Secret, UserId};
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::repositories::mysql_access::MySqlAccessRepository;
use aphel_kernel::infrastructure::repositories::mysql_users::MySqlUserRepository;
use aphel_kernel::modules::{module_secret_key, module_settings};

pub const API_PREFIX: &str = "/api/v1";
pub const PUBLIC_PREFIX: &str = "/api/v1/public";

pub fn api_path(key: &str) -> String {
    format!("{API_PREFIX}/{key}")
}

pub fn public_path(key: &str) -> String {
    format!("{PUBLIC_PREFIX}/{key}")
}

pub fn app_base_url(ctx: &Ctx<'_>) -> String {
    ctx.app_base_url()
}

pub async fn settings(ctx: &Ctx<'_>, key: &str) -> Result<Value, AppError> {
    module_settings(ctx.pool(), key).await
}

pub async fn secret(ctx: &Ctx<'_>, key: &str, name: &str) -> Result<Option<String>, AppError> {
    let secret = ctx
        .secrets()?
        .reveal(&AccessScope::system(), &module_secret_key(key, name))
        .await?;
    Ok(secret.map(|secret| secret.expose().to_owned()))
}

pub async fn store_secret(
    ctx: &Ctx<'_>,
    tx: &mut Tx,
    key: &str,
    name: &str,
    value: &str,
) -> Result<(), AppError> {
    ctx.secrets()?
        .store(
            tx,
            ctx.scope,
            &module_secret_key(key, name),
            &Secret::new(value.to_owned()),
        )
        .await
}

pub async fn remove_secret(
    ctx: &Ctx<'_>,
    tx: &mut Tx,
    key: &str,
    name: &str,
) -> Result<(), AppError> {
    ctx.secrets()?
        .remove(tx, ctx.scope, &module_secret_key(key, name))
        .await
}

pub fn user_id(id: Uuid) -> Result<UserId, AppError> {
    UserId::new(id.to_string()).map_err(AppError::from)
}

pub fn scope_user(scope: &AccessScope) -> Result<Uuid, AppError> {
    Uuid::parse_str(scope.user_id().as_str())
        .map_err(|error| AppError::Internal(format!("the acting user id is not a UUID: {error}")))
}

pub async fn user_email(ctx: &Ctx<'_>, id: Uuid) -> Result<Option<String>, AppError> {
    let user = MySqlUserRepository::new(ctx.pool().clone())
        .find_by_id(&user_id(id)?)
        .await?;
    Ok(user.map(|user| user.email.as_str().to_owned()))
}

pub async fn scope_for_user(
    ctx: &Ctx<'_>,
    key: &str,
    id: Uuid,
) -> Result<Option<AccessScope>, AppError> {
    let user = user_id(id)?;
    let Some(access) = MySqlAccessRepository::new(ctx.pool().clone())
        .load_access(&user)
        .await?
    else {
        return Ok(None);
    };
    let scope = build_access_scope(user, access);
    Ok(scope.module_enabled(key).then_some(scope))
}

pub fn can(scope: &AccessScope, permission: &Permission) -> bool {
    scope.scope_for(permission).is_some()
}

pub async fn rate_limit(
    key: &str,
    bucket: &str,
    limit: u32,
    window: Duration,
) -> Result<(), AppError> {
    type Limiters = Mutex<HashMap<String, Arc<SlidingWindowLimiter>>>;
    static LIMITERS: OnceLock<Limiters> = OnceLock::new();
    let limiter = {
        let mut limiters = LIMITERS
            .get_or_init(|| Mutex::new(HashMap::new()))
            .lock()
            .map_err(|_| AppError::Internal("rate limiter lock poisoned".to_owned()))?;
        limiters
            .entry(format!("{key}:{limit}:{}", window.as_secs()))
            .or_insert_with(|| Arc::new(SlidingWindowLimiter::new(limit, window)))
            .clone()
    };
    let decision = shared_hit(
        &format!("{key}.sso"),
        &format!("{key}:{bucket}"),
        limit,
        window,
        &limiter,
    )
    .await?;
    if decision.allowed {
        Ok(())
    } else {
        Err(decision.rejection("too many requests, try again later"))
    }
}
