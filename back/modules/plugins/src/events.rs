use async_trait::async_trait;
use serde_json::json;
use sqlx::MySqlPool;

use crate::registry;
use crate::runtime::{EVENT_TIMEOUT, invoke};
use aphel_kernel::application::shared::{DomainEvent, EventConsumer};
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::AppError;

pub(crate) const EVENTS_CONSUMER: &str = "plugins.events";

pub(crate) struct PluginEventsConsumer {
    pool: MySqlPool,
}

impl PluginEventsConsumer {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        registry::spawn_refresh(pool.clone());
        Self { pool }
    }
}

#[async_trait]
impl EventConsumer for PluginEventsConsumer {
    fn name(&self) -> &str {
        EVENTS_CONSUMER
    }

    fn accepts(&self, event_type: &str) -> bool {
        !registry::cached().event_consumers(event_type).is_empty()
    }

    async fn consume(&self, event: &DomainEvent) -> Result<(), AppError> {
        let snapshot = registry::current(&self.pool).await?;
        let consumers = snapshot.event_consumers(&event.event_type);
        if consumers.is_empty() {
            return Ok(());
        }
        let input = json!({
            "kind": "event",
            "event": {
                "id": event.id.to_string(),
                "type": event.event_type,
                "record_type": event.record.as_ref().map(|record| record.record_type.as_str().to_owned()),
                "record_id": event.record.as_ref().map(|record| record.record_id.to_string()),
                "payload": event.payload,
                "occurred_at": event.occurred_at.to_rfc3339(),
            },
        });
        let scope = AccessScope::system();
        let mut failed = None;
        for plugin in consumers {
            if let Err(error) = invoke(&plugin, &scope, &input, EVENT_TIMEOUT).await {
                tracing::warn!(plugin = %plugin.key, event = %event.event_type, error = %error, "plugin event handler failed");
                failed = Some(error);
            }
        }
        match failed {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
}
