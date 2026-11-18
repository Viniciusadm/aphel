use std::future::Future;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use sqlx::MySqlPool;
use uuid::Uuid;

use crate::database::transaction::Tx;
use crate::errors::AppError;

pub struct MutationReceipt {
    id: Uuid,
    state: Arc<ReceiptState>,
}

pub(crate) struct ReceiptState {
    active: AtomicBool,
    committed: AtomicBool,
}

pub(crate) struct ReceiptLease(Arc<ReceiptState>);

impl ReceiptLease {
    pub fn committed(&self) {
        self.0.committed.store(true, Ordering::SeqCst);
    }
}

impl Drop for ReceiptLease {
    fn drop(&mut self) {
        self.0.active.store(false, Ordering::SeqCst);
    }
}

tokio::task_local! {
    static RECEIPT: MutationReceipt;
}

impl MutationReceipt {
    pub async fn scope<F: Future>(id: Uuid, future: F) -> F::Output {
        RECEIPT
            .scope(
                Self {
                    id,
                    state: Arc::new(ReceiptState {
                        active: AtomicBool::new(false),
                        committed: AtomicBool::new(false),
                    }),
                },
                future,
            )
            .await
    }
}

pub async fn finish_noop(pool: &MySqlPool) -> Result<(), AppError> {
    if RECEIPT
        .try_with(|receipt| {
            !receipt.state.active.load(Ordering::SeqCst)
                && !receipt.state.committed.load(Ordering::SeqCst)
        })
        .unwrap_or(false)
    {
        begin(pool).await?.commit().await?;
    }
    Ok(())
}

pub async fn applied(pool: &MySqlPool, id: Uuid) -> Result<bool, AppError> {
    Ok(
        sqlx::query_scalar::<_, i32>("SELECT 1 FROM mutation_receipts WHERE id = ?")
            .bind(id.to_string())
            .fetch_optional(pool)
            .await?
            .is_some(),
    )
}

pub async fn begin(pool: &MySqlPool) -> Result<Tx, AppError> {
    let receipt = RECEIPT.try_with(|receipt| {
        if receipt.state.committed.load(Ordering::SeqCst) || receipt.state.active.swap(true, Ordering::SeqCst) {
            return Err(AppError::conflict_code("mutation_multiple_transactions", "durable mutations must use one committing transaction without nested transactions"));
        }
        Ok((receipt.id, ReceiptLease(receipt.state.clone())))
    }).ok().transpose()?;
    let mut tx = Tx::from(pool.begin().await?);
    if let Some((id, lease)) = receipt {
        tx.receipt_lease = Some(lease);
        let inserted = sqlx::query("INSERT IGNORE INTO mutation_receipts (id) VALUES (?)")
            .bind(id.to_string())
            .execute(&mut *tx)
            .await?
            .rows_affected();
        if inserted == 0 {
            return Err(AppError::conflict_code(
                "mutation_already_applied",
                "the durable mutation has already been committed",
            ));
        }
    }
    Ok(tx)
}
