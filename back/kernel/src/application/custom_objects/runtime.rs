use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use crate::application::custom_objects::RecordTypeCatalog;
use crate::application::fields::RecordTypeRegistry;
use crate::application::shared::{DomainEvent, EventConsumer, JobContext, JobHandler};
use crate::errors::AppError;

pub type JobHandlerFactory =
    Arc<dyn Fn(Arc<RecordTypeRegistry>) -> Arc<dyn JobHandler> + Send + Sync>;

pub type EventConsumerFactory =
    Arc<dyn Fn(Arc<RecordTypeRegistry>) -> Arc<dyn EventConsumer> + Send + Sync>;

pub struct CatalogJobHandler {
    kind: String,
    catalog: Arc<RecordTypeCatalog>,
    factory: JobHandlerFactory,
}

impl CatalogJobHandler {
    pub fn new(catalog: Arc<RecordTypeCatalog>, factory: JobHandlerFactory) -> Self {
        let kind = factory(catalog.builtin()).kind().to_owned();
        Self {
            kind,
            catalog,
            factory,
        }
    }

    pub fn shared(
        catalog: Arc<RecordTypeCatalog>,
        factory: JobHandlerFactory,
    ) -> Arc<dyn JobHandler> {
        Arc::new(Self::new(catalog, factory))
    }
}

#[async_trait]
impl JobHandler for CatalogJobHandler {
    fn kind(&self) -> &str {
        &self.kind
    }

    async fn handle(&self, context: &JobContext, payload: Value) -> Result<(), AppError> {
        let registry = self.catalog.registry().await?;
        (self.factory)(registry).handle(context, payload).await
    }
}

pub struct CatalogEventConsumer {
    name: String,
    probe: Arc<dyn EventConsumer>,
    catalog: Arc<RecordTypeCatalog>,
    factory: EventConsumerFactory,
}

impl CatalogEventConsumer {
    pub fn shared(
        catalog: Arc<RecordTypeCatalog>,
        factory: EventConsumerFactory,
    ) -> Arc<dyn EventConsumer> {
        let probe = factory(catalog.builtin());
        Arc::new(Self {
            name: probe.name().to_owned(),
            probe,
            catalog,
            factory,
        })
    }
}

#[async_trait]
impl EventConsumer for CatalogEventConsumer {
    fn name(&self) -> &str {
        &self.name
    }

    fn accepts(&self, event_type: &str) -> bool {
        self.probe.accepts(event_type)
    }

    async fn consume(&self, event: &DomainEvent) -> Result<(), AppError> {
        let registry = self.catalog.registry().await?;
        (self.factory)(registry).consume(event).await
    }
}
