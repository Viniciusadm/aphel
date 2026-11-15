use std::collections::HashMap;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::application::fields::permissions::{FieldAccess, FieldAccessPolicy};
use crate::application::fields::{EntityDefinition, FieldDefinitionRepository, SystemField};
use crate::domain::access::AccessScope;
use crate::domain::fields::FieldDefinition;
use crate::errors::AppError;

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedField {
    pub labels: super::LocalizedLabels,
    pub definition: FieldDefinition,
    pub system: Option<SystemField>,
    pub updated_at: Option<DateTime<Utc>>,
    pub access: FieldAccess,
}

impl ResolvedField {
    pub fn is_hidden(&self) -> bool {
        self.access == FieldAccess::Hidden
    }

    pub fn is_editable(&self) -> bool {
        self.access.is_editable()
    }

    pub fn key(&self) -> &str {
        self.definition.key.as_str()
    }

    pub fn is_system(&self) -> bool {
        self.system.is_some()
    }

    pub fn filterable(&self) -> bool {
        self.system.as_ref().is_none_or(|system| system.filterable)
    }

    pub fn sortable(&self) -> bool {
        match &self.system {
            Some(system) => system.sortable,
            None => self.definition.field_type.is_sortable(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldSet {
    pub record_type: EntityDefinition,
    fields: Vec<ResolvedField>,
}

impl FieldSet {
    pub fn new(record_type: &EntityDefinition, mut definitions: Vec<FieldDefinition>) -> Self {
        definitions.sort_by_key(|definition| definition.position);
        let fields = definitions
            .into_iter()
            .filter_map(|mut definition| {
                if !definition.is_system {
                    let access = if definition.is_formula() {
                        FieldAccess::ReadOnly
                    } else {
                        FieldAccess::Editable
                    };
                    let labels = record_type
                        .managed_field_by_key(definition.key.as_str())
                        .filter(|field| field.spec.label == definition.label)
                        .map(|field| field.spec.labels.clone())
                        .unwrap_or_else(|| definition.label.clone().into());
                    return Some(ResolvedField {
                        labels,
                        definition,
                        system: None,
                        updated_at: None,
                        access,
                    });
                }
                let system = record_type
                    .system_field_by_key(definition.key.as_str())?
                    .clone();
                definition.field_type = system.field_type;
                definition.column_name = Some(system.column.clone());
                definition.is_required = system.required;
                definition.is_active = system.is_available();
                let access = if system.is_computed() || !system.writable {
                    FieldAccess::ReadOnly
                } else {
                    FieldAccess::Editable
                };
                let untouched = system.label == definition.label
                    || system.labels.0.values().any(|label| *label == definition.label);
                let labels = if untouched {
                    definition.label = system.label.clone();
                    system.labels.clone()
                } else {
                    definition.label.clone().into()
                };
                Some(ResolvedField {
                    labels,
                    definition,
                    system: Some(system),
                    updated_at: None,
                    access,
                })
            })
            .collect();
        Self {
            record_type: record_type.clone(),
            fields,
        }
    }

    pub fn with_versions(mut self, versions: &HashMap<Uuid, DateTime<Utc>>) -> Self {
        for field in &mut self.fields {
            field.updated_at = versions.get(&field.definition.id).copied();
        }
        self
    }

    pub async fn for_scope<D>(
        definitions: &D,
        record_type: &EntityDefinition,
        scope: &AccessScope,
    ) -> Result<Self, AppError>
    where
        D: FieldDefinitionRepository + ?Sized,
    {
        let policy = FieldAccessPolicy::load(definitions, scope, &record_type.record_type).await?;
        Ok(Self::new(
            record_type,
            definitions.list(&record_type.record_type).await?,
        )
        .with_access(&policy))
    }

    pub fn with_access(mut self, policy: &FieldAccessPolicy) -> Self {
        let record_type = self.record_type.record_type.clone();
        for field in &mut self.fields {
            field.access = policy.access_for(&record_type, field);
        }
        self
    }

    pub fn all(&self) -> &[ResolvedField] {
        &self.fields
    }

    pub fn active(&self) -> impl Iterator<Item = &ResolvedField> {
        self.fields
            .iter()
            .filter(|field| field.definition.is_active && !field.is_hidden())
    }

    pub fn is_hidden(&self, key: &str) -> bool {
        self.find(key)
            .is_some_and(|field| field.definition.is_active && field.is_hidden())
    }

    pub fn hidden_keys(&self) -> Vec<String> {
        self.fields
            .iter()
            .filter(|field| field.is_hidden())
            .map(|field| field.key().to_owned())
            .collect()
    }

    pub fn active_custom(&self) -> impl Iterator<Item = &ResolvedField> {
        self.active().filter(|field| !field.is_system())
    }

    pub fn find(&self, key: &str) -> Option<&ResolvedField> {
        self.fields.iter().find(|field| field.key() == key)
    }

    pub fn find_active(&self, key: &str) -> Option<&ResolvedField> {
        self.active().find(|field| field.key() == key)
    }

    pub fn find_by_id(&self, id: uuid::Uuid) -> Option<&ResolvedField> {
        self.fields.iter().find(|field| field.definition.id == id)
    }
}
