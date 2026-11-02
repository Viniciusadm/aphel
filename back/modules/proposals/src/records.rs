use std::str::FromStr;

use chrono_tz::Tz;
use serde_json::Value;
use uuid::Uuid;

use aphel_kernel::application::auth::UserRepository;
use aphel_kernel::application::shared::Actor;
use aphel_kernel::context::Ctx;
use aphel_kernel::domain::UserId;
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::{AppError, CodedKind};
use aphel_kernel::infrastructure::repositories::mysql_organizations::find_organization;
use aphel_kernel::infrastructure::repositories::mysql_users::MySqlUserRepository;
use aphel_kernel::records::RecordCtx;
use deal_items::http::DealItemsResponse;

use crate::format::Locale;

pub(crate) struct FieldInfo {
    pub(crate) key: String,
    pub(crate) label: String,
    pub(crate) field_type: String,
}

pub(crate) struct OrganizationInfo {
    pub(crate) name: String,
    pub(crate) timezone: Tz,
    pub(crate) currency: String,
    pub(crate) locale: Locale,
}

pub(crate) fn is_tolerated(error: &AppError) -> bool {
    matches!(
        error,
        AppError::NotFound(_)
            | AppError::Forbidden(_)
            | AppError::Coded {
                kind: CodedKind::NotFound | CodedKind::Forbidden,
                ..
            }
            | AppError::CodedDetails {
                kind: CodedKind::NotFound | CodedKind::Forbidden,
                ..
            }
    )
}

pub(crate) fn is_validation(error: &AppError) -> bool {
    matches!(
        error,
        AppError::Validation(_)
            | AppError::InvalidFields { .. }
            | AppError::CodedFields { .. }
            | AppError::Coded {
                kind: CodedKind::Validation,
                ..
            }
            | AppError::CodedDetails {
                kind: CodedKind::Validation,
                ..
            }
    )
}

pub(crate) fn actor_id(scope: &AccessScope) -> Uuid {
    if scope.is_system() {
        return Uuid::nil();
    }
    Uuid::parse_str(scope.user_id().as_str()).unwrap_or_else(|_| Uuid::nil())
}

pub(crate) fn actor_kind(scope: &AccessScope) -> &'static str {
    Actor::from_scope(scope).kind()
}

pub(crate) async fn fields(ctx: &Ctx<'_>, record_type: &str) -> Result<Vec<FieldInfo>, AppError> {
    let fields = ctx.field_set(ctx.scope, record_type).await?;
    Ok(fields
        .active()
        .map(|field| FieldInfo {
            key: field.key().to_owned(),
            label: field.definition.label.clone(),
            field_type: field.definition.type_name().to_owned(),
        })
        .collect())
}

pub(crate) async fn load(
    ctx: &Ctx<'_>,
    record_type: &str,
    id: Uuid,
) -> Result<Option<Value>, AppError> {
    let Ok(handler) = ctx.registry().handler(record_type) else {
        return Ok(None);
    };
    let state = ctx.state();
    match handler.load(&RecordCtx::new(ctx.scope, &state), id).await {
        Ok(value) => Ok(value),
        Err(error) if is_tolerated(&error) => Ok(None),
        Err(error) => Err(error),
    }
}

pub(crate) async fn organization(ctx: &Ctx<'_>) -> Result<OrganizationInfo, AppError> {
    let organization = find_organization(ctx.pool()).await?.ok_or_else(|| {
        AppError::not_found_code("organization_not_found", "organization not found")
    })?;
    Ok(OrganizationInfo {
        name: organization.name.as_str().to_owned(),
        timezone: Tz::from_str(&organization.timezone).unwrap_or(chrono_tz::America::Sao_Paulo),
        currency: organization.currency,
        locale: Locale::parse(&organization.locale),
    })
}

pub(crate) async fn user_identity(ctx: &Ctx<'_>) -> (Option<String>, Option<String>) {
    if ctx.scope.is_system() {
        return (None, None);
    }
    let Ok(id) = UserId::new(ctx.scope.user_id().as_str().to_owned()) else {
        return (None, None);
    };
    match MySqlUserRepository::new(ctx.pool().clone())
        .find_by_id(&id)
        .await
    {
        Ok(Some(user)) => (
            Some(user.display_name.as_str().to_owned()),
            Some(user.email.as_str().to_owned()),
        ),
        _ => (None, None),
    }
}

pub(crate) async fn quote(ctx: &Ctx<'_>, quote_id: Uuid) -> Result<Value, AppError> {
    if !ctx.app().is_enabled(quotes::KEY) {
        return Err(aphel_kernel::modules::module_disabled());
    }
    let details = quotes::service::get(ctx, &quote_id.to_string()).await?;
    quotes::http::quote_detail(&ctx.state(), ctx.scope, details).await
}

pub(crate) async fn deal_items(ctx: &Ctx<'_>, deal_id: Uuid) -> Result<Value, AppError> {
    if !ctx.app().is_enabled(deal_items::KEY) {
        return Err(aphel_kernel::modules::module_disabled());
    }
    let summary = deal_items::service::summary(ctx, &deal_id.to_string()).await?;
    serde_json::to_value(DealItemsResponse::from(summary))
        .map_err(|error| AppError::Internal(error.to_string()))
}
