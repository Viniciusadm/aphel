use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

use serde_json::Value;
use sqlx::{MySql, MySqlConnection, Transaction};
use uuid::Uuid;

pub type DatabaseTransaction<'a> = Transaction<'a, MySql>;

pub struct MySqlTx {
    inner: DatabaseTransaction<'static>,
    pub operation_id: Uuid,
    pub(crate) reference_deletions: std::collections::HashSet<(String, Uuid)>,
    pub(crate) after_commit: Vec<crate::modules::BoxFuture<'static, ()>>,
    pub(crate) receipt_lease: Option<super::receipt::ReceiptLease>,
    pub record_states: HashMap<(String, Uuid), Value>,
    pub(crate) fresh_snapshot: Option<((String, Uuid), Option<Value>)>,
    pub(crate) visible_record: Option<(String, Uuid)>,
}

impl From<DatabaseTransaction<'static>> for MySqlTx {
    fn from(inner: DatabaseTransaction<'static>) -> Self {
        Self {
            inner,
            receipt_lease: None,
            operation_id: Uuid::now_v7(),
            reference_deletions: Default::default(),
            after_commit: Vec::new(),
            record_states: HashMap::new(),
            fresh_snapshot: None,
            visible_record: None,
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
    pub async fn commit(mut self) -> Result<(), sqlx::Error> {
        let after_commit = std::mem::take(&mut self.after_commit);
        self.inner.commit().await?;
        if let Some(lease) = self.receipt_lease {
            lease.committed();
        }
        for effect in after_commit {
            effect.await;
        }
        Ok(())
    }
    pub async fn rollback(self) -> Result<(), sqlx::Error> {
        self.inner.rollback().await
    }
}
