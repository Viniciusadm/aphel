use async_trait::async_trait;
use sqlx::MySqlPool;

use crate::application::shared::TransactionManager;
use crate::database::transaction::MySqlTx;
use crate::errors::AppError;

#[derive(Clone)]
pub struct MySqlTransactionManager {
    pool: MySqlPool,
}

impl MySqlTransactionManager {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl TransactionManager for MySqlTransactionManager {
    type Tx = MySqlTx;

    async fn begin(&self) -> Result<Self::Tx, AppError> {
        crate::database::receipt::begin(&self.pool).await
    }

    async fn commit(&self, tx: Self::Tx) -> Result<(), AppError> {
        Ok(tx.commit().await?)
    }
}
