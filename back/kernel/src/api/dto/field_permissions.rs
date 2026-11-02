use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use utoipa::ToSchema;

use crate::api::dto::fields::FieldTypeSchema;
use crate::api::dto::members::timestamp;
use crate::application::fields::CUSTOM_FIELDS_PREFIX;
use crate::application::fields::permissions::{
    FieldPermissionInput, FieldPermissionMatrix, SaveFieldPermissionsCommand,
};
use crate::errors::AppError;

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = FieldAccess)]
pub enum FieldAccessSchema {
    Hidden,
    ReadOnly,
    Editable,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct FieldPermissionRoleResponse {
    pub id: String,
    #[schema(example = "member")]
    pub key: String,
    pub name: String,
    pub is_system: bool,
    pub is_configurable: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct FieldPermissionCellResponse {
    pub role_id: String,
    #[schema(value_type = FieldAccessSchema)]
    pub access: &'static str,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct FieldPermissionFieldResponse {
    pub id: String,
    pub key: String,
    pub label: String,
    #[schema(value_type = FieldTypeSchema)]
    pub field_type: String,
    pub is_system: bool,
    pub is_required: bool,
    pub is_active: bool,
    pub is_personal: bool,
    pub is_sensitive: bool,
    pub is_locked: bool,
    pub access: Vec<FieldPermissionCellResponse>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct FieldPermissionMatrixResponse {
    #[schema(example = "contacts.contact")]
    pub record_type: String,
    pub roles: Vec<FieldPermissionRoleResponse>,
    pub fields: Vec<FieldPermissionFieldResponse>,
    #[schema(example = "2026-10-20T13:45:00Z")]
    pub updated_at: Option<String>,
}

impl From<FieldPermissionMatrix> for FieldPermissionMatrixResponse {
    fn from(matrix: FieldPermissionMatrix) -> Self {
        Self {
            record_type: matrix.record_type.as_str().to_owned(),
            roles: matrix
                .roles
                .iter()
                .map(|role| FieldPermissionRoleResponse {
                    id: role.id.to_string(),
                    key: role.key.clone(),
                    name: role.name.clone(),
                    is_system: role.is_system,
                    is_configurable: !role.is_admin(),
                })
                .collect(),
            fields: matrix
                .fields
                .into_iter()
                .map(|entry| {
                    let definition = entry.field.definition;
                    FieldPermissionFieldResponse {
                        id: definition.id.to_string(),
                        key: definition.key.as_str().to_owned(),
                        label: definition.label,
                        field_type: definition.field_type.as_str().to_owned(),
                        is_system: definition.is_system,
                        is_required: definition.is_required,
                        is_active: definition.is_active,
                        is_personal: definition.is_personal,
                        is_sensitive: definition.is_sensitive,
                        is_locked: entry.locked,
                        access: entry
                            .access
                            .into_iter()
                            .map(|(role_id, access)| FieldPermissionCellResponse {
                                role_id: role_id.to_string(),
                                access: access.as_str(),
                            })
                            .collect(),
                    }
                })
                .collect(),
            updated_at: matrix.updated_at.map(timestamp),
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct FieldPermissionEntryRequest {
    pub field_id: String,
    pub role_id: String,
    #[schema(value_type = FieldAccessSchema)]
    pub access: String,
}

#[derive(Debug, Deserialize, ToSchema)]
#[schema(example = json!({
    "entries": [
        { "field_id": "0192f0c4-0000-7000-8000-000000000001", "role_id": "0192f0c4-0000-7000-8000-000000000002", "access": "hidden" }
    ]
}))]
pub struct SaveFieldPermissionsRequest {
    pub entries: Vec<FieldPermissionEntryRequest>,
}

impl SaveFieldPermissionsRequest {
    pub fn into_command(
        self,
        ip: Option<String>,
        expected_updated_at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> SaveFieldPermissionsCommand {
        SaveFieldPermissionsCommand {
            entries: self
                .entries
                .into_iter()
                .map(|entry| FieldPermissionInput {
                    field_id: entry.field_id,
                    role_id: entry.role_id,
                    access: entry.access,
                })
                .collect(),
            expected_updated_at,
            ip,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FieldRedaction {
    system: BTreeSet<String>,
    custom: BTreeSet<String>,
}

impl FieldRedaction {
    pub fn new(system: BTreeSet<String>, custom: BTreeSet<String>) -> Self {
        Self { system, custom }
    }

    pub fn is_empty(&self) -> bool {
        self.system.is_empty() && self.custom.is_empty()
    }

    pub fn hides(&self, key: &str) -> bool {
        self.system.contains(key)
    }

    pub fn hides_path(&self, path: &str) -> bool {
        match path
            .strip_prefix(CUSTOM_FIELDS_PREFIX)
            .and_then(|rest| rest.strip_prefix('.'))
        {
            Some(key) => self.custom.contains(key.split('.').next().unwrap_or(key)),
            None => self.hides(path.split('.').next().unwrap_or(path)),
        }
    }

    pub fn redact_object(&self, object: &mut Map<String, Value>) {
        for key in &self.system {
            object.remove(key);
            if let Some(base) = key.strip_suffix("_id") {
                object.remove(base);
                object.remove(&format!("{base}_note"));
            }
            if let Some(base) = key
                .strip_suffix("_latitude")
                .or_else(|| key.strip_suffix("_longitude"))
            {
                object.remove(base);
            }
        }
        if let Some(Value::Object(custom)) = object.get_mut(CUSTOM_FIELDS_PREFIX) {
            for key in &self.custom {
                custom.remove(key);
            }
        }
    }

    pub fn redact_value(&self, value: &mut Value) {
        if let Value::Object(object) = value {
            self.redact_object(object);
        }
    }

    pub fn redact_nested(&self, value: &mut Value, key: &str) {
        if let Some(nested) = value.get_mut(key) {
            self.redact_value(nested);
        }
    }

    pub fn redact_previous(&self, previous: &mut Value) {
        if let Value::Object(object) = previous {
            object.retain(|path, _| !self.hides_path(path));
        }
    }

    pub fn item<T: Serialize>(&self, item: T) -> Result<Value, AppError> {
        let mut value = serde_json::to_value(item)
            .map_err(|error| AppError::Internal(format!("failed to serialize record: {error}")))?;
        self.redact_value(&mut value);
        Ok(value)
    }

    pub fn items<T: Serialize>(&self, items: Vec<T>) -> Result<Vec<Value>, AppError> {
        items.into_iter().map(|item| self.item(item)).collect()
    }
}
