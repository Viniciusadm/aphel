use std::sync::Arc;

use aphel_contract_publication::{
    MODULE, PUBLISHABLE, PublishableDeclaration, PublishableDefinition, PublishableField, STATES,
};
use aphel_kernel::errors::{AppError, CodedKind};
use aphel_sdk::platform::application::fields::{
    DRAFT_PREFIX, EntityDefinition, PUBLISHED_AT_FIELD, REVISED_AT_FIELD, RecordTypeRegistry,
    SystemField, publication_state_field,
};

use crate::domain::{DEFINITION_INVALID, coded, not_publishable};

fn declared() -> Vec<PublishableDeclaration> {
    let Some(app) = aphel_sdk::platform::modules::try_app() else {
        return Vec::new();
    };
    let native = app
        .contributions::<PublishableDeclaration>()
        .into_iter()
        .map(|declaration: Arc<PublishableDeclaration>| (*declaration).clone());
    let runtime = app
        .runtime_contributions(PUBLISHABLE)
        .into_iter()
        .filter_map(|contribution| {
            serde_json::from_value::<PublishableDeclaration>(contribution.data.clone())
                .inspect_err(|error| {
                    tracing::warn!(name = contribution.name, %error, "ignoring an invalid publishable contribution");
                })
                .ok()
        });
    native.chain(runtime).collect()
}

pub(crate) fn resolve(
    registry: &RecordTypeRegistry,
    declaration: &PublishableDeclaration,
) -> Result<PublishableDefinition, AppError> {
    let entity = registry
        .require(&declaration.entity)
        .map_err(|_| not_publishable(&declaration.entity))?;
    let state = publication_state_field(MODULE);
    let lifecycle = [state.as_str(), PUBLISHED_AT_FIELD, REVISED_AT_FIELD];
    let fields = entity
        .system_fields
        .iter()
        .filter(|field| {
            field.writers.iter().any(|writer| writer == MODULE)
                && !lifecycle.contains(&field.key.as_str())
        })
        .map(|field| PublishableField {
            draft: format!("{DRAFT_PREFIX}{}", field.key),
            published: field.key.to_string(),
            file: field.field_type.holds_files(),
        })
        .collect();
    let definition = PublishableDefinition {
        entity: declaration.entity.clone(),
        format: declaration.format,
        permission: declaration.permission.clone(),
        fields,
        state,
        published_at: PUBLISHED_AT_FIELD.to_owned(),
        revised_at: REVISED_AT_FIELD.to_owned(),
    };
    validate(registry, &definition)?;
    Ok(definition)
}

pub(crate) fn entities() -> Vec<String> {
    let mut entities: Vec<String> = declared()
        .into_iter()
        .map(|definition| definition.entity)
        .collect();
    entities.sort();
    entities.dedup();
    entities
}

fn invalid(entity: &str, problem: impl std::fmt::Display) -> AppError {
    coded(
        CodedKind::Conflict,
        DEFINITION_INVALID,
        format!("the publication definition of {entity} is invalid: {problem}"),
    )
}

fn field<'d>(entity: &'d EntityDefinition, key: &str) -> Result<&'d SystemField, AppError> {
    entity
        .system_field_by_key(key)
        .filter(|field| field.is_available() && !field.is_computed())
        .ok_or_else(|| invalid(entity.key.as_str(), format!("unknown field {key}")))
}

fn protected<'d>(entity: &'d EntityDefinition, key: &str) -> Result<&'d SystemField, AppError> {
    let found = field(entity, key)?;
    if !found.owner_managed || !found.writers.iter().any(|writer| writer == MODULE) {
        return Err(invalid(
            entity.key.as_str(),
            format!("{key} must be read_only with writers = [\"{MODULE}\"]"),
        ));
    }
    Ok(found)
}

pub(crate) fn validate(
    registry: &RecordTypeRegistry,
    definition: &PublishableDefinition,
) -> Result<(), AppError> {
    let entity = registry
        .require(&definition.entity)
        .map_err(|_| not_publishable(&definition.entity))?;
    if definition.fields.is_empty() {
        return Err(invalid(&definition.entity, "no fields"));
    }
    for mapping in &definition.fields {
        let draft = field(entity, &mapping.draft)?;
        if draft.owner_managed || !draft.writable {
            return Err(invalid(
                &definition.entity,
                format!("{} must be editable", mapping.draft),
            ));
        }
        let published = protected(entity, &mapping.published)?;
        if draft.field_type != published.field_type {
            return Err(invalid(
                &definition.entity,
                format!(
                    "{} and {} have different types",
                    mapping.draft, mapping.published
                ),
            ));
        }
    }
    let state = protected(entity, &definition.state)?;
    if !STATES.iter().all(|state_value| {
        state
            .options
            .iter()
            .any(|option| option.value == *state_value)
    }) {
        return Err(invalid(
            &definition.entity,
            format!(
                "{} must offer the options {}",
                definition.state,
                STATES.join(", ")
            ),
        ));
    }
    protected(entity, &definition.published_at)?;
    protected(entity, &definition.revised_at)?;
    if definition.format == 0 {
        return Err(invalid(&definition.entity, "format starts at 1"));
    }
    Ok(())
}

pub(crate) fn find(
    registry: &RecordTypeRegistry,
    record_type: &str,
) -> Result<PublishableDefinition, AppError> {
    let mut matching = declared()
        .into_iter()
        .filter(|declaration| declaration.entity == record_type);
    let declaration = matching
        .next()
        .ok_or_else(|| not_publishable(record_type))?;
    if matching.next().is_some() {
        return Err(invalid(record_type, "more than one publication definition"));
    }
    resolve(registry, &declaration)
}
