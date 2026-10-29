use async_trait::async_trait;

use crate::service;
use aphel_kernel::Ctx;
use aphel_kernel::application::automations::TagOperations;
use aphel_kernel::application::shared::RecordRef;
use aphel_kernel::errors::AppError;

pub(crate) struct AutomationTags;

#[async_trait]
impl TagOperations for AutomationTags {
    async fn find_tag(&self, ctx: &Ctx<'_>, id: &str) -> Result<(), AppError> {
        service::get(ctx, id).await.map(|_| ())
    }

    async fn set_tag(
        &self,
        ctx: &Ctx<'_>,
        record: &RecordRef,
        tag_id: &str,
        present: bool,
    ) -> Result<(), AppError> {
        let record_type = record.record_type.as_str();
        let id = record.record_id.to_string();
        if present {
            service::tag_record(ctx, record_type, &id, tag_id, None).await?;
        } else {
            service::untag_record(ctx, record_type, &id, tag_id, None).await?;
        }
        Ok(())
    }
}
