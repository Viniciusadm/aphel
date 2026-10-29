use async_trait::async_trait;

use crate::errors::AppError;

#[async_trait]
pub trait TransactionManager: Send + Sync {
    type Tx: Send;

    async fn begin(&self) -> Result<Self::Tx, AppError>;
    async fn commit(&self, tx: Self::Tx) -> Result<(), AppError>;
}
