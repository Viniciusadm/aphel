use crate::AppError;
use aphel_kernel::database::transaction::MySqlTx;
use async_trait::async_trait;
use uuid::Uuid;

#[async_trait]
pub trait ReferenceCleanup: Send + Sync {
    fn kinds(&self) -> &'static [&'static str];
    async fn removed(&self, tx: &mut MySqlTx, kind: &str, id: Uuid) -> Result<(), AppError>;
}

pub async fn removed(tx: &mut MySqlTx, kind: &str, id: Uuid) -> Result<(), AppError> {
    for cleanup in aphel_kernel::modules::app().contributions::<dyn ReferenceCleanup>() {
        if cleanup.kinds().contains(&kind) {
            cleanup.removed(tx, kind, id).await?;
        }
    }
    Ok(())
}
