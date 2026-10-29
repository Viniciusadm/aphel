use async_trait::async_trait;

use crate::application::shared::RecordRef;
use crate::domain::access::{AccessScope, Permission};
use crate::errors::AppError;

pub const UNSUPPORTED_RECORD_TYPE: &str = "unsupported_record_type";

#[async_trait]
pub trait RecordAccessChecker: Send + Sync {
    async fn can_read(&self, scope: &AccessScope, record: &RecordRef) -> Result<bool, AppError>;
    async fn can_update(&self, scope: &AccessScope, record: &RecordRef) -> Result<bool, AppError>;
    async fn can_with(
        &self,
        _scope: &AccessScope,
        _record: &RecordRef,
        _permission: &Permission,
    ) -> Result<bool, AppError> {
        Ok(false)
    }
}
