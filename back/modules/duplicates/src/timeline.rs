use aphel_kernel::{Ctx, application::shared::RecordRef, errors::AppError};
use aphel_sdk::timeline::TimelineEventScope;
use sqlx::{MySql, QueryBuilder};
pub(crate) struct MergedEvents;
#[async_trait::async_trait]
impl TimelineEventScope for MergedEvents {
    async fn extend(
        &self,
        _ctx: &Ctx<'_>,
        builder: &mut QueryBuilder<MySql>,
        subject: &RecordRef,
    ) -> Result<(), AppError> {
        builder.push(" OR (e.record_type = ").push_bind(subject.record_type.as_str().to_owned())
            .push(" AND e.record_id IN (SELECT m.merged_id FROM record_merges m WHERE m.record_type = ").push_bind(subject.record_type.as_str().to_owned())
            .push(" AND m.survivor_id = ").push_bind(subject.record_id.to_string()).push("))");
        Ok(())
    }
}
