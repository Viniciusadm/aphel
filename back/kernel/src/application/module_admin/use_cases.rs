use serde_json::{Map, Value, json};

use crate::application::access::Authorizer;
use crate::application::module_admin::{
    DependencyStatus, ModuleDataExport, ModuleDescriptor, ModuleLifecycle, ModuleLink, ModulePorts,
    ModuleState, ModuleStateRepository, ModuleView, missing_required, parse_settings,
    with_defaults,
};
use crate::application::secrets::{SecretKey, SecretVault};
use crate::application::security::Clock;
use crate::application::shared::{
    Actor, AuditEntry, AuditLog, DomainEvent, EventPublisher, FieldChange, RecordRef, RecordType,
    TransactionManager,
};
use crate::domain::Secret;
use crate::domain::access::{AccessScope, Permission};
use crate::errors::{AppError, FieldViolation};

pub const MODULE_NOT_FOUND: &str = "module_not_found";
pub const MODULE_INCOMPATIBLE: &str = "module_incompatible";
pub const MODULE_DEPENDENCY_DISABLED: &str = "module_dependency_disabled";
pub const MODULE_REQUIRED_BY: &str = "module_required_by";
pub const MODULE_REQUIRED_BY_EXTENSION: &str = "module_required_by_extension";
pub const MODULE_SETTINGS_REQUIRED: &str = "module_settings_required";
pub const MODULE_ENABLED: &str = "module_enabled";
pub const MODULE_EXPORT_REQUIRED: &str = "module_export_required";
pub const MODULE_CONFIRMATION_REQUIRED: &str = "module_confirmation_required";
pub const MODULE_REQUIRED: &str = "module_required";
pub const MODULE_HELD_BY_EXTENSION: &str = "module_held_by_extension";

pub const MODULE_ENABLED_EVENT: &str = "module.enabled";
pub const MODULE_DISABLED_EVENT: &str = "module.disabled";
pub const MODULE_SETTINGS_UPDATED: &str = "module.settings_updated";
pub const MODULE_DATA_EXPORTED: &str = "module.data_exported";
pub const MODULE_DATA_PURGED: &str = "module.data_purged";

pub const SECRET_OWNER_TYPE: &str = "module";
pub const MODULE_RECORD_TYPE: RecordType = RecordType::from_static("module");

pub fn module_not_found() -> AppError {
    AppError::not_found_code(MODULE_NOT_FOUND, "module not found")
}

pub struct ModuleUseCases<P> {
    ports: P,
}

impl<P> ModuleUseCases<P>
where
    P: ModulePorts,
{
    pub fn new(ports: P) -> Self {
        Self { ports }
    }

    pub async fn list(&self, scope: &AccessScope) -> Result<Vec<ModuleView>, AppError> {
        let manage = scope.scope_for(&Permission::SETTINGS_MANAGE).is_some();
        let descriptors = self.ports.runtime().descriptors();
        let states = self.ports.modules().list().await?;
        let held = self.ports.runtime().held_modules().await?;
        let mut extension_dependents = self
            .ports
            .runtime()
            .extension_dependents()
            .await
            .unwrap_or_else(|error| {
                tracing::warn!("could not list the extensions that use modules: {error}");
                Default::default()
            });
        let secrets = if manage {
            self.configured_secrets(scope, &descriptors).await?
        } else {
            Vec::new()
        };
        Ok(descriptors
            .iter()
            .map(|descriptor| {
                let mut view = self.view(descriptor, &descriptors, &states, &secrets, manage);
                view.held_by = held.get(descriptor.key()).cloned();
                view.extension_dependents = extension_dependents
                    .remove(descriptor.key())
                    .unwrap_or_default();
                view
            })
            .collect())
    }

    pub async fn get(&self, scope: &AccessScope, key: &str) -> Result<ModuleView, AppError> {
        self.list(scope)
            .await?
            .into_iter()
            .find(|view| view.descriptor.key() == key)
            .ok_or_else(module_not_found)
    }

    pub async fn enable(&self, scope: &AccessScope, key: &str) -> Result<ModuleView, AppError> {
        scope.require(&Permission::SETTINGS_MANAGE)?;
        let view = self.get(scope, key).await?;
        if !view.descriptor.compatibility.is_compatible() {
            return Err(AppError::conflict_code(
                MODULE_INCOMPATIBLE,
                "this module is not compatible with the installed Aphel or its dependencies",
            ));
        }
        if let Some(dependency) = view
            .dependencies
            .iter()
            .find(|dependency| !dependency.enabled)
        {
            return Err(AppError::conflict_code(
                MODULE_DEPENDENCY_DISABLED,
                format!("enable the module {} first", dependency.key),
            ));
        }
        let upgrading = view.state.as_ref().is_some_and(|state| {
            state.enabled && state.installed_version != view.descriptor.manifest.version
        });
        if view.enabled() && !upgrading {
            return Ok(view);
        }
        if let Some(holder) = &view.held_by {
            return Err(AppError::conflict_code(
                MODULE_HELD_BY_EXTENSION,
                format!("this module is kept disabled by {holder}"),
            ));
        }
        let missing = missing_required(
            &view.descriptor.manifest.settings,
            &view.settings,
            &view.configured_secrets,
        );
        if !missing.is_empty() {
            return Err(AppError::invalid_fields_code(
                MODULE_SETTINGS_REQUIRED,
                "configure the required settings before enabling the module",
                missing,
            ));
        }
        let now = self.ports.clock().now();
        let mut state = view
            .state
            .clone()
            .unwrap_or_else(|| ModuleState::new(key, view.descriptor.manifest.version, now));
        let previous_version = state.installed_version.clone();
        state.enabled = true;
        state.enabled_at = Some(now);
        state.installed_version = view.descriptor.manifest.version.to_owned();
        state.updated_at = now;
        self.save(
            scope,
            &state,
            MODULE_ENABLED_EVENT,
            json!({ "key": key, "version": state.installed_version, "previous_version": previous_version }),
            if upgrading {
                vec![FieldChange::new(
                    "installed_version",
                    json!(previous_version),
                    json!(view.descriptor.manifest.version),
                )]
            } else {
                vec![FieldChange::new("enabled", json!(false), json!(true))]
            },
        )
        .await?;
        if let Err(error) = self.ports.runtime().on_enable(scope, key).await {
            tracing::error!(module = key, "on_enable failed: {error}");
        }
        self.get(scope, key).await
    }

    pub async fn disable(&self, scope: &AccessScope, key: &str) -> Result<ModuleView, AppError> {
        scope.require(&Permission::SETTINGS_MANAGE)?;
        let view = self.get(scope, key).await?;
        if view.descriptor.required {
            return Err(AppError::conflict_code(
                MODULE_REQUIRED,
                "this module is required by this build and cannot be disabled",
            ));
        }
        if !view.enabled() {
            return Ok(view);
        }
        if !view.enabled_dependents.is_empty() {
            return Err(AppError::conflict_code(
                MODULE_REQUIRED_BY,
                format!(
                    "disable the dependent modules first: {}",
                    view.enabled_dependents.join(", ")
                ),
            ));
        }
        let extension_dependents = self
            .ports
            .runtime()
            .extension_dependents()
            .await?
            .remove(key)
            .unwrap_or_default();
        if !extension_dependents.is_empty() {
            return Err(AppError::conflict_code(
                MODULE_REQUIRED_BY_EXTENSION,
                format!(
                    "disable or uninstall the extensions that use this module first: {}",
                    extension_dependents.join(", ")
                ),
            ));
        }
        let now = self.ports.clock().now();
        let Some(mut state) = view.state.clone() else {
            return Ok(view);
        };
        state.enabled = false;
        state.disabled_at = Some(now);
        state.updated_at = now;
        self.save(
            scope,
            &state,
            MODULE_DISABLED_EVENT,
            json!({ "key": key }),
            vec![FieldChange::new("enabled", json!(true), json!(false))],
        )
        .await?;
        if let Err(error) = self.ports.runtime().on_disable(scope, key).await {
            tracing::error!(module = key, "on_disable failed: {error}");
        }
        self.get(scope, key).await
    }

    pub async fn update_settings(
        &self,
        scope: &AccessScope,
        key: &str,
        input: Map<String, Value>,
    ) -> Result<ModuleView, AppError> {
        scope.require(&Permission::SETTINGS_MANAGE)?;
        let view = self.get(scope, key).await?;
        let fields = &view.descriptor.manifest.settings;
        let now = self.ports.clock().now();
        let mut state = view
            .state
            .clone()
            .unwrap_or_else(|| ModuleState::new(key, view.descriptor.manifest.version, now));
        let change = parse_settings(fields, &state.settings, &input)
            .map_err(|violations| AppError::invalid_fields("invalid settings", violations))?;
        let mut violations = self
            .ports
            .runtime()
            .validate_settings(key, &Value::Object(with_defaults(fields, &change.values)));
        let mut configured = view.configured_secrets.clone();
        for (secret, value) in &change.secrets {
            configured.retain(|existing| existing != secret);
            if value.is_some() {
                configured.push(secret.clone());
            }
        }
        if view.enabled() {
            violations.extend(missing_required(fields, &change.values, &configured));
        }
        if !violations.is_empty() {
            return Err(AppError::invalid_fields("invalid settings", violations));
        }
        let mut changes: Vec<FieldChange> = fields
            .iter()
            .filter(|field| !field.is_secret())
            .filter_map(|field| {
                let old = state
                    .settings
                    .get(field.key)
                    .cloned()
                    .unwrap_or(Value::Null);
                let new = change.values.get(field.key).cloned().unwrap_or(Value::Null);
                (old != new).then(|| FieldChange::new(format!("settings.{}", field.key), old, new))
            })
            .collect();
        changes.extend(
            change
                .secrets
                .iter()
                .map(|(secret, _)| FieldChange::redacted(format!("settings.{secret}"))),
        );
        state.settings = change.values;
        state.updated_at = now;

        let mut tx = self.ports.transactions().begin().await?;
        for (secret, value) in &change.secrets {
            let secret_key = SecretKey::new(SECRET_OWNER_TYPE, key, secret.as_str());
            match value {
                Some(value) => {
                    self.ports
                        .secrets()
                        .store(&mut tx, scope, &secret_key, &Secret::new(value.clone()))
                        .await?
                }
                None => {
                    self.ports
                        .secrets()
                        .remove(&mut tx, scope, &secret_key)
                        .await?
                }
            }
        }
        self.write(
            &mut tx,
            scope,
            &state,
            MODULE_SETTINGS_UPDATED,
            json!({ "key": key, "fields": changes.iter().map(|change| change.field.clone()).collect::<Vec<_>>() }),
            changes,
        )
        .await?;
        self.ports.transactions().commit(tx).await?;
        self.get(scope, key).await
    }

    pub async fn export_data(
        &self,
        scope: &AccessScope,
        key: &str,
    ) -> Result<ModuleDataExport, AppError> {
        scope.require(&Permission::SETTINGS_MANAGE)?;
        let view = self.get(scope, key).await?;
        let data = self.ports.runtime().export_data(scope, key).await?;
        let now = self.ports.clock().now();
        let mut state = view
            .state
            .clone()
            .unwrap_or_else(|| ModuleState::new(key, view.descriptor.manifest.version, now));
        state.data_exported_at = Some(now);
        state.updated_at = now;
        self.save(
            scope,
            &state,
            MODULE_DATA_EXPORTED,
            json!({ "key": key }),
            Vec::new(),
        )
        .await?;
        Ok(ModuleDataExport {
            key: key.to_owned(),
            version: view.descriptor.manifest.version.to_owned(),
            exported_at: now,
            data,
        })
    }

    pub async fn purge_data(
        &self,
        scope: &AccessScope,
        key: &str,
        confirmation: &str,
    ) -> Result<ModuleView, AppError> {
        scope.require(&Permission::SETTINGS_MANAGE)?;
        let view = self.get(scope, key).await?;
        if confirmation.trim() != key {
            return Err(AppError::invalid_fields_code(
                MODULE_CONFIRMATION_REQUIRED,
                "type the module key to confirm",
                vec![FieldViolation::new("confirm", "must be the module key")],
            ));
        }
        if view.enabled() {
            return Err(AppError::conflict_code(
                MODULE_ENABLED,
                "disable the module before removing its data",
            ));
        }
        let Some(mut state) = view.state.clone() else {
            return Ok(view);
        };
        if !state.export_is_current() {
            return Err(AppError::conflict_code(
                MODULE_EXPORT_REQUIRED,
                "export the data of the module before removing it",
            ));
        }
        let now = self.ports.clock().now();
        let mut tx = self.ports.transactions().begin().await?;
        let removed = self.ports.runtime().purge_data(&mut tx, scope, key).await?;
        self.ports
            .secrets()
            .remove_owner(&mut tx, scope, SECRET_OWNER_TYPE, key)
            .await?;
        state.settings = Map::new();
        state.data_purged_at = Some(now);
        state.updated_at = now;
        self.write(
            &mut tx,
            scope,
            &state,
            MODULE_DATA_PURGED,
            json!({ "key": key, "rows": removed }),
            Vec::new(),
        )
        .await?;
        self.ports.transactions().commit(tx).await?;
        self.get(scope, key).await
    }

    async fn configured_secrets(
        &self,
        scope: &AccessScope,
        descriptors: &[ModuleDescriptor],
    ) -> Result<Vec<SecretKey>, AppError> {
        let owners: Vec<String> = descriptors
            .iter()
            .filter(|descriptor| {
                descriptor
                    .manifest
                    .settings
                    .iter()
                    .any(|field| field.is_secret())
            })
            .map(|descriptor| descriptor.key().to_owned())
            .collect();
        if owners.is_empty() {
            return Ok(Vec::new());
        }
        self.ports
            .secrets()
            .configured(scope, SECRET_OWNER_TYPE, &owners)
            .await
    }

    fn view(
        &self,
        descriptor: &ModuleDescriptor,
        descriptors: &[ModuleDescriptor],
        states: &[ModuleState],
        secrets: &[SecretKey],
        manage: bool,
    ) -> ModuleView {
        let key = descriptor.key();
        let enabled = |key: &str| states.iter().any(|state| state.key == key && state.enabled);
        let state = states.iter().find(|state| state.key == key).cloned();
        let settings = if manage {
            with_defaults(
                &descriptor.manifest.settings,
                &state
                    .as_ref()
                    .map(|state| state.settings.clone())
                    .unwrap_or_default(),
            )
        } else {
            Map::new()
        };
        ModuleView {
            descriptor: descriptor.clone(),
            settings,
            configured_secrets: secrets
                .iter()
                .filter(|secret| secret.owner_id == key)
                .map(|secret| secret.name.clone())
                .collect(),
            dependencies: descriptor
                .manifest
                .dependencies
                .iter()
                .map(|dependency| DependencyStatus {
                    key: dependency.key.to_owned(),
                    requirement: dependency.version_req.to_owned(),
                    installed: descriptors
                        .iter()
                        .any(|candidate| candidate.key() == dependency.key),
                    enabled: enabled(dependency.key),
                })
                .collect(),
            optional_dependencies: descriptor
                .optional
                .iter()
                .filter(|optional| {
                    descriptors
                        .iter()
                        .any(|candidate| candidate.key() == **optional)
                })
                .map(|optional| ModuleLink {
                    key: (*optional).to_owned(),
                    enabled: enabled(optional),
                    optional: true,
                })
                .collect(),
            dependents: descriptors
                .iter()
                .filter_map(|candidate| {
                    let hard = candidate
                        .manifest
                        .dependencies
                        .iter()
                        .any(|dependency| dependency.key == key);
                    let soft = candidate.optional.contains(&key);
                    (hard || soft).then(|| ModuleLink {
                        key: candidate.key().to_owned(),
                        enabled: enabled(candidate.key()),
                        optional: !hard,
                    })
                })
                .collect(),
            enabled_dependents: descriptors
                .iter()
                .filter(|candidate| {
                    candidate
                        .manifest
                        .dependencies
                        .iter()
                        .any(|dependency| dependency.key == key)
                        && enabled(candidate.key())
                })
                .map(|candidate| candidate.key().to_owned())
                .collect(),
            extension_dependents: Vec::new(),
            held_by: None,
            state,
        }
    }

    async fn save(
        &self,
        scope: &AccessScope,
        state: &ModuleState,
        event_type: &str,
        payload: Value,
        changes: Vec<FieldChange>,
    ) -> Result<(), AppError> {
        let mut tx = self.ports.transactions().begin().await?;
        self.write(&mut tx, scope, state, event_type, payload, changes)
            .await?;
        self.ports.transactions().commit(tx).await
    }

    async fn write(
        &self,
        tx: &mut <P::Transactions as TransactionManager>::Tx,
        scope: &AccessScope,
        state: &ModuleState,
        event_type: &str,
        payload: Value,
        changes: Vec<FieldChange>,
    ) -> Result<(), AppError> {
        self.ports.modules().save(tx, scope, state).await?;
        let record = RecordRef::new(MODULE_RECORD_TYPE, state.record_id());
        let actor = Actor::from_scope(scope);
        let now = state.updated_at;
        self.ports
            .events()
            .publish(
                tx,
                DomainEvent::new(actor.clone(), event_type, now)
                    .with_record(record.clone())
                    .with_payload(payload),
            )
            .await?;
        self.ports
            .audit()
            .record(
                tx,
                AuditEntry::new(actor, event_type, now)
                    .with_record(record)
                    .with_changes(changes),
            )
            .await
    }
}
