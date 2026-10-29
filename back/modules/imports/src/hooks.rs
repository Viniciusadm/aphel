use std::future::Future;

use uuid::Uuid;

use crate::RowAction;
use crate::store::ImportStore;
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::HookCtx;
use aphel_kernel::records::ChangedRecord;

tokio::task_local! {
    static WRITING_ROW: WritingRow;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WritingRow {
    pub(crate) import_id: Uuid,
    pub(crate) row_index: u32,
    pub(crate) record_type: String,
    pub(crate) action: RowAction,
}

pub(crate) async fn writing_row<F: Future>(row: WritingRow, future: F) -> F::Output {
    WRITING_ROW.scope(row, future).await
}

pub(crate) async fn mark_written_row(
    ctx: &mut HookCtx<'_>,
    record: &ChangedRecord,
) -> Result<(), AppError> {
    let Ok(row) = WRITING_ROW.try_with(Clone::clone) else {
        return Ok(());
    };
    if record.record_type.as_str() != row.record_type {
        return Ok(());
    }
    ImportStore::mark_row_written(
        ctx.tx(),
        row.import_id,
        row.row_index,
        row.action,
        record.record.record_id,
    )
    .await
}
