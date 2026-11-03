use crate::{AppError, Ctx, shared::RecordType};
use async_trait::async_trait;
use std::collections::HashMap;
use uuid::Uuid;

pub struct FavoriteViewTarget {
    pub record_type: RecordType,
    pub name: String,
}

#[async_trait]
pub trait FavoriteViewSource: Send + Sync {
    async fn visible(
        &self,
        ctx: &Ctx<'_>,
        ids: &[Uuid],
    ) -> Result<HashMap<Uuid, FavoriteViewTarget>, AppError>;
}
