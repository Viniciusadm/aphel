use async_trait::async_trait;

use crate::domain::UserId;
use crate::domain::access::AccessGrantView;
use crate::errors::AppError;

#[async_trait]
pub trait AccessRepository: Send + Sync {
    async fn load_access(&self, user_id: &UserId) -> Result<Option<AccessGrantView>, AppError>;
}
