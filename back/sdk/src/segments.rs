use crate::shared::RecordType;
use crate::{AppError, Ctx};
pub use aphel_contract_segments::*;
use uuid::Uuid;
#[async_trait::async_trait]
pub trait SegmentTargetReader: Send + Sync {
    async fn target(&self, ctx: &Ctx<'_>, id: Uuid) -> Result<Option<RecordType>, AppError>;
}
pub async fn target(ctx: &Ctx<'_>, id: Uuid) -> Result<Option<RecordType>, AppError> {
    let Some(reader) = ctx.services().get::<dyn SegmentTargetReader>() else {
        return Ok(None);
    };
    reader.target(ctx, id).await
}
