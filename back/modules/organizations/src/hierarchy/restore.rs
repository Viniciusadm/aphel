use aphel_contract_crm::CrmRecordTypes;
use std::sync::Arc;

use async_trait::async_trait;
use sqlx::MySqlPool;

use crate::domain::COMPANY_RESTORED;
use crate::hierarchy::{HierarchyStore, reattach_children};
use aphel_kernel::Ctx;
use aphel_kernel::application::custom_objects::RecordTypeCatalog;
use aphel_kernel::application::shared::{DomainEvent, EventConsumer, RecordType};
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::audit::MySqlAuditLog;
use aphel_kernel::infrastructure::events::MySqlEventPublisher;

pub(crate) const HIERARCHY_RESTORE_CONSUMER: &str = "organizations.hierarchy_restore";

pub(crate) struct HierarchyRestoreConsumer {
    pool: MySqlPool,
    catalog: Arc<RecordTypeCatalog>,
}

impl HierarchyRestoreConsumer {
    pub(crate) fn shared(
        pool: MySqlPool,
        catalog: Arc<RecordTypeCatalog>,
    ) -> Arc<dyn EventConsumer> {
        Arc::new(Self { pool, catalog })
    }
}

#[async_trait]
impl EventConsumer for HierarchyRestoreConsumer {
    fn name(&self) -> &str {
        HIERARCHY_RESTORE_CONSUMER
    }

    fn accepts(&self, event_type: &str) -> bool {
        event_type == COMPANY_RESTORED
    }

    async fn consume(&self, event: &DomainEvent) -> Result<(), AppError> {
        let Some(record) = event
            .record
            .as_ref()
            .filter(|record| record.record_type == RecordType::COMPANY)
        else {
            return Ok(());
        };
        let scope = AccessScope::system();
        let ctx = Ctx::from_parts(
            &scope,
            self.pool.clone(),
            self.catalog.registry().await?,
            Arc::new(MySqlEventPublisher::new()),
            Arc::new(MySqlAuditLog::new()),
        );
        let now = ctx.now();
        let mut tx = ctx.begin().await?;
        reattach_children(
            &HierarchyStore::of(&ctx),
            ctx.events(),
            ctx.audit(),
            &mut tx,
            &scope,
            record.record_id,
            now,
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }
}
