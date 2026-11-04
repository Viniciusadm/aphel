use aphel_sdk::configuration::ConfigReferenceKind;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::application::{is_relatable_core_type, is_title_field_type};
use aphel_kernel::application::custom_objects::{
    CustomObjectRepository, RelationshipRepository, custom_record_definition,
};
use aphel_kernel::application::fields::{
    FieldDefinitionRepository, RecordTypeRegistry, RelationPath, is_sql_identifier, normalize_color,
};
use aphel_kernel::application::shared::RecordType;
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::domain::custom_objects::{
    Cardinality, CustomObject, CustomObjectData, DEFAULT_TITLE_FIELD_KEY,
    MAX_OBJECT_DESCRIPTION_LENGTH, MAX_OBJECT_LABEL_LENGTH, MAX_RELATIONSHIP_KEY_LENGTH,
    MAX_RELATIONSHIP_LABEL_LENGTH, OnDelete, RelationshipDefinition, is_custom_record_type,
    is_object_key, object_record_type,
};
use aphel_kernel::domain::fields::{FieldConfig, FieldDefinition, FieldKey, FieldType};
use aphel_kernel::errors::AppError;
use aphel_sdk::configuration::sections::common::diff;
use aphel_sdk::configuration::sections::fields::{PackageField, package_fields};
use aphel_sdk::configuration::{
    ApplyContext, ConfigPackage, ConfigSection, Decisions, PackageIssue, PlanChange, PlanItem,
    parse_section,
};

const OBJECT_KIND: &str = "custom_object";
const RELATIONSHIP_KIND: &str = "relationship";
const RELATIONSHIP_INCOMPATIBLE: &str = "relationship_incompatible";
const RELATIONSHIP_KEY_COLLISION: &str = "relationship_key_collision";

use aphel_sdk::configuration::sections::custom_objects::*;

fn object_snapshot(object: &CustomObjectData) -> Vec<(String, Value)> {
    vec![
        ("label_singular".to_owned(), json!(object.label_singular)),
        ("label_plural".to_owned(), json!(object.label_plural)),
        ("description".to_owned(), json!(object.description)),
        ("icon".to_owned(), json!(object.icon)),
        ("color".to_owned(), json!(object.color)),
        ("owner_enabled".to_owned(), json!(object.owner_enabled)),
        ("active".to_owned(), json!(object.is_active)),
    ]
}

fn relationship_snapshot(relationship: &RelationshipDefinition) -> Vec<(String, Value)> {
    vec![
        ("from_label".to_owned(), json!(relationship.from_label)),
        ("to_label".to_owned(), json!(relationship.to_label)),
        ("required".to_owned(), json!(relationship.required)),
        (
            "on_delete".to_owned(),
            json!(relationship.on_delete.as_str()),
        ),
    ]
}

fn object_data(entry: &PackageCustomObject, title_field_key: &str) -> CustomObjectData {
    CustomObjectData {
        label_singular: entry.label_singular.trim().to_owned(),
        label_plural: entry.label_plural.trim().to_owned(),
        description: entry
            .description
            .as_deref()
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(str::to_owned),
        icon: entry.icon.clone().filter(|icon| !icon.trim().is_empty()),
        color: entry.color.as_deref().and_then(normalize_color),
        title_field_key: title_field_key.to_owned(),
        owner_enabled: entry.owner_enabled,
        is_active: true,
    }
}

enum ObjectOperation {
    Create(CustomObject),
    Update(CustomObject, Option<Uuid>),
    Keep,
}

enum RelationshipOperation {
    Create(RelationshipDefinition),
    Update(RelationshipDefinition),
    Keep,
}

fn bounded(value: &str, max: usize) -> bool {
    let text = value.trim();
    !text.is_empty() && text.chars().count() <= max
}

pub struct CustomObjectsSection<O, R, D> {
    registry: Arc<RecordTypeRegistry>,
    objects: O,
    relationships: R,
    definitions: D,
}

impl<O, R, D> CustomObjectsSection<O, R, D>
where
    O: CustomObjectRepository,
    R: RelationshipRepository,
    D: FieldDefinitionRepository,
{
    pub fn new(
        registry: Arc<RecordTypeRegistry>,
        objects: O,
        relationships: R,
        definitions: D,
    ) -> Self {
        Self {
            registry,
            objects,
            relationships,
            definitions,
        }
    }

    async fn reconcile_objects(
        &self,
        scope: &AccessScope,
        entries: &[PackageCustomObject],
        at: DateTime<Utc>,
    ) -> Result<Vec<(PlanItem, ObjectOperation)>, AppError> {
        let existing = self.objects.list().await?;
        let mut position = existing
            .iter()
            .map(|object| object.position)
            .max()
            .unwrap_or(-1);
        let mut plans = Vec::new();
        for entry in entries {
            let item = PlanItem::new(
                CUSTOM_OBJECTS_SECTION,
                OBJECT_KIND,
                entry.key.clone(),
                entry.label_plural.trim(),
            );
            match existing.iter().find(|object| object.key == entry.key) {
                None => {
                    position += 1;
                    let data = object_data(entry, DEFAULT_TITLE_FIELD_KEY);
                    let changes = diff(&[], &object_snapshot(&data));
                    plans.push((
                        item.created(changes),
                        ObjectOperation::Create(CustomObject {
                            id: Uuid::now_v7(),
                            key: entry.key.clone(),
                            data,
                            position,
                            module_key: None,
                            entity_definition: None,
                            is_internal: false,
                            created_by: Some(scope.user_id().clone()),
                            created_at: at,
                            updated_at: at,
                        }),
                    ));
                }
                Some(current) => {
                    let (title_key, title_field) =
                        self.title_key(current, &entry.title_field_key).await?;
                    let data = object_data(entry, &title_key);
                    let mut changes =
                        diff(&object_snapshot(&current.data), &object_snapshot(&data));
                    if title_key != current.data.title_field_key {
                        changes.push(PlanChange::new(
                            "title_field_key",
                            json!(current.data.title_field_key),
                            json!(title_key),
                        ));
                    }
                    let operation = if changes.is_empty() {
                        ObjectOperation::Keep
                    } else {
                        ObjectOperation::Update(
                            CustomObject {
                                data,
                                updated_at: at,
                                ..current.clone()
                            },
                            title_field,
                        )
                    };
                    plans.push((item.compared(changes), operation));
                }
            }
        }
        Ok(plans)
    }

    async fn title_key(
        &self,
        object: &CustomObject,
        requested: &str,
    ) -> Result<(String, Option<Uuid>), AppError> {
        if requested == object.data.title_field_key {
            return Ok((requested.to_owned(), None));
        }
        let record_type = RecordType::parse(object.record_type())?;
        let usable = self
            .definitions
            .list(&record_type)
            .await?
            .into_iter()
            .find(|field| {
                field.key.as_str() == requested
                    && !field.is_system
                    && field.is_active
                    && is_title_field_type(field.field_type)
            });
        Ok(match usable {
            Some(field) => (requested.to_owned(), Some(field.id)),
            None => (object.data.title_field_key.clone(), None),
        })
    }

    async fn side_collisions(
        &self,
        package: &ConfigPackage,
        existing: &[RelationshipDefinition],
        entries: &[PackageRelationship],
    ) -> Result<BTreeMap<String, String>, AppError> {
        let registry = package_registry(&self.registry, package);
        let declared_fields = package_fields(package);
        let mut custom_fields: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        let mut collisions = BTreeMap::new();
        for entry in entries {
            for (record_type, side_key) in [
                (&entry.from_type, &entry.key),
                (&entry.to_type, &entry.inverse_key),
            ] {
                if !custom_fields.contains_key(record_type) {
                    let keys = match RecordType::parse(record_type) {
                        Ok(parsed) => self
                            .definitions
                            .list(&parsed)
                            .await?
                            .into_iter()
                            .map(|field| field.key.as_str().to_owned())
                            .collect(),
                        Err(_) => BTreeSet::new(),
                    };
                    custom_fields.insert(record_type.clone(), keys);
                }
                let field_taken = custom_fields
                    .get(record_type)
                    .is_some_and(|keys| keys.contains(side_key))
                    || declared_field(&declared_fields, record_type, side_key)
                    || RecordType::parse(record_type)
                        .ok()
                        .and_then(|parsed| registry.get(&parsed).cloned())
                        .is_some_and(|definition| {
                            definition.system_field_by_key(side_key).is_some()
                                || definition
                                    .relation_by_key(side_key)
                                    .is_some_and(|relation| {
                                        !relation.paths.iter().any(|path| {
                                            matches!(path, RelationPath::Relationship { .. })
                                        })
                                    })
                        });
                let relationship_taken = existing
                    .iter()
                    .filter(|other| other.key != entry.key)
                    .any(|other| {
                        (&other.from_type == record_type && &other.key == side_key)
                            || (&other.to_type == record_type && &other.inverse_key == side_key)
                    })
                    || entries
                        .iter()
                        .filter(|other| other.key != entry.key)
                        .any(|other| {
                            (&other.from_type == record_type && &other.key == side_key)
                                || (&other.to_type == record_type && &other.inverse_key == side_key)
                        });
                if field_taken || relationship_taken {
                    collisions.entry(entry.key.clone()).or_insert_with(|| {
                        format!(
                            "{side_key} is already used by a field or relation of {record_type}"
                        )
                    });
                }
            }
        }
        Ok(collisions)
    }

    async fn reconcile_relationships(
        &self,
        scope: &AccessScope,
        package: &ConfigPackage,
        entries: &[PackageRelationship],
        at: DateTime<Utc>,
    ) -> Result<Vec<(PlanItem, RelationshipOperation)>, AppError> {
        let existing = self.relationships.list().await?;
        let collisions = self.side_collisions(package, &existing, entries).await?;
        let mut plans = Vec::new();
        for entry in entries {
            let item = PlanItem::new(
                CUSTOM_OBJECTS_SECTION,
                RELATIONSHIP_KIND,
                entry.key.clone(),
                format!("{} → {}", entry.from_type, entry.to_type),
            );
            let (Some(cardinality), Some(on_delete)) = (
                Cardinality::parse(&entry.cardinality),
                OnDelete::parse(&entry.on_delete),
            ) else {
                continue;
            };
            let proposed = RelationshipDefinition {
                id: Uuid::now_v7(),
                key: entry.key.clone(),
                inverse_key: entry.inverse_key.clone(),
                from_type: entry.from_type.clone(),
                to_type: entry.to_type.clone(),
                cardinality,
                from_label: entry.from_label.trim().to_owned(),
                to_label: entry.to_label.trim().to_owned(),
                required: entry.required,
                on_delete,
                created_by: Some(scope.user_id().clone()),
                created_at: at,
                updated_at: at,
            };
            let current = existing
                .iter()
                .find(|relationship| relationship.key == entry.key);
            if let Some(message) = collisions.get(&entry.key) {
                let changes = diff(
                    &current.map(relationship_snapshot).unwrap_or_default(),
                    &relationship_snapshot(&proposed),
                );
                plans.push((
                    item.conflicting(changes, RELATIONSHIP_KEY_COLLISION, message.clone(), false),
                    RelationshipOperation::Keep,
                ));
                continue;
            }
            match current {
                None => {
                    let changes = diff(&[], &relationship_snapshot(&proposed));
                    plans.push((
                        item.created(changes),
                        RelationshipOperation::Create(proposed),
                    ));
                }
                Some(current) => {
                    let updated = RelationshipDefinition {
                        id: current.id,
                        created_by: current.created_by.clone(),
                        created_at: current.created_at,
                        ..proposed
                    };
                    let changes = diff(
                        &relationship_snapshot(current),
                        &relationship_snapshot(&updated),
                    );
                    let compatible = current.inverse_key == updated.inverse_key
                        && current.from_type == updated.from_type
                        && current.to_type == updated.to_type
                        && current.cardinality == updated.cardinality;
                    if !compatible {
                        plans.push((
                            item.conflicting(
                                changes,
                                RELATIONSHIP_INCOMPATIBLE,
                                "the relationship exists with other types, inverse key or cardinality",
                                false,
                            ),
                            RelationshipOperation::Keep,
                        ));
                    } else if changes.is_empty() {
                        plans.push((item.compared(changes), RelationshipOperation::Keep));
                    } else {
                        plans.push((
                            item.compared(changes),
                            RelationshipOperation::Update(updated),
                        ));
                    }
                }
            }
        }
        Ok(plans)
    }
}

fn declared_field(fields: &[PackageField], record_type: &str, key: &str) -> bool {
    fields
        .iter()
        .any(|field| field.record_type == record_type && field.key == key)
}

fn parse(content: &Value) -> Result<PackageCustomObjects, AppError> {
    parse_section(content).map_err(|issue| AppError::Validation(issue.message))
}

fn relatable(record_type: &str, declared: &BTreeSet<String>, existing: &[CustomObject]) -> bool {
    aphel_kernel::modules::try_app()
        .is_some_and(|app| is_relatable_core_type(app.record_types(), record_type))
        || (is_custom_record_type(record_type)
            && (declared.contains(record_type)
                || existing
                    .iter()
                    .any(|object| object.record_type() == record_type)))
}

#[async_trait]
impl<O, R, D, Tx> ConfigSection<Tx> for CustomObjectsSection<O, R, D>
where
    Tx: Send,
    O: CustomObjectRepository<Tx = Tx>,
    R: RelationshipRepository<Tx = Tx>,
    D: FieldDefinitionRepository<Tx = Tx>,
{
    fn key(&self) -> &'static str {
        CUSTOM_OBJECTS_SECTION
    }

    async fn export(&self, _scope: &AccessScope) -> Result<Option<Value>, AppError> {
        let objects: Vec<CustomObject> = self
            .objects
            .list()
            .await?
            .into_iter()
            .filter(|object| object.data.is_active)
            .collect();
        let exported: BTreeSet<String> = objects.iter().map(CustomObject::record_type).collect();
        let relationships = self
            .relationships
            .list()
            .await?
            .into_iter()
            .filter(|relationship| {
                [&relationship.from_type, &relationship.to_type]
                    .iter()
                    .all(|record_type| {
                        !is_custom_record_type(record_type) || exported.contains(*record_type)
                    })
            })
            .map(|relationship| PackageRelationship {
                key: relationship.key,
                inverse_key: relationship.inverse_key,
                from_type: relationship.from_type,
                to_type: relationship.to_type,
                cardinality: relationship.cardinality.as_str().to_owned(),
                from_label: relationship.from_label,
                to_label: relationship.to_label,
                required: relationship.required,
                on_delete: relationship.on_delete.as_str().to_owned(),
            })
            .collect::<Vec<_>>();
        if objects.is_empty() && relationships.is_empty() {
            return Ok(None);
        }
        let section = PackageCustomObjects {
            objects: objects
                .into_iter()
                .map(|object| PackageCustomObject {
                    key: object.key,
                    label_singular: object.data.label_singular,
                    label_plural: object.data.label_plural,
                    description: object.data.description,
                    icon: object.data.icon,
                    color: object.data.color,
                    owner_enabled: object.data.owner_enabled,
                    title_field_key: object.data.title_field_key,
                })
                .collect(),
            relationships,
        };
        Ok(Some(json!(section)))
    }

    async fn validate(
        &self,
        _scope: &AccessScope,
        _package: &ConfigPackage,
        content: &Value,
    ) -> Result<Vec<PackageIssue>, AppError> {
        let section: PackageCustomObjects = match parse_section(content) {
            Ok(section) => section,
            Err(issue) => return Ok(vec![issue]),
        };
        let mut issues = Vec::new();
        if section.objects.len() > MAX_PACKAGE_CUSTOM_OBJECTS {
            issues.push(PackageIssue::new(
                "objects",
                "too_many_custom_objects",
                format!("a package can have at most {MAX_PACKAGE_CUSTOM_OBJECTS} custom objects"),
            ));
        }
        if section.relationships.len() > MAX_PACKAGE_RELATIONSHIPS {
            issues.push(PackageIssue::new(
                "relationships",
                "too_many_relationships",
                format!("a package can have at most {MAX_PACKAGE_RELATIONSHIPS} relationships"),
            ));
        }
        let mut declared = BTreeSet::new();
        for (index, object) in section.objects.iter().enumerate() {
            let path = format!("objects.{index}");
            if !is_object_key(&object.key) {
                issues.push(PackageIssue::new(
                    format!("{path}.key"),
                    "invalid_key",
                    "must have lowercase letters and single underscores, starting with a letter (max 40)",
                ));
            } else if !declared.insert(object_record_type(&object.key)) {
                issues.push(PackageIssue::new(
                    format!("{path}.key"),
                    "duplicated_key",
                    "is declared more than once",
                ));
            }
            for (property, value) in [
                ("label_singular", &object.label_singular),
                ("label_plural", &object.label_plural),
            ] {
                if !bounded(value, MAX_OBJECT_LABEL_LENGTH) {
                    issues.push(PackageIssue::new(
                        format!("{path}.{property}"),
                        "invalid_label",
                        format!("must have between 1 and {MAX_OBJECT_LABEL_LENGTH} characters"),
                    ));
                }
            }
            if object
                .description
                .as_deref()
                .is_some_and(|text| text.chars().count() > MAX_OBJECT_DESCRIPTION_LENGTH)
            {
                issues.push(PackageIssue::new(
                    format!("{path}.description"),
                    "invalid_description",
                    format!("must have at most {MAX_OBJECT_DESCRIPTION_LENGTH} characters"),
                ));
            }
            if object
                .color
                .as_deref()
                .is_some_and(|color| normalize_color(color).is_none())
            {
                issues.push(PackageIssue::new(
                    format!("{path}.color"),
                    "invalid_color",
                    "must be a color like #1f6feb",
                ));
            }
        }
        let existing = self.objects.list().await?;
        let mut keys = BTreeSet::new();
        for (index, relationship) in section.relationships.iter().enumerate() {
            let path = format!("relationships.{index}");
            for (property, value) in [
                ("key", &relationship.key),
                ("inverse_key", &relationship.inverse_key),
            ] {
                if value.len() > MAX_RELATIONSHIP_KEY_LENGTH || !is_sql_identifier(value) {
                    issues.push(PackageIssue::new(
                        format!("{path}.{property}"),
                        "invalid_key",
                        "must have lowercase letters, digits or underscores and start with a letter",
                    ));
                }
            }
            if !keys.insert(relationship.key.clone()) {
                issues.push(PackageIssue::new(
                    format!("{path}.key"),
                    "duplicated_key",
                    "is declared more than once",
                ));
            }
            if relationship.from_type == relationship.to_type
                && relationship.key == relationship.inverse_key
            {
                issues.push(PackageIssue::new(
                    format!("{path}.inverse_key"),
                    "invalid_key",
                    "must differ from key when both sides are the same record type",
                ));
            }
            for (property, value) in [
                ("from_type", &relationship.from_type),
                ("to_type", &relationship.to_type),
            ] {
                if !relatable(value, &declared, &existing) {
                    issues.push(PackageIssue::new(
                        format!("{path}.{property}"),
                        "unknown_record_type",
                        "must be an available record type or a custom object from the package or installation",
                    ));
                }
            }
            if Cardinality::parse(&relationship.cardinality).is_none() {
                issues.push(PackageIssue::new(
                    format!("{path}.cardinality"),
                    "invalid_cardinality",
                    "must be one_to_one, one_to_many or many_to_many",
                ));
            }
            if OnDelete::parse(&relationship.on_delete).is_none() {
                issues.push(PackageIssue::new(
                    format!("{path}.on_delete"),
                    "invalid_on_delete",
                    "must be restrict or unlink",
                ));
            }
            for (property, value) in [
                ("from_label", &relationship.from_label),
                ("to_label", &relationship.to_label),
            ] {
                if !bounded(value, MAX_RELATIONSHIP_LABEL_LENGTH) {
                    issues.push(PackageIssue::new(
                        format!("{path}.{property}"),
                        "invalid_label",
                        format!(
                            "must have between 1 and {MAX_RELATIONSHIP_LABEL_LENGTH} characters"
                        ),
                    ));
                }
            }
        }
        Ok(issues)
    }

    async fn plan(
        &self,
        scope: &AccessScope,
        package: &ConfigPackage,
        content: &Value,
    ) -> Result<Vec<PlanItem>, AppError> {
        let section = parse(content)?;
        let at = DateTime::<Utc>::UNIX_EPOCH;
        let mut items: Vec<PlanItem> = self
            .reconcile_objects(scope, &section.objects, at)
            .await?
            .into_iter()
            .map(|(item, _)| item)
            .collect();
        items.extend(
            self.reconcile_relationships(scope, package, &section.relationships, at)
                .await?
                .into_iter()
                .map(|(item, _)| item),
        );
        Ok(items)
    }

    async fn apply(
        &self,
        tx: &mut Tx,
        scope: &AccessScope,
        package: &ConfigPackage,
        content: &Value,
        decisions: &Decisions,
        context: &mut ApplyContext,
    ) -> Result<(), AppError> {
        let section = parse(content)?;
        let declared_fields = package_fields(package);
        let mut changed = false;
        for (item, operation) in self
            .reconcile_objects(scope, &section.objects, context.at)
            .await?
        {
            match operation {
                ObjectOperation::Create(object) => {
                    let definition = custom_record_definition(&object)?;
                    self.objects.insert(tx, &object).await?;
                    self.objects.seed_system_fields(tx, &definition).await?;
                    let record_type = object.record_type();
                    let title_declared = declared_fields.iter().any(|field| {
                        field.record_type == record_type && field.key == DEFAULT_TITLE_FIELD_KEY
                    });
                    if !title_declared {
                        self.definitions
                            .insert(
                                tx,
                                &FieldDefinition {
                                    id: Uuid::now_v7(),
                                    record_type: object.record_type(),
                                    key: FieldKey::new(DEFAULT_TITLE_FIELD_KEY).map_err(|_| {
                                        AppError::Internal("invalid title field key".to_owned())
                                    })?,
                                    label: "Nome".to_owned(),
                                    field_type: FieldType::Text,
                                    is_system: false,
                                    column_name: None,
                                    is_required: true,
                                    is_unique: false,
                                    is_searchable: true,
                                    is_personal: false,
                                    is_sensitive: false,
                                    module_key: None,
                                    is_active: true,
                                    position: definition.system_fields.len() as i32,
                                    config: FieldConfig::default(),
                                    options: Vec::new(),
                                },
                            )
                            .await?;
                    }
                    self.objects
                        .grant_default_permissions(tx, &object.record_type(), None)
                        .await?;
                    context.resolve(
                        ConfigReferenceKind::new("objects.object"),
                        &object.key,
                        object.id,
                    );
                    changed = true;
                }
                ObjectOperation::Update(object, title_field) if decisions.executes(&item) => {
                    self.objects.update(tx, &object).await?;
                    if let Some(field_id) = title_field {
                        self.objects.refresh_titles(tx, &object, field_id).await?;
                    }
                    self.objects
                        .seed_system_fields(tx, &custom_record_definition(&object)?)
                        .await?;
                    context.resolve(
                        ConfigReferenceKind::new("objects.object"),
                        &object.key,
                        object.id,
                    );
                    changed = true;
                }
                ObjectOperation::Update(..) | ObjectOperation::Keep => {}
            }
        }
        for (item, operation) in self
            .reconcile_relationships(scope, package, &section.relationships, context.at)
            .await?
        {
            match operation {
                RelationshipOperation::Create(relationship) => {
                    self.relationships.insert(tx, &relationship).await?;
                    changed = true;
                }
                RelationshipOperation::Update(relationship) if decisions.executes(&item) => {
                    self.relationships.update(tx, &relationship).await?;
                    changed = true;
                }
                RelationshipOperation::Update(_) | RelationshipOperation::Keep => {}
            }
        }
        if changed {
            self.objects.bump_schema_version(tx).await?;
        }
        Ok(())
    }
}
