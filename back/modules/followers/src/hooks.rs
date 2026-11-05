use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use sqlx::MySqlPool;

use crate::domain::FollowOrigin;
use crate::store::FollowerStore;
use aphel_sdk::platform::application::custom_objects::{CatalogEventConsumer, RecordTypeCatalog};

use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::fields::RecordTypeRegistry;
use aphel_sdk::platform::application::record_sharing::RecordDirectory;
use aphel_sdk::platform::application::shared::{DomainEvent, EventConsumer};
use aphel_sdk::platform::database::receipt;
use aphel_sdk::platform::domain::access::{AccessScope, RecordScopeFilter};
use aphel_sdk::platform::domain::custom_objects::is_custom_record_type;
use aphel_sdk::platform::infrastructure::record_sharing::MySqlRecordDirectory;

pub(crate) const FOLLOWERS_OWNERS_CONSUMER: &str = "followers.owners";

const OWNER_ACTIONS: [&str; 3] = ["created", "updated", "owner_changed"];

pub(crate) struct OwnerFollowConsumer {
    pool: MySqlPool,
    registry: Arc<RecordTypeRegistry>,
    followers: FollowerStore,
    records: MySqlRecordDirectory,
}

impl OwnerFollowConsumer {
    pub(crate) fn new(pool: MySqlPool, registry: Arc<RecordTypeRegistry>) -> Self {
        Self {
            registry,
            followers: FollowerStore::new(pool.clone()),
            records: MySqlRecordDirectory::new(pool.clone()),
            pool,
        }
    }

    pub(crate) fn shared(
        pool: MySqlPool,
        catalog: Arc<RecordTypeCatalog>,
    ) -> Arc<dyn EventConsumer> {
        CatalogEventConsumer::shared(
            catalog,
            Arc::new(move |registry| {
                Arc::new(Self::new(pool.clone(), registry)) as Arc<dyn EventConsumer>
            }),
        )
    }

    fn known(&self, record_type: &str) -> bool {
        is_custom_record_type(record_type)
            || self
                .registry
                .definitions()
                .any(|definition| definition.event_prefix == record_type)
    }
}

fn changes_field(payload: &Value, field: &str) -> bool {
    payload
        .get("changes")
        .and_then(Value::as_array)
        .is_some_and(|changes| {
            changes
                .iter()
                .any(|change| change.get("field").and_then(Value::as_str) == Some(field))
        })
}

#[async_trait]
impl EventConsumer for OwnerFollowConsumer {
    fn name(&self) -> &str {
        FOLLOWERS_OWNERS_CONSUMER
    }

    fn accepts(&self, event_type: &str) -> bool {
        event_type
            .rsplit_once('.')
            .is_some_and(|(prefix, action)| OWNER_ACTIONS.contains(&action) && self.known(prefix))
    }

    async fn consume(&self, event: &DomainEvent) -> Result<(), AppError> {
        let Some(record) = &event.record else {
            return Ok(());
        };
        let Some((prefix, action)) = event.event_type.rsplit_once('.') else {
            return Ok(());
        };
        let Some(definition) = self.registry.get(&record.record_type) else {
            return Ok(());
        };
        if definition.event_prefix != prefix {
            return Ok(());
        }
        let Some(owner_column) = definition.owner_column.as_deref() else {
            return Ok(());
        };
        if action == "updated" && !changes_field(&event.payload, owner_column) {
            return Ok(());
        }
        let scope = AccessScope::system();
        let Some(header) = self
            .records
            .header(
                &scope,
                definition,
                &RecordScopeFilter::Organization,
                record.record_id,
            )
            .await?
        else {
            return Ok(());
        };
        let Some(owner_id) = header.owner_id else {
            return Ok(());
        };
        let mut tx = receipt::begin(&self.pool).await?;
        self.followers
            .follow(
                &mut tx,
                record,
                &owner_id,
                FollowOrigin::Owner,
                event.occurred_at,
            )
            .await?;
        tx.commit().await?;
        Ok(())
    }
}
