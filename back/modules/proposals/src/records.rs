use std::str::FromStr;

use bytes::Bytes;
use chrono_tz::Tz;
use serde_json::Value;
use uuid::Uuid;

use aphel_kernel::api::routes::v1::files::file_use_cases;
use aphel_kernel::application::auth::UserRepository;
use aphel_kernel::application::files::UploadFileCommand;
use aphel_kernel::application::shared::{Actor, RecordRef};
use aphel_kernel::context::Ctx;
use aphel_kernel::domain::UserId;
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::{AppError, CodedKind};
use aphel_kernel::infrastructure::repositories::mysql_organizations::find_organization;
use aphel_kernel::infrastructure::repositories::mysql_users::MySqlUserRepository;
use aphel_kernel::infrastructure::storage::references::delete_unreferenced_files;
use aphel_kernel::records::{RecordCtx, field_set};
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

pub(crate) struct UploadedFile {
    pub(crate) id: Uuid,
    pub(crate) attachment_id: Option<Uuid>,
    pub(crate) file_name: String,
    pub(crate) size: u64,
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
    let fields = field_set(&ctx.state(), ctx.scope, record_type).await?;
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

pub(crate) async fn upload(
    ctx: &Ctx<'_>,
    file_name: String,
    content: Vec<u8>,
    attach_to: RecordRef,
) -> Result<UploadedFile, AppError> {
    let uploaded = file_use_cases(&ctx.state())
        .await?
        .upload(
            ctx.scope,
            UploadFileCommand {
                file_name,
                content: Bytes::from(content),
                attach_to: Some(attach_to),
                ip: None,
            },
        )
        .await?;
    Ok(UploadedFile {
        id: uploaded.file.id,
        attachment_id: uploaded.attachment.as_ref().map(|attachment| attachment.id),
        file_name: uploaded.file.original_name,
        size: uploaded.file.size,
    })
}

pub(crate) async fn discard_upload(ctx: &Ctx<'_>, file: &UploadedFile) {
    if let Err(error) = try_discard_upload(ctx, file).await {
        tracing::warn!(%error, file_id = %file.id, "could not discard an unused proposal file");
    }
}

async fn try_discard_upload(ctx: &Ctx<'_>, file: &UploadedFile) -> Result<(), AppError> {
    if let Some(attachment_id) = file.attachment_id {
        file_use_cases(&ctx.state())
            .await?
            .remove_attachment(ctx.scope, &attachment_id.to_string(), None)
            .await?;
    }
    let mut tx = ctx.begin().await?;
    let storage_keys = delete_unreferenced_files(&mut tx, &[file.id]).await?;
    tx.commit().await?;
    let storage = ctx.storage()?;
    for key in storage_keys {
        storage.delete(&key).await?;
    }
    Ok(())
}
