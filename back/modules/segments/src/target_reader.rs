use aphel_sdk::records::optional_record;
use aphel_sdk::segments::SegmentTargetReader;
use aphel_sdk::shared::RecordType;
use aphel_sdk::{AppError, Ctx};
use uuid::Uuid;
pub(crate) struct TargetReader;
#[async_trait::async_trait]
impl SegmentTargetReader for TargetReader {
    async fn target(&self, ctx: &Ctx<'_>, id: Uuid) -> Result<Option<RecordType>, AppError> {
        let Some(segment) = optional_record(crate::service::visible(ctx, &id.to_string()).await)?
        else {
            return Ok(None);
        };
        if !aphel_sdk::records::enabled(ctx, &segment.record_type) {
            return Ok(None);
        }
        Ok(Some(segment.record_type))
    }
}
