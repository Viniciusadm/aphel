use super::{ApplyContext, ConfigPackage, Decisions, PackageIssue, PlanItem, UnavailableFields};
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::AppError;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::BTreeSet;
use std::sync::Arc;

#[async_trait]
pub trait ConfigSection<Tx: Send>: Send + Sync {
    fn key(&self) -> &'static str;
    fn prune(&self, _content: &mut Value, _unavailable: &UnavailableFields) -> Vec<String> {
        Vec::new()
    }
    async fn export(&self, scope: &AccessScope) -> Result<Option<Value>, AppError>;
    async fn validate(
        &self,
        scope: &AccessScope,
        package: &ConfigPackage,
        content: &Value,
    ) -> Result<Vec<PackageIssue>, AppError>;
    async fn plan(
        &self,
        scope: &AccessScope,
        package: &ConfigPackage,
        content: &Value,
    ) -> Result<Vec<PlanItem>, AppError>;
    async fn apply(
        &self,
        tx: &mut Tx,
        scope: &AccessScope,
        package: &ConfigPackage,
        content: &Value,
        decisions: &Decisions,
        context: &mut ApplyContext,
    ) -> Result<(), AppError>;
}

pub struct ConfigSectionRegistry<Tx> {
    sections: Vec<Arc<dyn ConfigSection<Tx>>>,
    unavailable: BTreeSet<&'static str>,
    unavailable_fields: UnavailableFields,
}

impl<Tx: Send> Default for ConfigSectionRegistry<Tx> {
    fn default() -> Self {
        Self {
            sections: Vec::new(),
            unavailable: BTreeSet::new(),
            unavailable_fields: UnavailableFields::default(),
        }
    }
}

impl<Tx: Send> Clone for ConfigSectionRegistry<Tx> {
    fn clone(&self) -> Self {
        Self {
            sections: self.sections.clone(),
            unavailable: self.unavailable.clone(),
            unavailable_fields: self.unavailable_fields.clone(),
        }
    }
}

impl<Tx: Send> ConfigSectionRegistry<Tx> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with(mut self, section: Arc<dyn ConfigSection<Tx>>) -> Self {
        self.register(section);
        self
    }

    pub fn with_all(self, sections: Vec<Arc<dyn ConfigSection<Tx>>>) -> Self {
        sections
            .into_iter()
            .fold(self, |registry, section| registry.with(section))
    }

    pub fn register(&mut self, section: Arc<dyn ConfigSection<Tx>>) {
        self.sections
            .retain(|existing| existing.key() != section.key());
        self.sections.push(section);
    }

    pub fn mark_unavailable(&mut self, key: &'static str) {
        self.unavailable.insert(key);
    }

    pub fn with_unavailable_fields(mut self, fields: UnavailableFields) -> Self {
        self.unavailable_fields = fields;
        self
    }

    pub fn unavailable_fields(&self) -> &UnavailableFields {
        &self.unavailable_fields
    }

    pub fn is_available(&self, key: &str) -> bool {
        !self.unavailable.contains(key)
    }

    pub fn sections(&self) -> &[Arc<dyn ConfigSection<Tx>>] {
        &self.sections
    }

    pub fn keys(&self) -> Vec<&'static str> {
        self.sections.iter().map(|section| section.key()).collect()
    }

    pub fn get(&self, key: &str) -> Option<&Arc<dyn ConfigSection<Tx>>> {
        self.sections.iter().find(|section| section.key() == key)
    }
}
