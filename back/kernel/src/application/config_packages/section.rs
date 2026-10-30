use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::application::config_packages::ConfigPackage;
use crate::domain::access::AccessScope;
use crate::errors::AppError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageIssue {
    pub path: String,
    pub code: String,
    pub message: String,
}

impl PackageIssue {
    pub fn new(
        path: impl Into<String>,
        code: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            path: path.into(),
            code: code.into(),
            message: message.into(),
        }
    }

    pub fn under(mut self, prefix: &str) -> Self {
        self.path = if self.path.is_empty() {
            prefix.to_owned()
        } else {
            format!("{prefix}.{}", self.path)
        };
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanAction {
    Create,
    Update,
    SkipConflict,
    Unchanged,
}

impl PlanAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Create => "create",
            Self::Update => "update",
            Self::SkipConflict => "skip_conflict",
            Self::Unchanged => "unchanged",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlanChange {
    pub property: String,
    pub current: Value,
    pub proposed: Value,
}

impl PlanChange {
    pub fn new(property: impl Into<String>, current: Value, proposed: Value) -> Self {
        Self {
            property: property.into(),
            current,
            proposed,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanConflict {
    pub code: String,
    pub message: String,
    pub resolvable: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlanItem {
    pub id: String,
    pub section: String,
    pub kind: String,
    pub key: String,
    pub label: String,
    pub action: PlanAction,
    pub changes: Vec<PlanChange>,
    pub conflict: Option<PlanConflict>,
}

impl PlanItem {
    pub fn new(
        section: &str,
        kind: &str,
        key: impl Into<String>,
        label: impl Into<String>,
    ) -> Self {
        let key = key.into();
        Self {
            id: format!("{section}:{kind}:{key}"),
            section: section.to_owned(),
            kind: kind.to_owned(),
            key,
            label: label.into(),
            action: PlanAction::Create,
            changes: Vec::new(),
            conflict: None,
        }
    }

    pub fn created(mut self, changes: Vec<PlanChange>) -> Self {
        self.action = PlanAction::Create;
        self.changes = changes;
        self
    }

    pub fn compared(mut self, changes: Vec<PlanChange>) -> Self {
        self.action = if changes.is_empty() {
            PlanAction::Unchanged
        } else {
            PlanAction::Update
        };
        self.changes = changes;
        self
    }

    pub fn conflicting(
        mut self,
        changes: Vec<PlanChange>,
        code: &str,
        message: impl Into<String>,
        resolvable: bool,
    ) -> Self {
        self.action = PlanAction::SkipConflict;
        self.changes = changes;
        self.conflict = Some(PlanConflict {
            code: code.to_owned(),
            message: message.into(),
            resolvable,
        });
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictDecision {
    Apply,
    Skip,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Decisions(HashMap<String, ConflictDecision>);

impl Decisions {
    pub fn new(decisions: HashMap<String, ConflictDecision>) -> Self {
        Self(decisions)
    }

    pub fn get(&self, item_id: &str) -> Option<ConflictDecision> {
        self.0.get(item_id).copied()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &ConflictDecision)> {
        self.0.iter()
    }

    pub fn executes(&self, item: &PlanItem) -> bool {
        match item.action {
            PlanAction::Create | PlanAction::Update => true,
            PlanAction::Unchanged => false,
            PlanAction::SkipConflict => {
                item.conflict
                    .as_ref()
                    .is_some_and(|conflict| conflict.resolvable)
                    && self.get(&item.id) == Some(ConflictDecision::Apply)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ApplyContext {
    pub at: DateTime<Utc>,
    resolved: BTreeMap<(String, String), Uuid>,
}

impl ApplyContext {
    pub fn new(at: DateTime<Utc>) -> Self {
        Self {
            at,
            ..Self::default()
        }
    }

    pub fn resolve(&mut self, item_type: &str, key: &str, id: Uuid) {
        self.resolved
            .insert((item_type.to_owned(), key.to_owned()), id);
    }

    pub fn resolved(&self, item_type: &str, key: &str) -> Option<Uuid> {
        self.resolved
            .get(&(item_type.to_owned(), key.to_owned()))
            .copied()
    }
}

#[derive(Debug, Clone, Default)]
pub struct UnavailableFields {
    fields: BTreeSet<(String, String)>,
}

impl UnavailableFields {
    pub fn insert(&mut self, record_type: &str, key: &str) {
        self.fields.insert((record_type.to_owned(), key.to_owned()));
    }

    pub fn contains(&self, record_type: &str, key: &str) -> bool {
        self.fields
            .contains(&(record_type.to_owned(), key.to_owned()))
    }

    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    pub fn referenced_in(&self, record_type: &str, value: &Value) -> Option<String> {
        match value {
            Value::Object(map) => map.iter().find_map(|(name, item)| match item {
                Value::String(key) if name == "field" && self.contains(record_type, key) => {
                    Some(key.clone())
                }
                _ => self.referenced_in(record_type, item),
            }),
            Value::Array(items) => items
                .iter()
                .find_map(|item| self.referenced_in(record_type, item)),
            _ => None,
        }
    }
}

pub fn field_reference(record_type: &str, key: &str) -> String {
    format!("{record_type}.{key}")
}

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

pub fn parse_section<T: for<'de> Deserialize<'de>>(content: &Value) -> Result<T, PackageIssue> {
    serde_json::from_value(content.clone())
        .map_err(|error| PackageIssue::new("", "invalid_section", error.to_string()))
}
