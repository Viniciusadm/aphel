use std::collections::{BTreeSet, HashMap};

use serde_json::Value;
use uuid::Uuid;

use crate::application::audit::{DiffOptions, diff_values};
use crate::application::fields::{CUSTOM_FIELDS_PREFIX, FieldDefinitionRepository, ResolvedField};
use crate::application::shared::{FieldChange, RecordType};
use crate::domain::access::AccessScope;
use crate::errors::{AppError, FieldViolation};

pub const FIELD_NOT_ACCESSIBLE: &str = "field_not_accessible";
pub const FIELD_READ_ONLY: &str = "field_read_only";
pub const FIELD_PERMISSION_LOCKED: &str = "field_permission_locked";

pub const NOT_ACCESSIBLE_MESSAGE: &str = "is not accessible";
pub const READ_ONLY_MESSAGE: &str = "is read-only";
pub const HIDDEN_VALUE: &str = "[hidden]";

const ALWAYS_LOCKED_KEYS: [&str; 3] = ["owner_id", "created_at", "updated_at"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub enum FieldAccess {
    Hidden,
    ReadOnly,
    #[default]
    Editable,
}

impl FieldAccess {
    pub const ALL: [Self; 3] = [Self::Hidden, Self::ReadOnly, Self::Editable];

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Hidden => "hidden",
            Self::ReadOnly => "read_only",
            Self::Editable => "editable",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|access| access.as_str() == value)
    }

    pub fn is_visible(&self) -> bool {
        *self != Self::Hidden
    }

    pub fn is_editable(&self) -> bool {
        *self == Self::Editable
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldAccessRule {
    pub field_id: Uuid,
    pub key: String,
    pub is_system: bool,
    pub access: FieldAccess,
}

pub fn is_locked_key(record_type: &RecordType, key: &str) -> bool {
    ALWAYS_LOCKED_KEYS.contains(&key)
        || crate::modules::app()
            .record_types()
            .get(record_type)
            .is_some_and(|definition| {
                definition
                    .capabilities
                    .locked_fields
                    .iter()
                    .any(|locked| locked == key)
            })
}

pub fn is_access_locked(record_type: &RecordType, field: &ResolvedField) -> bool {
    match &field.system {
        Some(system) => system.required || is_locked_key(record_type, field.key()),
        None => field.definition.is_required && field.definition.config.default_value.is_none(),
    }
}

pub fn is_unrestricted(scope: &AccessScope) -> bool {
    scope.permissions().grants_everything() || scope.role_key().is_admin()
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FieldAccessPolicy {
    by_id: HashMap<Uuid, FieldAccess>,
    by_key: HashMap<String, FieldAccess>,
    system_keys: BTreeSet<String>,
}

impl FieldAccessPolicy {
    pub fn unrestricted() -> Self {
        Self::default()
    }

    pub fn from_rules(record_type: &RecordType, rules: Vec<FieldAccessRule>) -> Self {
        let mut policy = Self::default();
        for rule in rules {
            if rule.access == FieldAccess::Editable || is_locked_key(record_type, &rule.key) {
                continue;
            }
            policy.by_id.insert(rule.field_id, rule.access);
            if rule.is_system {
                policy.system_keys.insert(rule.key.clone());
            }
            policy.by_key.insert(rule.key, rule.access);
        }
        policy
    }

    pub async fn load<D>(
        definitions: &D,
        scope: &AccessScope,
        record_type: &RecordType,
    ) -> Result<Self, AppError>
    where
        D: FieldDefinitionRepository + ?Sized,
    {
        if is_unrestricted(scope) {
            return Ok(Self::unrestricted());
        }
        let rules = definitions.access_rules(scope, record_type).await?;
        Ok(Self::from_rules(record_type, rules))
    }

    pub fn is_restricted(&self) -> bool {
        !self.by_id.is_empty()
    }

    pub fn access_of(&self, field_id: Uuid) -> FieldAccess {
        self.by_id.get(&field_id).copied().unwrap_or_default()
    }

    pub fn access_for(&self, record_type: &RecordType, field: &ResolvedField) -> FieldAccess {
        if field.definition.is_formula() {
            return match self.access_of(field.definition.id) {
                FieldAccess::Hidden => FieldAccess::Hidden,
                _ => FieldAccess::ReadOnly,
            };
        }
        if field
            .system
            .as_ref()
            .is_some_and(|system| system.is_computed())
        {
            return match self.access_of(field.definition.id) {
                FieldAccess::Hidden => FieldAccess::Hidden,
                _ => FieldAccess::ReadOnly,
            };
        }
        if is_access_locked(record_type, field) {
            return FieldAccess::Editable;
        }
        self.access_of(field.definition.id)
    }

    pub fn key_access(&self, key: &str) -> FieldAccess {
        self.by_key.get(key).copied().unwrap_or_default()
    }

    pub fn path_access(&self, path: &str) -> FieldAccess {
        let key = path
            .strip_prefix(CUSTOM_FIELDS_PREFIX)
            .and_then(|rest| rest.strip_prefix('.'))
            .unwrap_or(path);
        let key = key.split('.').next().unwrap_or(key);
        self.key_access(key)
    }

    pub fn hidden_keys(&self) -> BTreeSet<String> {
        self.by_key
            .iter()
            .filter(|(_, access)| **access == FieldAccess::Hidden)
            .map(|(key, _)| key.clone())
            .collect()
    }

    pub fn hidden_system_keys(&self) -> BTreeSet<String> {
        self.hidden_keys()
            .into_iter()
            .filter(|key| self.system_keys.contains(key))
            .collect()
    }

    pub fn hidden_custom_keys(&self) -> BTreeSet<String> {
        self.hidden_keys()
            .into_iter()
            .filter(|key| !self.system_keys.contains(key))
            .collect()
    }

    pub fn check_system_writes(
        &self,
        provided: &[&str],
        changes: &[FieldChange],
    ) -> Result<(), AppError> {
        let changed: BTreeSet<&str> = changes.iter().map(|change| change.field.as_str()).collect();
        check_access(provided.iter().filter_map(|key| {
            let access = self.key_access(key);
            let violates = match access {
                FieldAccess::Hidden => true,
                FieldAccess::ReadOnly => changed.contains(key),
                FieldAccess::Editable => false,
            };
            violates.then(|| (access, (*key).to_owned()))
        }))
    }

    pub fn mask_changes(&self, changes: &mut [FieldChange]) {
        for change in changes {
            if self.path_access(&change.field) == FieldAccess::Hidden {
                change.old = Value::String(HIDDEN_VALUE.to_owned());
                change.new = Value::String(HIDDEN_VALUE.to_owned());
            }
        }
    }

    pub fn mask_event_payload(&self, event_type: &str, payload: &mut Value) {
        let hidden = self.hidden_keys();
        if hidden.is_empty() {
            return;
        }
        let changed_field = event_type
            .split_once('.')
            .and_then(|(_, action)| action.strip_suffix("_changed"))
            .filter(|field| hidden.contains(*field));
        let Some(object) = payload.as_object_mut() else {
            return;
        };
        for (key, value) in object.iter_mut() {
            let masked = match key.as_str() {
                "changes" => {
                    self.mask_change_values(value);
                    false
                }
                "old" | "new" | "from" | "to" => changed_field.is_some() || hidden.contains(key),
                _ => hidden.contains(key),
            };
            if masked {
                *value = Value::String(HIDDEN_VALUE.to_owned());
            }
        }
    }

    pub fn mask_change_values(&self, changes: &mut Value) {
        let Some(items) = changes.as_array_mut() else {
            return;
        };
        for item in items {
            let hidden = item
                .get("field")
                .and_then(Value::as_str)
                .is_some_and(|field| self.path_access(field) == FieldAccess::Hidden);
            if let (true, Some(object)) = (hidden, item.as_object_mut()) {
                for key in ["old", "new"] {
                    if object.contains_key(key) {
                        object.insert(key.to_owned(), Value::String(HIDDEN_VALUE.to_owned()));
                    }
                }
            }
        }
    }
}

pub async fn check_record_writes<D>(
    definitions: &D,
    scope: &AccessScope,
    record_type: &RecordType,
    provided: &[&str],
    old: &Value,
    new: &Value,
) -> Result<(), AppError>
where
    D: FieldDefinitionRepository + ?Sized,
{
    let policy = FieldAccessPolicy::load(definitions, scope, record_type).await?;
    if !policy.is_restricted() {
        return Ok(());
    }
    let changes = diff_values(old, new, &DiffOptions::default())?;
    policy.check_system_writes(provided, &changes)
}

pub fn provided_key(keys: &mut Vec<&'static str>, key: &'static str, provided: bool) {
    if provided {
        keys.push(key);
    }
}

pub fn check_access(
    entries: impl IntoIterator<Item = (FieldAccess, String)>,
) -> Result<(), AppError> {
    let mut hidden = Vec::new();
    let mut read_only = Vec::new();
    for (access, path) in entries {
        match access {
            FieldAccess::Hidden => hidden.push(FieldViolation::new(path, NOT_ACCESSIBLE_MESSAGE)),
            FieldAccess::ReadOnly => read_only.push(FieldViolation::new(path, READ_ONLY_MESSAGE)),
            FieldAccess::Editable => {}
        }
    }
    if !hidden.is_empty() {
        hidden.sort_by(|left, right| left.field.cmp(&right.field));
        return Err(not_accessible(hidden));
    }
    if !read_only.is_empty() {
        read_only.sort_by(|left, right| left.field.cmp(&right.field));
        return Err(read_only_error(read_only));
    }
    Ok(())
}

pub fn read_only_error(fields: Vec<FieldViolation>) -> AppError {
    AppError::invalid_fields_code(
        FIELD_READ_ONLY,
        "the role of the user cannot change these fields",
        fields,
    )
}

pub fn not_accessible(fields: Vec<FieldViolation>) -> AppError {
    AppError::invalid_fields_code(
        FIELD_NOT_ACCESSIBLE,
        "the role of the user cannot see these fields",
        fields,
    )
}

pub fn has_inaccessible(violations: &[FieldViolation]) -> bool {
    violations
        .iter()
        .any(|violation| violation.message == NOT_ACCESSIBLE_MESSAGE)
}

pub fn query_error(message: &str, violations: Vec<FieldViolation>) -> AppError {
    if has_inaccessible(&violations) {
        return not_accessible(violations);
    }
    if violations
        .iter()
        .any(|violation| violation.message == READ_ONLY_MESSAGE)
    {
        return AppError::invalid_fields_code(FIELD_READ_ONLY, message.to_owned(), violations);
    }
    AppError::invalid_fields(message.to_owned(), violations)
}
