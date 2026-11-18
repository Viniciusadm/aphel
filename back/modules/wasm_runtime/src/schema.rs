use serde_json::json;
use uuid::Uuid;

use crate::manifest::{FieldDef, PluginManifest};
use crate::store::schema as store;
use aphel_kernel::errors::{AppError, CodedKind, FieldViolation};
use aphel_kernel::state::AppState;
use aphel_sdk::objects::{
    CreateObjectCommand, ManagedObject, ManagedObjectWriter, managed_object_writer,
};
use aphel_sdk::platform::application::custom_objects::custom_record_definition;
use aphel_sdk::platform::application::fields::{FieldDefinitionRepository, RecordTypeRegistry};
use aphel_sdk::platform::database::Db;
use aphel_sdk::platform::domain::access::AccessScope;
use aphel_sdk::platform::domain::custom_objects::{
    CustomObject, CustomObjectData, DEFAULT_TITLE_FIELD_KEY,
};
use aphel_sdk::platform::domain::fields::{FieldDefinition, FieldKey, FieldOption, FieldType};
use aphel_sdk::platform::infrastructure::fields::mysql_definitions::MySqlFieldDefinitionRepository;
use aphel_sdk::platform::infrastructure::fields::unique_values;

const PLUGIN_FIELD_POSITION: i32 = 20_000;
const FIELD_TAKEN: &str = "plugin_field_conflict";
const OBJECT_TAKEN: &str = "plugin_object_conflict";
const FIELD_TYPE_CHANGED: &str = "plugin_field_type_changed";
const FIELD_UNIQUE_CONFLICT: &str = "plugin_field_unique_conflict";
const DUPLICATE_SAMPLE: u32 = 20;

async fn bump_schema(state: &AppState) -> Result<(), AppError> {
    store::bump_schema_version(&state.db()?).await?;
    state.record_types.invalidate();
    Ok(())
}

async fn object_owner(pool: &Db, key: &str) -> Result<Option<Option<String>>, AppError> {
    store::object_owner(pool, key).await
}

pub async fn validate_dependencies(
    state: &AppState,
    manifest: &PluginManifest,
) -> Result<(), AppError> {
    validate_dependencies_with(state, manifest, &[], &[]).await
}

fn pack_entity(pack: &[PluginManifest], target: &str) -> bool {
    pack.iter().any(|other| {
        other
            .entity_record_types()
            .iter()
            .any(|record_type| record_type == target)
    })
}

pub async fn validate_dependencies_with(
    state: &AppState,
    manifest: &PluginManifest,
    pack: &[PluginManifest],
    pending_modules: &[String],
) -> Result<(), AppError> {
    let registry = state.registry().await?;
    let mut provided: Vec<String> = state
        .app
        .modules()
        .iter()
        .filter(|loaded| state.app.is_enabled(loaded.key()))
        .flat_map(|loaded| loaded.manifest.provides.contracts.clone())
        .collect();
    provided.extend(
        state
            .app
            .all_contributions::<aphel_sdk::ContractDescriptor>()
            .into_iter()
            .filter(|(provider, _)| state.app.is_enabled(provider))
            .map(|(_, descriptor)| descriptor.contract.requirement()),
    );
    for other in &crate::registry::current(&state.db()?).await?.plugins {
        if other.key != manifest.key {
            provided.extend(other.manifest.provides.contracts.clone());
        }
    }
    for other in pack {
        if other.key != manifest.key {
            provided.extend(other.provides.contracts.clone());
        }
    }
    provided.extend(crate::provision::module_contracts(state, pending_modules));
    let unmet = manifest.uses.unmet(&provided);
    if !unmet.is_empty() {
        return Err(AppError::validation_code(
            "plugin_contract_unavailable",
            format!(
                "no enabled provider for the required contracts: {}",
                unmet.join(", ")
            ),
        ));
    }
    for required in &manifest.requires {
        let permission = aphel_sdk::platform::domain::access::Permission::parse(required.clone())?;
        if let Some(owner) = state.app.catalog().permission_owner(&permission)
            && !state.app.is_enabled(owner)
            && !pending_modules.iter().any(|module| module == owner)
        {
            return Err(AppError::validation_code(
                "plugin_dependency_unavailable",
                format!("permission {required} requires enabled module {owner}"),
            ));
        }
    }
    for entity in &manifest.entities {
        for reference in &entity.references {
            let aphel_sdk::platform::application::fields::ReferenceTarget::Entity {
                entity: target,
            } = &reference.target
            else {
                continue;
            };
            if manifest.entity_record_types().contains(target) {
                continue;
            }
            if pack_entity(pack, target) {
                let permission = aphel_sdk::platform::domain::access::Permission::parse(format!(
                    "{target}.read"
                ))?;
                if !manifest.allows(&permission) {
                    return Err(AppError::validation_code(
                        "plugin_reference_permission_required",
                        format!("reference {} requires {permission}", reference.field),
                    ));
                }
                continue;
            }
            let target = registry.require(target)?;
            if !manifest.allows(&target.read_permission) {
                return Err(AppError::validation_code(
                    "plugin_reference_permission_required",
                    format!(
                        "reference {} requires {}",
                        reference.field, target.read_permission
                    ),
                ));
            }
        }
    }
    for (_, field) in declared_fields(manifest) {
        if field.field_type != "record" {
            continue;
        }
        let Some(target) = &field.target else {
            continue;
        };
        if manifest
            .entities
            .iter()
            .any(|entity| manifest.entity_record_type(&entity.key) == *target)
        {
            continue;
        }
        if pack_entity(pack, target) {
            let permission =
                aphel_sdk::platform::domain::access::Permission::parse(format!("{target}.read"))?;
            if !manifest.allows(&permission) {
                return Err(AppError::validation_code(
                    "plugin_reference_permission_required",
                    format!(
                        "field {} references {target}; declare {permission} in requires",
                        field.key
                    ),
                ));
            }
            continue;
        }
        let definition = registry.require(target)?;
        if !manifest.allows(&definition.read_permission) {
            return Err(AppError::validation_code(
                "plugin_reference_permission_required",
                format!(
                    "field {} references {target}; declare {} in requires",
                    field.key, definition.read_permission
                ),
            ));
        }
    }
    Ok(())
}

pub async fn check(state: &AppState, manifest: &PluginManifest) -> Result<(), AppError> {
    check_with(state, manifest, &[], &[]).await
}

pub fn object_administration_module(state: &AppState) -> Option<&'static str> {
    let wanted = std::any::type_name::<dyn ManagedObjectWriter>();
    state
        .app
        .services()
        .describe()
        .into_iter()
        .find(|service| service.type_name == wanted)
        .map(|service| service.module)
}

fn ensure_object_administration(
    state: &AppState,
    pending_modules: &[String],
) -> Result<(), AppError> {
    let Some(module) = object_administration_module(state) else {
        return Err(AppError::validation_code(
            "plugin_dependency_unavailable",
            "the plugin declares entities, but this Aphel build has no module that administers objects",
        ));
    };
    if state.app.is_enabled(module) || pending_modules.iter().any(|key| key == module) {
        return Ok(());
    }
    Err(AppError::validation_code(
        "plugin_dependency_unavailable",
        format!(
            "the plugin declares entities, which require enabled module {module}; enable it before installing"
        ),
    ))
}

pub async fn check_with(
    state: &AppState,
    manifest: &PluginManifest,
    pack: &[PluginManifest],
    pending_modules: &[String],
) -> Result<(), AppError> {
    for entity in &manifest.entities {
        if entity.storage == aphel_sdk::platform::application::fields::EntityStorage::Dynamic
            && !entity.indexes.is_empty()
        {
            return Err(AppError::Validation(
                "entity indexes require managed storage".to_owned(),
            ));
        }
        if entity.storage == aphel_sdk::platform::application::fields::EntityStorage::Table {
            return Err(AppError::Validation(
                "runtime entities cannot use native table storage".to_owned(),
            ));
        }
        let key = format!("{}.{}", manifest.key, entity.key);
        let permission =
            aphel_sdk::platform::domain::access::Permission::parse(format!("{key}.read"))?;
        let definition = entity.definition(&key, permission)?;
        if entity.storage == aphel_sdk::platform::application::fields::EntityStorage::Managed {
            aphel_sdk::platform::infrastructure::fields::managed_schema(&definition)?;
        }
    }
    if !manifest.entities.is_empty() {
        ensure_object_administration(state, pending_modules)?;
    }
    validate_dependencies_with(state, manifest, pack, pending_modules).await?;
    let pool = state.db()?;
    ensure_compatible(&pool, manifest).await?;
    preflight(&pool, manifest).await?;
    ensure_unique_values(state, manifest).await
}

async fn ensure_unique_values(state: &AppState, manifest: &PluginManifest) -> Result<(), AppError> {
    let pool = state.db()?;
    let registry = state.registry().await?;
    let mut conflicts = Vec::new();
    for (record_type, field) in declared_fields(manifest) {
        if !field.unique {
            continue;
        }
        let Ok(definition) = registry.require(&record_type) else {
            continue;
        };
        let Some(existing) = MySqlFieldDefinitionRepository::new(pool.clone())
            .list(&aphel_sdk::platform::application::shared::RecordType::parse(&record_type)?)
            .await?
            .into_iter()
            .find(|candidate| candidate.key.as_str() == field.key)
        else {
            continue;
        };
        if existing.is_unique {
            continue;
        }
        let records =
            unique_values::duplicated_records(&pool, definition, &existing, DUPLICATE_SAMPLE)
                .await?;
        if !records.is_empty() {
            conflicts.push(json!({
                "record_type": record_type,
                "field": field.key,
                "record_ids": records.iter().map(ToString::to_string).collect::<Vec<_>>(),
            }));
        }
    }
    if conflicts.is_empty() {
        return Ok(());
    }
    Err(AppError::with_details(
        CodedKind::Conflict,
        FIELD_UNIQUE_CONFLICT,
        "existing records share values of fields the plugin declares as unique; resolve the duplicates before installing",
        json!({ "conflicts": conflicts }),
    ))
}

pub async fn apply(
    state: &AppState,
    scope: &AccessScope,
    manifest: &PluginManifest,
) -> Result<(), AppError> {
    check(state, manifest).await?;
    let pool = state.db()?;
    let ctx = aphel_kernel::Ctx::from_parts(
        scope,
        pool.clone(),
        state.registry().await?,
        state.events.clone(),
        state.audit.clone(),
    );
    if !manifest.entities.is_empty() {
        managed_object_writer(&ctx)?;
    }
    for entity in &manifest.entities {
        let object_key = manifest.object_key(&entity.key);
        let storage = store::object_entity_definition(&pool, &object_key).await?;
        if let Some(stored) = storage {
            let previous = stored
                .and_then(|json| {
                    serde_json::from_value::<
                        aphel_sdk::platform::application::fields::EntityDeclaration,
                    >(json)
                    .ok()
                })
                .map(|declaration| declaration.storage)
                .unwrap_or(aphel_sdk::platform::application::fields::EntityStorage::Dynamic);
            if previous != entity.storage {
                return Err(AppError::Configuration(
                    "changing entity storage needs a data migration".to_owned(),
                ));
            }
        }
        if entity.storage == aphel_sdk::platform::application::fields::EntityStorage::Managed {
            let key = format!("{}.{}", manifest.key, entity.key);
            let permission = aphel_sdk::platform::domain::access::Permission::parse(format!(
                "{}.read",
                manifest.entity_record_type(&entity.key)
            ))?;
            let definition = entity.definition(&key, permission)?;
            aphel_sdk::platform::infrastructure::fields::ensure_managed_schema(&pool, &definition)
                .await?;
        }
        match object_owner(&pool, &object_key).await? {
            Some(Some(owner)) if owner == manifest.key => {
                store::update_object_labels(
                    &pool,
                    &object_key,
                    store::ObjectLabels {
                        label_singular: &entity.label,
                        label_plural: &entity.label_plural,
                        description: &entity.description,
                        owner_enabled: entity.owner_enabled,
                        internal: entity.internal,
                    },
                )
                .await?;
            }
            Some(_) => {
                return Err(AppError::conflict_code(
                    OBJECT_TAKEN,
                    format!("a custom object with the key {object_key} already exists"),
                ));
            }
            None => {
                managed_object_writer(&ctx)?
                    .create(
                        &ctx,
                        CreateObjectCommand {
                            key: object_key.clone(),
                            label_singular: entity.label.clone(),
                            label_plural: entity.label_plural.clone(),
                            description: entity.description.clone(),
                            icon: entity.icon.clone(),
                            color: None,
                            owner_enabled: Some(entity.owner_enabled),
                            ip: None,
                        },
                        ManagedObject {
                            entity_definition: Some(
                                serde_json::to_value(entity)
                                    .map_err(|error| AppError::Configuration(error.to_string()))?,
                            ),
                            module_key: manifest.key.clone(),
                            permission_scope: manifest.entity_scope(entity),
                            internal: entity.internal,
                        },
                    )
                    .await?;
            }
        }
    }
    for entity in &manifest.entities {
        store::set_entity_definition(
            &pool,
            &manifest.object_key(&entity.key),
            &manifest.key,
            entity,
        )
        .await?;
    }
    bump_schema(state).await?;
    let registry = state.registry().await?;
    let fields = declared_fields(manifest);
    let repository = MySqlFieldDefinitionRepository::new(pool.clone());
    let mut tx = aphel_sdk::platform::database::transaction::Tx::from(pool.begin().await?);
    for (index, (record_type, field)) in fields.into_iter().enumerate() {
        let mut definition = definition(&registry, &manifest.key, &record_type, &field, index)?;
        let existing = store::field_id_and_owner(&mut tx, &record_type, &field.key).await?;
        match existing {
            None => repository.insert(&mut tx, &definition).await?,
            Some((id, owner))
                if owner.as_deref() == Some(manifest.key.as_str())
                    || (owner.is_none()
                        && definition.is_system
                        && registry.require(&record_type)?.storage
                            == aphel_sdk::platform::application::fields::EntityStorage::Managed) =>
            {
                definition.id = Uuid::parse_str(&id)
                    .map_err(|_| AppError::Internal(format!("invalid field id {id}")))?;
                store::update_plugin_field(&mut tx, &id, &manifest.key, &definition).await?;
                for (position, option) in definition.options.iter().enumerate() {
                    store::upsert_field_option(&mut tx, &id, option, position as i32).await?;
                }
                store::deactivate_undeclared_options(&mut tx, &id, &definition.options).await?;
                if definition.field_type.supports_unique() {
                    unique_values::rebuild(&mut tx, registry.require(&record_type)?, &definition)
                        .await
                        .map_err(|error| match error {
                            AppError::Coded {
                                kind: CodedKind::Conflict,
                                ..
                            } => AppError::conflict_code(
                                FIELD_UNIQUE_CONFLICT,
                                format!(
                                    "existing records share values of {} on {record_type}",
                                    field.key
                                ),
                            ),
                            other => other,
                        })?;
                }
            }
            Some(_) => {
                return Err(AppError::conflict_code(
                    FIELD_TAKEN,
                    format!("the field {} already exists on {record_type}", field.key),
                ));
            }
        }
    }
    tx.commit().await?;
    deactivate_undeclared(&pool, manifest).await?;
    bump_schema(state).await
}

fn reserved_keys(manifest: &PluginManifest, entity_key: &str) -> Result<Vec<String>, AppError> {
    let Some(entity) = manifest
        .entities
        .iter()
        .find(|entity| entity.key == entity_key)
    else {
        return Ok(Vec::new());
    };
    if entity.storage == aphel_sdk::platform::application::fields::EntityStorage::Managed {
        return Ok([
            "id",
            "created_at",
            "updated_at",
            "created_by",
            "owner_id",
            "deleted_at",
            "archived_at",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect());
    }
    let now = chrono::Utc::now();
    let object = CustomObject {
        id: Uuid::nil(),
        key: manifest.object_key(&entity.key),
        data: CustomObjectData {
            label_singular: entity.label.clone(),
            label_plural: entity.label_plural.clone(),
            description: None,
            icon: None,
            color: None,
            title_field_key: DEFAULT_TITLE_FIELD_KEY.to_owned(),
            owner_enabled: entity.owner_enabled,
            is_active: true,
        },
        position: 0,
        module_key: Some(manifest.key.clone()),
        entity_definition: None,
        is_internal: entity.internal,
        created_by: None,
        created_at: now,
        updated_at: now,
    };
    let mut keys: Vec<String> = custom_record_definition(&object)?
        .system_fields
        .iter()
        .map(|field| field.key.as_str().to_owned())
        .collect();
    keys.push(DEFAULT_TITLE_FIELD_KEY.to_owned());
    Ok(keys)
}

async fn preflight(pool: &Db, manifest: &PluginManifest) -> Result<(), AppError> {
    let mut new_entities = Vec::new();
    for entity in &manifest.entities {
        let object_key = manifest.object_key(&entity.key);
        match object_owner(pool, &object_key).await? {
            Some(Some(owner)) if owner == manifest.key => {}
            Some(_) => {
                return Err(AppError::conflict_code(
                    OBJECT_TAKEN,
                    format!("a custom object with the key {object_key} already exists"),
                ));
            }
            None => new_entities.push((manifest.entity_record_type(&entity.key), &entity.key)),
        }
    }
    for (record_type, field) in declared_fields(manifest) {
        let taken = match new_entities
            .iter()
            .find(|(entity_type, _)| entity_type == &record_type)
        {
            Some((_, entity_key)) => reserved_keys(manifest, entity_key)?.contains(&field.key),
            None => {
                let owner = store::field_claim(pool, &record_type, &field.key).await?;
                let owned_managed = manifest.entities.iter().any(|entity| {
                    entity.storage
                        == aphel_sdk::platform::application::fields::EntityStorage::Managed
                        && manifest.entity_record_type(&entity.key) == record_type
                });
                owner.is_some_and(|(owner, system, column)| {
                    owner.as_deref() != Some(manifest.key.as_str())
                        && !(owned_managed
                            && owner.is_none()
                            && system
                            && column.as_deref() == Some(field.key.as_str()))
                })
            }
        };
        if taken {
            return Err(AppError::conflict_code(
                FIELD_TAKEN,
                format!("the field {} already exists on {record_type}", field.key),
            ));
        }
    }
    Ok(())
}

pub(crate) fn declared_fields(manifest: &PluginManifest) -> Vec<(String, FieldDef)> {
    let mut fields: Vec<(String, FieldDef)> = Vec::new();
    for entity in &manifest.entities {
        let record_type = manifest.entity_record_type(&entity.key);
        fields.extend(
            entity
                .expanded_fields()
                .unwrap_or_else(|_| entity.fields.clone())
                .into_iter()
                .map(|field| (record_type.clone(), field)),
        );
    }
    fields.extend(
        manifest
            .fields
            .iter()
            .map(|extension| (extension.record_type.clone(), extension.field.clone())),
    );
    fields
}

async fn ensure_compatible(pool: &Db, manifest: &PluginManifest) -> Result<(), AppError> {
    let existing = store::module_field_types(pool, &manifest.key).await?;
    for (record_type, field) in declared_fields(manifest) {
        let Some(new_type) = FieldType::parse(&field.field_type) else {
            continue;
        };
        if let Some((_, _, old_type)) = existing
            .iter()
            .find(|(existing_type, key, _)| existing_type == &record_type && key == &field.key)
            && old_type != new_type.as_str()
        {
            return Err(AppError::conflict_code(
                FIELD_TYPE_CHANGED,
                format!(
                    "the field {} on {record_type} is {old_type} and the new version declares it as {}; the type of a field cannot change, declare a field with a new key",
                    field.key,
                    new_type.as_str()
                ),
            ));
        }
    }
    Ok(())
}

async fn deactivate_undeclared(pool: &Db, manifest: &PluginManifest) -> Result<(), AppError> {
    let declared: Vec<(String, String)> = declared_fields(manifest)
        .into_iter()
        .map(|(record_type, field)| (record_type, field.key.clone()))
        .collect();
    let existing = store::active_module_fields(pool, &manifest.key).await?;
    for (id, record_type, key) in existing {
        if !declared.contains(&(record_type, key)) {
            store::deactivate_field(pool, &id).await?;
        }
    }
    let objects: Vec<String> = manifest
        .entities
        .iter()
        .map(|entity| manifest.object_key(&entity.key))
        .collect();
    let owned = store::active_module_objects(pool, &manifest.key).await?;
    for key in owned {
        if !objects.contains(&key) {
            store::deactivate_object(pool, &key).await?;
        }
    }
    Ok(())
}

fn definition(
    registry: &RecordTypeRegistry,
    plugin: &str,
    record_type: &str,
    field: &FieldDef,
    index: usize,
) -> Result<FieldDefinition, AppError> {
    let invalid = |message: String| {
        AppError::invalid_fields(
            "invalid plugin field",
            vec![FieldViolation::new(
                format!("{record_type}.{}", field.key),
                message,
            )],
        )
    };
    registry
        .require(record_type)
        .map_err(|_| invalid(format!("{record_type} is not a record type")))?;
    let field_type = FieldType::parse(&field.field_type)
        .ok_or_else(|| invalid(format!("{} is not a field type", field.field_type)))?;
    let config = field.config().map_err(|errors| {
        invalid(
            errors
                .into_iter()
                .map(|error| format!("{}: {}", error.property, error.message))
                .collect::<Vec<_>>()
                .join("; "),
        )
    })?;
    if let Some(target) = &field.target
        && registry.require(target).is_err()
    {
        return Err(invalid(format!("the target {target} is not a record type")));
    }
    if let Some(violation) = config.validate(field_type).into_iter().next() {
        return Err(invalid(violation.message));
    }
    Ok(FieldDefinition {
        id: Uuid::now_v7(),
        record_type: record_type.to_owned(),
        key: FieldKey::new(&field.key).map_err(|_| invalid("invalid key".to_owned()))?,
        label: field.label.clone(),
        field_type,
        is_system: registry.require(record_type)?.storage
            == aphel_sdk::platform::application::fields::EntityStorage::Managed,
        column_name: (registry.require(record_type)?.storage
            == aphel_sdk::platform::application::fields::EntityStorage::Managed)
            .then(|| field.key.clone()),
        is_required: field.required,
        is_unique: field.unique,
        is_searchable: field.searchable,
        is_personal: field.personal || field.sensitive,
        is_sensitive: field.sensitive,
        module_key: Some(plugin.to_owned()),
        is_active: true,
        position: PLUGIN_FIELD_POSITION + index as i32,
        config,
        options: field
            .options
            .iter()
            .enumerate()
            .map(|(position, option)| FieldOption {
                id: Uuid::now_v7(),
                value: option.value.clone(),
                label: option.label.clone(),
                color: option.color.clone(),
                position: position as i32,
                is_active: true,
            })
            .collect(),
    })
}

pub async fn set_active(
    state: &AppState,
    manifest: &PluginManifest,
    active: bool,
) -> Result<(), AppError> {
    if active {
        validate_dependencies(state, manifest).await?;
    }
    let pool = state.db()?;
    store::set_module_schema_active(&pool, &manifest.key, active).await?;
    if active {
        deactivate_undeclared(&pool, manifest).await?;
    }
    bump_schema(state).await
}

pub async fn purge(state: &AppState, manifest: &PluginManifest) -> Result<(), AppError> {
    let pool = state.db()?;
    let mut tx = aphel_sdk::platform::database::transaction::Tx::from(pool.begin().await?);
    let mut attached: Vec<Uuid> = Vec::new();
    for record_type in manifest.entity_record_types() {
        attached.extend(store::purge_record_type(&mut tx, &record_type).await?);
    }
    store::delete_module_schema(&mut tx, &manifest.key).await?;
    let orphans =
        aphel_sdk::platform::infrastructure::storage::references::delete_unreferenced_files(
            &mut tx, &attached,
        )
        .await?;
    tx.commit().await?;
    for entity in &manifest.entities {
        if entity.storage == aphel_sdk::platform::application::fields::EntityStorage::Managed {
            let table = aphel_sdk::platform::application::fields::managed_table(&format!(
                "{}.{}",
                manifest.key, entity.key
            ));
            store::drop_table(&pool, &table).await?;
        }
    }

    for key in orphans {
        if let Err(error) = state.storage.delete(&key).await {
            tracing::warn!(error = %error, "failed to remove a purged plugin file");
        }
    }
    bump_schema(state).await
}
