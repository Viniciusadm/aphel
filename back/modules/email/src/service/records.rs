use serde_json::Value;

use crate::compose::AttachmentFile;
use crate::templates::VariableField;
use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::access::PERMISSION_DENIED;
use aphel_sdk::platform::application::files::{FileSettings, FileUseCases, RecordAccessChecker};
use aphel_sdk::platform::application::shared::RecordRef;
use aphel_sdk::platform::infrastructure::records::mysql_record_access::MySqlRecordAccessChecker;
use aphel_sdk::platform::infrastructure::storage::mysql_files::MySqlFilePorts;

pub(crate) async fn load_record(
    ctx: &Ctx<'_>,
    record: &RecordRef,
) -> Result<Option<Value>, AppError> {
    let state = ctx.runtime();
    ctx.registry()
        .handler(record.record_type.as_str())?
        .load(
            &aphel_kernel::records::RecordCtx::from_runtime(ctx.scope, &state),
            record.record_id,
        )
        .await
}

pub(crate) async fn readable_fields(
    ctx: &Ctx<'_>,
    record_type: &str,
) -> Result<Vec<VariableField>, AppError> {
    let fields = ctx.field_set(ctx.scope, record_type).await?;
    Ok(fields
        .active()
        .map(|field| VariableField {
            key: field.key().to_owned(),
            label: field.definition.label.clone(),
        })
        .collect())
}

pub(crate) async fn can_read(ctx: &Ctx<'_>, record: &RecordRef) -> Result<bool, AppError> {
    record_access(ctx).can_read(ctx.scope, record).await
}

pub(crate) async fn require_readable(ctx: &Ctx<'_>, record: &RecordRef) -> Result<(), AppError> {
    if can_read(ctx, record).await? {
        return Ok(());
    }
    Err(AppError::forbidden_code(
        PERMISSION_DENIED,
        format!(
            "missing permission to access {} records",
            record.record_type
        ),
    ))
}

pub(crate) async fn describe_attachment(
    ctx: &Ctx<'_>,
    file_id: &str,
) -> Result<AttachmentFile, AppError> {
    let state = ctx.runtime();
    let files = FileUseCases::new(
        MySqlFilePorts::new(
            ctx.pool().clone(),
            state.storage().clone(),
            ctx.events().clone(),
            ctx.audit().clone(),
        )
        .with_records(record_access(ctx)),
        FileSettings {
            max_upload_bytes: state.max_upload_bytes(),
        },
    );
    let file = files.download(ctx.scope, file_id).await?.file;
    Ok(AttachmentFile {
        file_id: file.id,
        name: file.original_name,
        mime_type: file.mime_type,
        size: file.size,
    })
}

fn record_access(ctx: &Ctx<'_>) -> MySqlRecordAccessChecker {
    MySqlRecordAccessChecker::new(ctx.pool().clone(), ctx.registry().clone())
}
