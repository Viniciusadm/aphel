use std::collections::BTreeMap;
use std::sync::Arc;

use crate::application::shared::JobHandler;
use crate::errors::AppError;

#[derive(Clone, Default)]
pub struct JobRegistry {
    handlers: BTreeMap<String, Arc<dyn JobHandler>>,
}

impl JobRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, handler: Arc<dyn JobHandler>) -> &mut Self {
        self.handlers.insert(handler.kind().to_owned(), handler);
        self
    }

    pub fn with(mut self, handler: Arc<dyn JobHandler>) -> Self {
        self.register(handler);
        self
    }

    pub fn merge(mut self, other: &JobRegistry) -> Self {
        for handler in other.handlers.values() {
            self.register(handler.clone());
        }
        self
    }

    pub fn get(&self, kind: &str) -> Option<Arc<dyn JobHandler>> {
        self.handlers.get(kind).cloned()
    }

    pub fn resolve(&self, kind: &str) -> Result<Arc<dyn JobHandler>, AppError> {
        self.get(kind).ok_or_else(|| {
            AppError::Internal(format!("no job handler is registered for kind {kind}"))
        })
    }

    pub fn contains(&self, kind: &str) -> bool {
        self.handlers.contains_key(kind)
    }

    pub fn kinds(&self) -> impl Iterator<Item = &str> {
        self.handlers.keys().map(String::as_str)
    }

    pub fn len(&self) -> usize {
        self.handlers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.handlers.is_empty()
    }
}
