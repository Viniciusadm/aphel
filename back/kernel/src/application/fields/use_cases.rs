use std::collections::BTreeSet;

use serde::Serialize;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::application::access::{Authorizer, PERMISSION_DENIED};
use crate::application::audit::{DiffOptions, diff_changes};
use crate::application::fields::definition_violations;
use crate::application::fields::permissions::FieldAccessPolicy;
use crate::application::fields::{
    CreateFieldCommand, CustomFieldValueRepository, EntityDefinition, FieldDefinitionRepository,
    FieldOptionInput, FieldPorts, FieldSchema, FieldSet, FieldView, FieldsTx, OperatorDescription,
    RecordTypeRegistry, ReorderCommand, ResolvedField, UpdateFieldCommand, UpdateOptionCommand,
    config_from_json, config_to_json, is_option_value,
};
use crate::application::organization_admin::{ListWindow, Listing};
use crate::application::query::{FilterOperator, operators_for, operators_for_definition};
use crate::application::security::Clock;
use crate::application::shared::{
    Actor, AuditEntry, AuditLog, DomainEvent, EventPublisher, FieldChange, RecordRef, RecordType,
    TransactionManager, ensure_version,
};
use crate::domain::access::{AccessScope, Permission};
use crate::domain::fields::{
    FORMULA_FIELD_TYPE, FieldConfig, FieldDefinition, FieldKey, FieldOption, FieldType,
    resolve_declared_type, validate_value,
};
use crate::errors::{AppError, FieldViolation};

pub const FIELD_NOT_FOUND: &str = "field_not_found";
pub const FIELD_OPTION_NOT_FOUND: &str = "field_option_not_found";
pub const FIELD_KEY_TAKEN: &str = "field_key_taken";
pub const OPTION_VALUE_TAKEN: &str = "option_value_taken";
pub const FIELD_MANAGED_BY_MODULE: &str = "field_managed_by_module";
pub const SYSTEM_FIELD_LOCKED: &str = "system_field_locked";
pub const INCOMPATIBLE_FIELD_TYPE: &str = "incompatible_field_type";
pub const DUPLICATE_VALUES: &str = "duplicate_values";
pub const TOO_MANY_FIELDS: &str = "too_many_fields";

pub const FIELD_CREATED: &str = "field.created";
pub const FIELD_UPDATED: &str = "field.updated";
pub const FIELD_DEACTIVATED: &str = "field.deactivated";
pub const FIELD_ACTIVATED: &str = "field.activated";
pub const FIELDS_REORDERED: &str = "field.reordered";
pub const FIELD_OPTION_CREATED: &str = "field.option_created";
pub const FIELD_OPTION_UPDATED: &str = "field.option_updated";
pub const FIELD_OPTIONS_REORDERED: &str = "field.options_reordered";

pub const MAX_LABEL_LENGTH: usize = 140;
pub const MAX_CUSTOM_FIELDS: usize = 200;
pub const MAX_OPTIONS: usize = 500;

pub const FIELD_RECORD_TYPE: RecordType = RecordType::from_static("field");

#[derive(Serialize)]
struct FieldSnapshot<'a> {
    label: &'a str,
    field_type: &'static str,
    is_required: bool,
    is_unique: bool,
    is_searchable: bool,
    is_personal: bool,
    is_sensitive: bool,
    is_active: bool,
    config: Value,
}

impl<'a> From<&'a FieldDefinition> for FieldSnapshot<'a> {
    fn from(field: &'a FieldDefinition) -> Self {
        Self {
            label: &field.label,
            field_type: field.type_name(),
            is_required: field.is_required,
            is_unique: field.is_unique,
            is_searchable: field.is_searchable,
            is_personal: field.is_personal,
            is_sensitive: field.is_sensitive,
            is_active: field.is_active,
            config: config_to_json(&field.config),
        }
    }
}

#[derive(Serialize)]
struct OptionSnapshot<'a> {
    label: &'a str,
    color: Option<&'a str>,
    is_active: bool,
}

impl<'a> From<&'a FieldOption> for OptionSnapshot<'a> {
    fn from(option: &'a FieldOption) -> Self {
        Self {
            label: &option.label,
            color: option.color.as_deref(),
            is_active: option.is_active,
        }
    }
}

pub struct FieldUseCases<P> {
    ports: P,
    custom_object_field_limit: usize,
}

impl<P> FieldUseCases<P>
where
    P: FieldPorts,
{
    pub fn new(ports: P) -> Self {
        Self {
            ports,
            custom_object_field_limit: MAX_CUSTOM_FIELDS,
        }
    }

    pub fn with_custom_object_field_limit(mut self, limit: usize) -> Self {
        self.custom_object_field_limit = limit;
        self
    }

    pub async fn list_fields(
        &self,
        scope: &AccessScope,
        record_type: &str,
        include_inactive: bool,
        window: ListWindow,
    ) -> Result<Listing<FieldView>, AppError> {
        let (_, fields) = self.readable_fields(scope, record_type).await?;
        let manages = scope.require(&Permission::SETTINGS_MANAGE).is_ok();
        let all: Vec<FieldView> = fields
            .all()
            .iter()
            .filter(|field| include_inactive || field.definition.is_active)
            .filter(|field| manages || !field.is_hidden())
            .map(field_view)
            .collect();
        let total = all.len() as u64;
        Ok(Listing {
            items: all
                .into_iter()
                .skip(usize::try_from(window.offset).unwrap_or(usize::MAX))
                .take(usize::try_from(window.limit).unwrap_or(usize::MAX))
                .collect(),
            total,
        })
    }

    pub async fn schema(
        &self,
        scope: &AccessScope,
        record_type: &str,
    ) -> Result<FieldSchema, AppError> {
        let (definition, fields) = self.readable_fields(scope, record_type).await?;
        let views: Vec<FieldView> = fields
            .active()
            .map(|field| {
                let mut view = field_view(field);
                view.definition.options.retain(|option| option.is_active);
                view
            })
            .collect();
        let mut used: Vec<FilterOperator> = Vec::new();
        for operator in FieldType::ALL
            .into_iter()
            .flat_map(operators_for)
            .chain([FilterOperator::InGroup])
        {
            if !used.contains(&operator) {
                used.push(operator);
            }
        }
        Ok(FieldSchema {
            record_type: definition.record_type.as_str().to_owned(),
            fields: views,
            operators: used
                .into_iter()
                .map(|operator| OperatorDescription {
                    operator,
                    operand: operator.value_kind(),
                })
                .collect(),
        })
    }

    pub async fn create_field(
        &self,
        scope: &AccessScope,
        record_type: &str,
        command: CreateFieldCommand,
    ) -> Result<FieldView, AppError> {
        scope.require(&Permission::SETTINGS_MANAGE)?;
        let (definition, fields) = self.load_fields(record_type).await?;
        let mut violations = Vec::new();

        let key = FieldKey::new(command.key.trim()).ok();
        match &key {
            None => violations.push(FieldViolation::new(
                "key",
                "must be snake_case: lowercase letters, digits and single underscores, starting with a letter (max 64)",
            )),
            Some(key) if definition.system_field_by_key(key.as_str()).is_some() => {
                violations.push(FieldViolation::new("key", "is reserved by a system field"))
            }
            Some(key) if definition.relation_by_key(key.as_str()).is_some() => violations.push(
                FieldViolation::new("key", "is reserved by a relationship of the record type"),
            ),
            Some(_) => {}
        }
        let label = bounded_label(&command.label);
        if label.is_none() {
            violations.push(label_violation("label"));
        }
        let config = match config_from_json(&command.config) {
            Ok(config) => Some(config),
            Err(errors) => {
                violations.extend(errors.into_iter().map(|error| {
                    FieldViolation::new(format!("config.{}", error.property), error.message)
                }));
                None
            }
        };
        let field_type =
            declared_field_type(command.field_type.trim(), config.as_ref(), &mut violations);
        let (Some(key), Some(label), Some(field_type), Some(config)) =
            (key, label, field_type, config)
        else {
            return Err(AppError::invalid_fields("invalid field", violations));
        };

        let options = build_options(field_type, &command.options, &mut violations);
        let field = FieldDefinition {
            id: Uuid::now_v7(),
            record_type: definition.record_type.as_str().to_owned(),
            key,
            label,
            field_type,
            is_system: false,
            column_name: None,
            is_required: command.is_required,
            is_unique: command.is_unique,
            is_searchable: command.is_searchable,
            is_personal: command.is_personal || command.is_sensitive,
            is_sensitive: command.is_sensitive,
            module_key: None,
            is_active: true,
            position: fields
                .all()
                .iter()
                .map(|field| field.definition.position)
                .max()
                .map_or(0, |position| position + 1),
            config,
            options,
        };
        violations.extend(self.field_violations(&field));
        if violations.is_empty() && field.is_formula() {
            violations.extend(
                definition_violations(self.ports.registry(), self.ports.definitions(), &field)
                    .await?,
            );
        }
        if !violations.is_empty() {
            return Err(AppError::invalid_fields("invalid field", violations));
        }
        if fields.find(field.key.as_str()).is_some() {
            return Err(AppError::conflict_code(
                FIELD_KEY_TAKEN,
                format!("a field with the key {} already exists", field.key),
            ));
        }
        let limit = if fields.record_type.is_custom() {
            self.custom_object_field_limit
        } else {
            MAX_CUSTOM_FIELDS
        };
        if fields
            .all()
            .iter()
            .filter(|field| !field.is_system())
            .count()
            >= limit
        {
            return Err(AppError::validation_code(
                TOO_MANY_FIELDS,
                format!("a record type can have at most {limit} custom fields"),
            ));
        }

        let changes = diff_changes(
            &Value::Null,
            &FieldSnapshot::from(&field),
            &DiffOptions::default(),
        )?;
        let mut tx = self.ports.transactions().begin().await?;
        self.ports.definitions().insert(&mut tx, &field).await?;
        self.record(
            &mut tx,
            scope,
            FIELD_CREATED,
            &field,
            json!({ "options": field.options.len() }),
            changes,
            command.ip,
        )
        .await?;
        self.ports.transactions().commit(tx).await?;
        self.require_view(definition, field.id).await
    }

    pub async fn update_field(
        &self,
        scope: &AccessScope,
        record_type: &str,
        field_id: &str,
        command: UpdateFieldCommand,
    ) -> Result<FieldView, AppError> {
        scope.require(&Permission::SETTINGS_MANAGE)?;
        let (definition, fields) = self.load_fields(record_type).await?;
        let current = require_field(&fields, field_id)?;
        ensure_unmanaged(&current.definition)?;
        if let Some(updated_at) = current.updated_at {
            ensure_version(command.expected_updated_at, updated_at)?;
        }
        let mut updated = current.definition.clone();
        let mut violations = Vec::new();

        if current.is_system()
            && (command.field_type.is_some()
                || command.is_required.is_some()
                || command.is_unique.is_some()
                || command.is_searchable.is_some()
                || command.config.is_some())
        {
            return Err(system_field_locked());
        }
        if let Some(label) = &command.label {
            match bounded_label(label) {
                Some(label) => updated.label = label,
                None => violations.push(label_violation("label")),
            }
        }
        let declared = command.field_type.as_deref().map(str::trim);
        if let Some(declared) = declared
            && (declared == FORMULA_FIELD_TYPE) != current.definition.is_formula()
        {
            return Err(AppError::validation_code(
                INCOMPATIBLE_FIELD_TYPE,
                format!(
                    "a {} field cannot become {declared}",
                    current.definition.type_name()
                ),
            ));
        }
        if let Some(field_type) = declared.filter(|declared| *declared != FORMULA_FIELD_TYPE) {
            match FieldType::parse(field_type) {
                Some(field_type) if updated.field_type.can_change_to(field_type) => {
                    updated.field_type = field_type
                }
                Some(field_type) => {
                    return Err(AppError::validation_code(
                        INCOMPATIBLE_FIELD_TYPE,
                        format!(
                            "a {} field cannot become {}",
                            updated.field_type.as_str(),
                            field_type.as_str()
                        ),
                    ));
                }
                None => violations.push(FieldViolation::new(
                    "field_type",
                    "is not a known field type",
                )),
            }
        }
        if let Some(config) = &command.config {
            match config_from_json(config) {
                Ok(config) => updated.config = config,
                Err(errors) => violations.extend(errors.into_iter().map(|error| {
                    FieldViolation::new(format!("config.{}", error.property), error.message)
                })),
            }
        }
        if command.config.is_some() && !current.is_system() {
            match (current.definition.is_formula(), &updated.config.formula) {
                (true, Some(formula)) => updated.field_type = formula.result_type.field_type(),
                (true, None) => violations.push(FieldViolation::new(
                    "config.expression",
                    "is required for formula fields",
                )),
                (false, Some(_)) => violations.push(FieldViolation::new(
                    "config.expression",
                    "is only accepted by formula fields",
                )),
                (false, None) => {}
            }
        }
        if let Some(is_required) = command.is_required {
            updated.is_required = is_required;
        }
        if let Some(is_unique) = command.is_unique {
            updated.is_unique = is_unique;
        }
        if let Some(is_searchable) = command.is_searchable {
            updated.is_searchable = is_searchable;
        }
        if let Some(is_personal) = command.is_personal {
            updated.is_personal = is_personal;
        }
        if let Some(is_sensitive) = command.is_sensitive {
            updated.is_sensitive = is_sensitive;
        }
        if updated.is_sensitive {
            updated.is_personal = true;
        }
        if !current.is_system() {
            violations.extend(self.field_violations(&updated));
        }
        if violations.is_empty() && command.config.is_some() && updated.is_formula() {
            violations.extend(
                definition_violations(self.ports.registry(), self.ports.definitions(), &updated)
                    .await?,
            );
        }
        if !violations.is_empty() {
            return Err(AppError::invalid_fields("invalid field", violations));
        }
        if updated.is_unique
            && !current.definition.is_unique
            && self
                .ports
                .values()
                .has_duplicates(
                    self.ports.registry().require(&updated.record_type)?,
                    &updated,
                )
                .await?
        {
            return Err(AppError::conflict_code(
                DUPLICATE_VALUES,
                "existing records share values of this field, so it cannot be unique",
            ));
        }

        let changes = diff_changes(
            &FieldSnapshot::from(&current.definition),
            &FieldSnapshot::from(&updated),
            &DiffOptions::default(),
        )?;
        if changes.is_empty() {
            return Ok(field_view(current));
        }
        let mut tx = self.ports.transactions().begin().await?;
        self.ports.definitions().update(&mut tx, &updated).await?;
        if updated.is_unique != current.definition.is_unique {
            self.ports
                .values()
                .rebuild_unique(
                    &mut tx,
                    self.ports.registry().require(&updated.record_type)?,
                    &updated,
                )
                .await?;
        }
        self.record(
            &mut tx,
            scope,
            FIELD_UPDATED,
            &updated,
            json!({}),
            changes,
            command.ip,
        )
        .await?;
        self.ports.transactions().commit(tx).await?;
        self.require_view(definition, updated.id).await
    }

    pub async fn set_field_active(
        &self,
        scope: &AccessScope,
        record_type: &str,
        field_id: &str,
        active: bool,
        ip: Option<String>,
    ) -> Result<FieldView, AppError> {
        scope.require(&Permission::SETTINGS_MANAGE)?;
        let (definition, fields) = self.load_fields(record_type).await?;
        let current = require_field(&fields, field_id)?;
        ensure_unmanaged(&current.definition)?;
        if current.is_system() {
            return Err(system_field_locked());
        }
        if current.definition.is_active == active {
            return Ok(field_view(current));
        }
        let mut updated = current.definition.clone();
        updated.is_active = active;
        let changes = vec![FieldChange::new("is_active", json!(!active), json!(active))];
        let action = if active {
            FIELD_ACTIVATED
        } else {
            FIELD_DEACTIVATED
        };
        let mut tx = self.ports.transactions().begin().await?;
        self.ports.definitions().update(&mut tx, &updated).await?;
        self.record(&mut tx, scope, action, &updated, json!({}), changes, ip)
            .await?;
        self.ports.transactions().commit(tx).await?;
        self.require_view(definition, updated.id).await
    }

    pub async fn reorder_fields(
        &self,
        scope: &AccessScope,
        record_type: &str,
        command: ReorderCommand,
    ) -> Result<Vec<FieldView>, AppError> {
        scope.require(&Permission::SETTINGS_MANAGE)?;
        let (definition, fields) = self.load_fields(record_type).await?;
        let current: Vec<Uuid> = fields
            .all()
            .iter()
            .map(|field| field.definition.id)
            .collect();
        let ordered = permutation(&command.ids, &current, "ids")?;
        if ordered == current {
            return Ok(fields.all().iter().map(field_view).collect());
        }
        let actor = Actor::from_scope(scope);
        let now = self.ports.clock().now();
        let keys: Vec<&str> = ordered
            .iter()
            .filter_map(|id| fields.find_by_id(*id).map(ResolvedField::key))
            .collect();
        let old_keys: Vec<&str> = fields.all().iter().map(ResolvedField::key).collect();
        let event = DomainEvent::new(actor.clone(), FIELDS_REORDERED, now)
            .with_payload(json!({ "record_type": definition.record_type.as_str(), "keys": keys }));
        let entry = AuditEntry::new(actor, FIELDS_REORDERED, now)
            .with_changes(vec![FieldChange::new(
                "order",
                json!(old_keys),
                json!(keys),
            )])
            .with_ip(command.ip);

        let mut tx = self.ports.transactions().begin().await?;
        self.ports
            .definitions()
            .reorder(&mut tx, &definition.record_type, &ordered)
            .await?;
        self.ports.events().publish(&mut tx, event).await?;
        self.ports.audit().record(&mut tx, entry).await?;
        self.ports.transactions().commit(tx).await?;

        let (_, fields) = self.load_fields(record_type).await?;
        Ok(fields.all().iter().map(field_view).collect())
    }

    pub async fn add_option(
        &self,
        scope: &AccessScope,
        record_type: &str,
        field_id: &str,
        input: FieldOptionInput,
        ip: Option<String>,
    ) -> Result<FieldView, AppError> {
        scope.require(&Permission::SETTINGS_MANAGE)?;
        let (definition, fields) = self.load_fields(record_type).await?;
        let field = self.option_field(&fields, field_id)?;
        if field.definition.options.len() >= MAX_OPTIONS {
            return Err(AppError::invalid_fields(
                "invalid option",
                vec![FieldViolation::new(
                    "value",
                    format!("a field can have at most {MAX_OPTIONS} options"),
                )],
            ));
        }
        let mut violations = Vec::new();
        let taken: Vec<String> = field
            .definition
            .options
            .iter()
            .map(|option| option.value.clone())
            .collect();
        let option = build_option(
            &input,
            &taken,
            next_position(&field.definition),
            "",
            &mut violations,
        );
        let Some(option) = option.filter(|_| violations.is_empty()) else {
            return Err(AppError::invalid_fields("invalid option", violations));
        };
        if taken.contains(&option.value) {
            return Err(AppError::conflict_code(
                OPTION_VALUE_TAKEN,
                format!("the option {} already exists", option.value),
            ));
        }
        let changes = diff_changes(
            &Value::Null,
            &OptionSnapshot::from(&option),
            &DiffOptions::default(),
        )?;
        let mut tx = self.ports.transactions().begin().await?;
        self.ports
            .definitions()
            .insert_option(&mut tx, field.definition.id, &option)
            .await?;
        self.record(
            &mut tx,
            scope,
            FIELD_OPTION_CREATED,
            &field.definition,
            json!({ "option_id": option.id.to_string(), "value": option.value }),
            prefixed(changes, &option.value),
            ip,
        )
        .await?;
        self.ports.transactions().commit(tx).await?;
        self.require_view(definition, field.definition.id).await
    }

    pub async fn update_option(
        &self,
        scope: &AccessScope,
        record_type: &str,
        field_id: &str,
        option_id: &str,
        command: UpdateOptionCommand,
    ) -> Result<FieldView, AppError> {
        scope.require(&Permission::SETTINGS_MANAGE)?;
        let (definition, fields) = self.load_fields(record_type).await?;
        let field = self.option_field(&fields, field_id)?;
        if let Some(updated_at) = field.updated_at {
            ensure_version(command.expected_updated_at, updated_at)?;
        }
        let current = Uuid::parse_str(option_id)
            .ok()
            .and_then(|id| field.definition.option_by_id(id))
            .ok_or_else(option_not_found)?;
        let mut updated = current.clone();
        let mut violations = Vec::new();
        if let Some(label) = &command.label {
            match bounded_label(label) {
                Some(label) => updated.label = label,
                None => violations.push(label_violation("label")),
            }
        }
        if let Some(color) = &command.color {
            match color.as_deref().map(normalize_color) {
                None => updated.color = None,
                Some(Some(color)) => updated.color = Some(color),
                Some(None) => violations.push(color_violation("color")),
            }
        }
        if let Some(is_active) = command.is_active {
            updated.is_active = is_active;
        }
        if !violations.is_empty() {
            return Err(AppError::invalid_fields("invalid option", violations));
        }
        let changes = diff_changes(
            &OptionSnapshot::from(current),
            &OptionSnapshot::from(&updated),
            &DiffOptions::default(),
        )?;
        if changes.is_empty() {
            return Ok(field_view(field));
        }
        let mut tx = self.ports.transactions().begin().await?;
        self.ports
            .definitions()
            .update_option(&mut tx, field.definition.id, &updated)
            .await?;
        self.record(
            &mut tx,
            scope,
            FIELD_OPTION_UPDATED,
            &field.definition,
            json!({ "option_id": updated.id.to_string(), "value": updated.value }),
            prefixed(changes, &updated.value),
            command.ip,
        )
        .await?;
        self.ports.transactions().commit(tx).await?;
        self.require_view(definition, field.definition.id).await
    }

    pub async fn reorder_options(
        &self,
        scope: &AccessScope,
        record_type: &str,
        field_id: &str,
        command: ReorderCommand,
    ) -> Result<FieldView, AppError> {
        scope.require(&Permission::SETTINGS_MANAGE)?;
        let (definition, fields) = self.load_fields(record_type).await?;
        let field = self.option_field(&fields, field_id)?;
        let mut options = field.definition.options.clone();
        options.sort_by_key(|option| option.position);
        let current: Vec<Uuid> = options.iter().map(|option| option.id).collect();
        let ordered = permutation(&command.ids, &current, "ids")?;
        if ordered == current {
            return Ok(field_view(field));
        }
        let values = |ids: &[Uuid]| -> Vec<String> {
            ids.iter()
                .filter_map(|id| {
                    field
                        .definition
                        .option_by_id(*id)
                        .map(|option| option.value.clone())
                })
                .collect()
        };
        let mut tx = self.ports.transactions().begin().await?;
        self.ports
            .definitions()
            .reorder_options(&mut tx, field.definition.id, &ordered)
            .await?;
        self.record(
            &mut tx,
            scope,
            FIELD_OPTIONS_REORDERED,
            &field.definition,
            json!({ "values": values(&ordered) }),
            vec![FieldChange::new(
                "options",
                json!(values(&current)),
                json!(values(&ordered)),
            )],
            command.ip,
        )
        .await?;
        self.ports.transactions().commit(tx).await?;
        self.require_view(definition, field.definition.id).await
    }

    async fn readable_fields(
        &self,
        scope: &AccessScope,
        record_type: &str,
    ) -> Result<(&EntityDefinition, FieldSet), AppError> {
        let definition = self.ports.registry().require(record_type)?;
        if scope.require(&Permission::SETTINGS_MANAGE).is_err()
            && scope.require(&definition.read_permission).is_err()
        {
            return Err(AppError::forbidden_code(
                PERMISSION_DENIED,
                format!(
                    "missing one of the permissions {}, {}",
                    Permission::SETTINGS_MANAGE,
                    definition.read_permission
                ),
            ));
        }
        let (definition, fields) = self.load_fields(record_type).await?;
        let policy =
            FieldAccessPolicy::load(self.ports.definitions(), scope, &definition.record_type)
                .await?;
        Ok((definition, fields.with_access(&policy)))
    }

    async fn field_set(&self, definition: &EntityDefinition) -> Result<FieldSet, AppError> {
        let definitions = self.ports.definitions();
        let versions = definitions.versions(&definition.record_type).await?;
        Ok(
            FieldSet::new(definition, definitions.list(&definition.record_type).await?)
                .with_versions(&versions),
        )
    }

    async fn load_fields(
        &self,
        record_type: &str,
    ) -> Result<(&EntityDefinition, FieldSet), AppError> {
        let definition = self.ports.registry().require(record_type)?;
        let fields = self.field_set(definition).await?;
        Ok((definition, fields))
    }

    async fn require_view(
        &self,
        definition: &EntityDefinition,
        field_id: Uuid,
    ) -> Result<FieldView, AppError> {
        let fields = self.field_set(definition).await?;
        fields
            .find_by_id(field_id)
            .map(field_view)
            .ok_or_else(field_not_found)
    }

    fn option_field<'f>(
        &self,
        fields: &'f FieldSet,
        field_id: &str,
    ) -> Result<&'f ResolvedField, AppError> {
        let field = require_field(fields, field_id)?;
        ensure_unmanaged(&field.definition)?;
        if field.is_system() {
            return Err(system_field_locked());
        }
        if !field.definition.field_type.has_options() {
            return Err(AppError::validation_code(
                "field_without_options",
                "only select and multi_select fields have options",
            ));
        }
        Ok(field)
    }

    fn field_violations(&self, field: &FieldDefinition) -> Vec<FieldViolation> {
        field_definition_violations(self.ports.registry(), field)
    }

    #[allow(clippy::too_many_arguments)]
    async fn record(
        &self,
        tx: &mut FieldsTx<P>,
        scope: &AccessScope,
        action: &str,
        field: &FieldDefinition,
        extra: Value,
        changes: Vec<FieldChange>,
        ip: Option<String>,
    ) -> Result<(), AppError> {
        let actor = Actor::from_scope(scope);
        let now = self.ports.clock().now();
        let record = RecordRef::new(FIELD_RECORD_TYPE, field.id);
        let mut payload = json!({
            "record_type": field.record_type,
            "key": field.key.as_str(),
            "field_type": field.type_name(),
        });
        if let (Some(payload), Value::Object(extra)) = (payload.as_object_mut(), extra) {
            payload.extend(extra);
        }
        let event = DomainEvent::new(actor.clone(), action, now)
            .with_record(record.clone())
            .with_payload(payload);
        let entry = AuditEntry::new(actor, action, now)
            .with_record(record)
            .with_changes(changes)
            .with_ip(ip);
        self.ports.events().publish(tx, event).await?;
        self.ports.audit().record(tx, entry).await
    }
}

fn field_view(field: &ResolvedField) -> FieldView {
    let mut definition = field.definition.clone();
    definition.options.sort_by_key(|option| option.position);
    FieldView {
        labels: field.labels.clone(),
        access: field.access,
        updated_at: field.updated_at,
        filterable: field.filterable(),
        sortable: field.sortable(),
        operators: if field.filterable() {
            operators_for_definition(&definition)
        } else {
            Vec::new()
        },
        definition,
    }
}

fn require_field<'f>(fields: &'f FieldSet, field_id: &str) -> Result<&'f ResolvedField, AppError> {
    Uuid::parse_str(field_id)
        .ok()
        .and_then(|id| fields.find_by_id(id))
        .ok_or_else(field_not_found)
}

fn field_not_found() -> AppError {
    AppError::not_found_code(FIELD_NOT_FOUND, "field was not found")
}

fn option_not_found() -> AppError {
    AppError::not_found_code(FIELD_OPTION_NOT_FOUND, "option was not found")
}

fn managed_field_locked(field: &FieldDefinition) -> AppError {
    AppError::conflict_code(
        FIELD_MANAGED_BY_MODULE,
        format!(
            "the field {} is managed by the module {} and cannot be changed",
            field.key.as_str(),
            field.module_key.as_deref().unwrap_or_default()
        ),
    )
}

fn ensure_unmanaged(field: &FieldDefinition) -> Result<(), AppError> {
    if field.is_managed() {
        return Err(managed_field_locked(field));
    }
    Ok(())
}

fn system_field_locked() -> AppError {
    AppError::validation_code(
        SYSTEM_FIELD_LOCKED,
        "only the label and the position of system fields can be changed",
    )
}

pub fn field_definition_violations(
    registry: &RecordTypeRegistry,
    field: &FieldDefinition,
) -> Vec<FieldViolation> {
    let mut violations: Vec<FieldViolation> = field
        .config
        .validate(field.field_type)
        .into_iter()
        .map(|violation| {
            FieldViolation::new(
                format!("config.{}", violation.property.as_str()),
                violation.message,
            )
        })
        .collect();
    if let Some(target) = &field.config.target_record_type
        && registry.require(target).is_err()
    {
        violations.push(FieldViolation::new(
            "config.target_record_type",
            "is not a registered record type",
        ));
    }
    if field.is_formula() {
        for (property, enabled) in [
            ("is_required", field.is_required),
            ("is_unique", field.is_unique),
        ] {
            if enabled {
                violations.push(FieldViolation::new(
                    property,
                    "is not supported by formula fields",
                ));
            }
        }
    }
    if field.is_unique && !field.field_type.supports_unique() {
        violations.push(FieldViolation::new(
            "is_unique",
            format!("is not supported by {} fields", field.field_type.as_str()),
        ));
    }
    if field.is_searchable && !field.field_type.is_textual() {
        violations.push(FieldViolation::new(
            "is_searchable",
            format!("is not supported by {} fields", field.field_type.as_str()),
        ));
    }
    if let Some(default) = &field.config.default_value
        && violations.is_empty()
        && let Err(error) = validate_value(field, default)
    {
        violations.push(FieldViolation::new("config.default_value", error.message()));
    }
    violations
}

fn declared_field_type(
    declared: &str,
    config: Option<&FieldConfig>,
    violations: &mut Vec<FieldViolation>,
) -> Option<FieldType> {
    let formula = config.and_then(|config| config.formula.as_ref());
    if declared == FORMULA_FIELD_TYPE {
        if config.is_some() && formula.is_none() {
            violations.push(FieldViolation::new(
                "config.expression",
                "is required for formula fields",
            ));
        }
        return resolve_declared_type(declared, formula);
    }
    let field_type = FieldType::parse(declared);
    match field_type {
        None => violations.push(FieldViolation::new(
            "field_type",
            "is not a known field type",
        )),
        Some(_) if formula.is_some() => violations.push(FieldViolation::new(
            "config.expression",
            "is only accepted by formula fields",
        )),
        Some(FieldType::References) => violations.push(FieldViolation::new(
            "field_type",
            "is only available to fields declared by modules and plugins",
        )),
        Some(_) => {}
    }
    field_type
}

pub fn bounded_label(value: &str) -> Option<String> {
    let label = value.trim();
    (!label.is_empty() && label.chars().count() <= MAX_LABEL_LENGTH).then(|| label.to_owned())
}

fn label_violation(path: &str) -> FieldViolation {
    FieldViolation::new(
        path,
        format!("must have between 1 and {MAX_LABEL_LENGTH} characters"),
    )
}

fn color_violation(path: &str) -> FieldViolation {
    FieldViolation::new(path, "must be a hex color such as #1f6feb")
}

pub fn normalize_color(value: &str) -> Option<String> {
    let color = value.trim().to_ascii_lowercase();
    (color.len() == 7
        && color.starts_with('#')
        && color[1..]
            .chars()
            .all(|character| character.is_ascii_hexdigit()))
    .then_some(color)
}

fn next_position(field: &FieldDefinition) -> i32 {
    field
        .options
        .iter()
        .map(|option| option.position)
        .max()
        .map_or(0, |position| position + 1)
}

fn prefixed(changes: Vec<FieldChange>, value: &str) -> Vec<FieldChange> {
    changes
        .into_iter()
        .map(|change| {
            FieldChange::new(
                format!("options.{value}.{}", change.field),
                change.old,
                change.new,
            )
        })
        .collect()
}

pub fn option_value_from_label(label: &str) -> String {
    let mut value = String::new();
    for character in label.trim().to_lowercase().chars() {
        let mapped = match character {
            'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            'ñ' => 'n',
            other if other.is_ascii_alphanumeric() => other,
            _ => '_',
        };
        if mapped == '_' && (value.is_empty() || value.ends_with('_')) {
            continue;
        }
        value.push(mapped);
    }
    let mut value = value.trim_end_matches('_').to_owned();
    if value.starts_with(|character: char| character.is_ascii_digit()) {
        value.insert_str(0, "option_");
    }
    value.truncate(64);
    value.trim_end_matches('_').to_owned()
}

fn build_option(
    input: &FieldOptionInput,
    taken: &[String],
    position: i32,
    path: &str,
    violations: &mut Vec<FieldViolation>,
) -> Option<FieldOption> {
    let before = violations.len();
    let label = bounded_label(&input.label);
    if label.is_none() {
        violations.push(label_violation(&format!("{path}label")));
    }
    let value = match input
        .value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some(value) => value.to_owned(),
        None => {
            let base = option_value_from_label(&input.label);
            let base = if base.is_empty() {
                "option".to_owned()
            } else {
                base
            };
            let mut candidate = base.clone();
            let mut suffix = 2;
            while taken.contains(&candidate) {
                candidate = format!("{base}_{suffix}");
                suffix += 1;
            }
            candidate
        }
    };
    if !is_option_value(&value) {
        violations.push(FieldViolation::new(
            format!("{path}value"),
            "must be snake_case: lowercase letters, digits and single underscores, starting with a letter (max 64)",
        ));
    }
    let color = match input.color.as_deref() {
        None => None,
        Some(color) => match normalize_color(color) {
            Some(color) => Some(color),
            None => {
                violations.push(color_violation(&format!("{path}color")));
                None
            }
        },
    };
    (violations.len() == before).then(|| FieldOption {
        id: Uuid::now_v7(),
        value,
        label: label.unwrap_or_default(),
        color,
        position,
        is_active: true,
    })
}

fn build_options(
    field_type: FieldType,
    inputs: &[FieldOptionInput],
    violations: &mut Vec<FieldViolation>,
) -> Vec<FieldOption> {
    if !field_type.has_options() {
        if !inputs.is_empty() {
            violations.push(FieldViolation::new(
                "options",
                "are only accepted by select and multi_select fields",
            ));
        }
        return Vec::new();
    }
    if inputs.len() > MAX_OPTIONS {
        violations.push(FieldViolation::new(
            "options",
            format!("a field can have at most {MAX_OPTIONS} options"),
        ));
        return Vec::new();
    }
    let mut options: Vec<FieldOption> = Vec::new();
    for (index, input) in inputs.iter().enumerate() {
        let taken: Vec<String> = options.iter().map(|option| option.value.clone()).collect();
        let path = format!("options.{index}.");
        if let Some(option) = build_option(input, &taken, index as i32, &path, violations) {
            if taken.contains(&option.value) {
                violations.push(FieldViolation::new(format!("{path}value"), "is duplicated"));
                continue;
            }
            options.push(option);
        }
    }
    options
}

fn permutation(ids: &[String], current: &[Uuid], path: &str) -> Result<Vec<Uuid>, AppError> {
    let parsed: Option<Vec<Uuid>> = ids.iter().map(|id| Uuid::parse_str(id).ok()).collect();
    let valid = parsed.as_ref().is_some_and(|parsed| {
        let unique: BTreeSet<&Uuid> = parsed.iter().collect();
        parsed.len() == current.len()
            && unique.len() == parsed.len()
            && parsed.iter().all(|id| current.contains(id))
    });
    match parsed {
        Some(parsed) if valid => Ok(parsed),
        _ => Err(AppError::invalid_fields(
            "invalid order",
            vec![FieldViolation::new(path, "must list every id exactly once")],
        )),
    }
}
