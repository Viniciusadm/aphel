use std::sync::Arc;

use crate::application::shared::EventConsumer;

#[derive(Clone, Default)]
pub struct EventConsumerRegistry {
    consumers: Vec<Arc<dyn EventConsumer>>,
}

impl EventConsumerRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, consumer: Arc<dyn EventConsumer>) -> &mut Self {
        self.consumers
            .retain(|existing| existing.name() != consumer.name());
        self.consumers.push(consumer);
        self
    }

    pub fn with(mut self, consumer: Arc<dyn EventConsumer>) -> Self {
        self.register(consumer);
        self
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn EventConsumer>> {
        self.consumers
            .iter()
            .find(|consumer| consumer.name() == name)
            .cloned()
    }

    pub fn accepting(&self, event_type: &str) -> Vec<Arc<dyn EventConsumer>> {
        self.consumers
            .iter()
            .filter(|consumer| consumer.accepts(event_type))
            .cloned()
            .collect()
    }

    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.consumers.iter().map(|consumer| consumer.name())
    }

    pub fn len(&self) -> usize {
        self.consumers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.consumers.is_empty()
    }
}
