use aphel_kernel::errors::AppError;
use aphel_kernel::modules::HookCtx;
use aphel_kernel::records::RemovedRecord;

pub(crate) async fn purge_record_notes(
    ctx: &mut HookCtx<'_>,
    removed: &RemovedRecord,
) -> Result<(), AppError> {
    crate::store::delete_for_record(
        ctx.tx,
        removed.record_type.as_str(),
        &removed.record.record_id.to_string(),
    )
    .await?;
    Ok(())
}
