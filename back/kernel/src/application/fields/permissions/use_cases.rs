use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Utc};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::application::access::Authorizer;
use crate::application::fields::permissions::{
    FIELD_PERMISSION_LOCKED, FieldAccess, FieldPermissionPorts, FieldPermissionRepository,
    PermissionRole, StoredFieldPermission, is_access_locked,
};
use crate::application::fields::{
    EntityDefinition, FieldDefinitionRepository, FieldSet, ResolvedField,
};
use crate::application::security::Clock;
use crate::application::shared::{
    Actor, AuditEntry, AuditLog, DomainEvent, EventPublisher, FieldChange, RecordType,
    TransactionManager, ensure_version,
};
use crate::domain::access::{AccessScope, Permission};
use crate::errors::{AppError, FieldViolation};

pub const FIELD_PERMISSIONS_UPDATED: &str = "field_permissions.updated";

#[derive(Debug, Clone, PartialEq)]
pub struct MatrixField {
    pub field: ResolvedField,
    pub locked: bool,
    pub access: BTreeMap<Uuid, FieldAccess>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldPermissionMatrix {
    pub record_type: RecordType,
    pub roles: Vec<PermissionRole>,
    pub fields: Vec<MatrixField>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldPermissionInput {
    pub field_id: String,
    pub role_id: String,
    pub access: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SaveFieldPermissionsCommand {
    pub entries: Vec<FieldPermissionInput>,
    pub expected_updated_at: Option<DateTime<Utc>>,
    pub ip: Option<String>,
}

pub struct FieldPermissionUseCases<P> {
    ports: P,
}

impl<P> FieldPermissionUseCases<P>
where
    P: FieldPermissionPorts,
{
    pub fn new(ports: P) -> Self {
        Self { ports }
    }

    pub async fn matrix(
        &self,
        scope: &AccessScope,
        record_type: &str,
    ) -> Result<FieldPermissionMatrix, AppError> {
        scope.require(&Permission::SETTINGS_MANAGE)?;
        let definition = self.ports.registry().require(record_type)?;
        let fields = self.field_set(definition).await?;
        let roles = self.ports.permissions().roles().await?;
        let state = self
            .ports
            .permissions()
            .state(&definition.record_type)
            .await?;
        let stored: BTreeMap<(Uuid, Uuid), FieldAccess> = state
            .entries
            .iter()
            .map(|entry| ((entry.field_id, entry.role_id), entry.access))
            .collect();
        let matrix_fields = fields
            .all()
            .iter()
            .map(|field| {
                let locked = is_access_locked(&definition.record_type, field);
                let access = roles
                    .iter()
                    .map(|role| {
                        let access = if locked || role.is_admin() {
                            FieldAccess::Editable
                        } else {
                            stored
                                .get(&(field.definition.id, role.id))
                                .copied()
                                .unwrap_or_default()
                        };
                        (role.id, access)
                    })
                    .collect();
                MatrixField {
                    field: field.clone(),
                    locked,
                    access,
                }
            })
            .collect();
        Ok(FieldPermissionMatrix {
            record_type: definition.record_type.clone(),
            roles,
            fields: matrix_fields,
            updated_at: state.updated_at,
        })
    }

    pub async fn save(
        &self,
        scope: &AccessScope,
        record_type: &str,
        command: SaveFieldPermissionsCommand,
    ) -> Result<FieldPermissionMatrix, AppError> {
        scope.require(&Permission::SETTINGS_MANAGE)?;
        let definition = self.ports.registry().require(record_type)?;
        let fields = self.field_set(definition).await?;
        let roles = self.ports.permissions().roles().await?;
        let state = self
            .ports
            .permissions()
            .state(&definition.record_type)
            .await?;
        if let Some(current) = state.updated_at {
            ensure_version(command.expected_updated_at, current)?;
        }

        let desired = self.validate(definition, &fields, &roles, &command.entries)?;
        let current: BTreeMap<(Uuid, Uuid), FieldAccess> = state
            .entries
            .iter()
            .map(|entry| ((entry.field_id, entry.role_id), entry.access))
            .collect();
        let changes = diff(&fields, &roles, &current, &desired);
        if changes.is_empty() {
            return self.matrix(scope, record_type).await;
        }

        let entries: Vec<StoredFieldPermission> = desired
            .iter()
            .map(|((field_id, role_id), access)| StoredFieldPermission {
                field_id: *field_id,
                role_id: *role_id,
                access: *access,
            })
            .collect();
        let now = self.ports.clock().now();
        let actor = Actor::from_scope(scope);
        let event =
            DomainEvent::new(actor.clone(), FIELD_PERMISSIONS_UPDATED, now).with_payload(json!({
                "record_type": definition.record_type.as_str(),
                "restrictions": entries.len(),
                "changed": changes.len(),
            }));
        let entry = AuditEntry::new(actor, FIELD_PERMISSIONS_UPDATED, now)
            .with_changes(changes)
            .with_ip(command.ip);

        let mut tx = self.ports.transactions().begin().await?;
        self.ports
            .permissions()
            .replace(&mut tx, scope, &definition.record_type, &entries, now)
            .await?;
        self.ports.events().publish(&mut tx, event).await?;
        self.ports.audit().record(&mut tx, entry).await?;
        self.ports.transactions().commit(tx).await?;
        self.matrix(scope, record_type).await
    }

    async fn field_set(&self, definition: &EntityDefinition) -> Result<FieldSet, AppError> {
        Ok(FieldSet::new(
            definition,
            self.ports
                .definitions()
                .list(&definition.record_type)
                .await?,
        ))
    }

    fn validate(
        &self,
        definition: &EntityDefinition,
        fields: &FieldSet,
        roles: &[PermissionRole],
        inputs: &[FieldPermissionInput],
    ) -> Result<BTreeMap<(Uuid, Uuid), FieldAccess>, AppError> {
        let mut violations = Vec::new();
        let mut locked = Vec::new();
        let mut desired = BTreeMap::new();
        let mut seen = BTreeSet::new();
        for (index, input) in inputs.iter().enumerate() {
            let path = format!("entries.{index}");
            let field = Uuid::parse_str(&input.field_id)
                .ok()
                .and_then(|id| fields.find_by_id(id));
            let role = Uuid::parse_str(&input.role_id)
                .ok()
                .and_then(|id| roles.iter().find(|role| role.id == id));
            let access = FieldAccess::parse(&input.access);
            let (Some(field), Some(role), Some(access)) = (field, role, access) else {
                if field.is_none() {
                    violations.push(FieldViolation::new(
                        format!("{path}.field_id"),
                        "is not a field of this record type",
                    ));
                }
                if role.is_none() {
                    violations.push(FieldViolation::new(
                        format!("{path}.role_id"),
                        "is not a role of the organization",
                    ));
                }
                if access.is_none() {
                    violations.push(FieldViolation::new(
                        format!("{path}.access"),
                        "must be one of hidden, read_only, editable",
                    ));
                }
                continue;
            };
            if !seen.insert((field.definition.id, role.id)) {
                violations.push(FieldViolation::new(path, "is duplicated"));
                continue;
            }
            if access == FieldAccess::Editable {
                continue;
            }
            if role.is_admin() {
                violations.push(FieldViolation::new(
                    format!("{path}.role_id"),
                    "the admin role always edits every field",
                ));
                continue;
            }
            if is_access_locked(&definition.record_type, field) {
                locked.push(FieldViolation::new(
                    format!("{path}.access"),
                    format!("the field {} cannot be restricted", field.key()),
                ));
                continue;
            }
            desired.insert((field.definition.id, role.id), access);
        }
        if !violations.is_empty() {
            return Err(AppError::invalid_fields(
                "invalid field permissions",
                violations,
            ));
        }
        if !locked.is_empty() {
            return Err(AppError::invalid_fields_code(
                FIELD_PERMISSION_LOCKED,
                "required and structural fields are always editable",
                locked,
            ));
        }
        Ok(desired)
    }
}

fn diff(
    fields: &FieldSet,
    roles: &[PermissionRole],
    current: &BTreeMap<(Uuid, Uuid), FieldAccess>,
    desired: &BTreeMap<(Uuid, Uuid), FieldAccess>,
) -> Vec<FieldChange> {
    let keys: BTreeSet<&(Uuid, Uuid)> = current.keys().chain(desired.keys()).collect();
    keys.into_iter()
        .filter_map(|cell| {
            let old = current.get(cell).copied().unwrap_or_default();
            let new = desired.get(cell).copied().unwrap_or_default();
            if old == new {
                return None;
            }
            let field = fields
                .find_by_id(cell.0)
                .map(|field| field.key().to_owned())
                .unwrap_or_else(|| cell.0.to_string());
            let role = roles
                .iter()
                .find(|role| role.id == cell.1)
                .map(|role| role.key.clone())
                .unwrap_or_else(|| cell.1.to_string());
            Some(FieldChange::new(
                format!("{field}.{role}"),
                Value::String(old.as_str().to_owned()),
                Value::String(new.as_str().to_owned()),
            ))
        })
        .collect()
}
