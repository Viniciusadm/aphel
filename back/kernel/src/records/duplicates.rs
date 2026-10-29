use async_trait::async_trait;

use crate::Ctx;
use crate::application::shared::RecordType;
use crate::errors::AppError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PossibleDuplicate {
    pub record_id: String,
    pub title: String,
    pub rules: Vec<String>,
}

#[async_trait]
pub trait DuplicateDetector: Send + Sync {
    async fn detect(
        &self,
        ctx: &Ctx<'_>,
        record_type: &RecordType,
        record_id: &str,
    ) -> Vec<PossibleDuplicate>;

    async fn merged_into(
        &self,
        ctx: &Ctx<'_>,
        record_type: &RecordType,
        record_id: &str,
    ) -> Result<Option<String>, AppError>;
}

pub async fn possible_duplicates(
    ctx: &Ctx<'_>,
    record_type: &RecordType,
    record_id: &str,
) -> Vec<PossibleDuplicate> {
    match ctx.services().get::<dyn DuplicateDetector>() {
        Some(detector) => detector.detect(ctx, record_type, record_id).await,
        None => Vec::new(),
    }
}

pub async fn merge_survivor(
    ctx: &Ctx<'_>,
    record_type: &RecordType,
    record_id: &str,
) -> Result<Option<String>, AppError> {
    match ctx.services().get::<dyn DuplicateDetector>() {
        Some(detector) => detector.merged_into(ctx, record_type, record_id).await,
        None => Ok(None),
    }
}
