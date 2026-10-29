use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

use serde_json::Value;
use sqlx::{MySql, MySqlConnection, Transaction};
use uuid::Uuid;

pub type DatabaseTransaction<'a> = Transaction<'a, MySql>;

pub struct MySqlTx {
    inner: DatabaseTransaction<'static>,
    pub operation_id: Uuid,
    pub(crate) receipt_lease: Option<super::receipt::ReceiptLease>,
    pub record_states: HashMap<(String, Uuid), Value>,
}

impl From<DatabaseTransaction<'static>> for MySqlTx {
    fn from(inner: DatabaseTransaction<'static>) -> Self {
        Self {
            inner,
            receipt_lease: None,
            operation_id: Uuid::now_v7(),
            record_states: HashMap::new(),
        }
    }
}

impl Deref for MySqlTx {
    type Target = MySqlConnection;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for MySqlTx {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl MySqlTx {
    pub async fn commit(self) -> Result<(), sqlx::Error> {
        self.inner.commit().await?;
        if let Some(lease) = self.receipt_lease {
            lease.committed();
        }
        Ok(())
    }
    pub async fn rollback(self) -> Result<(), sqlx::Error> {
        self.inner.rollback().await
    }
}
