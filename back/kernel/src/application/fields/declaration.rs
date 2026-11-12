use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntityDeclaration {
    #[serde(default = "dynamic_storage")]
    pub storage: super::EntityStorage,
    #[serde(default)]
    pub indexes: Vec<super::EntityIndex>,
    #[serde(default)]
    pub references: Vec<super::SoftReference>,
    #[serde(default)]
    pub traits: std::collections::BTreeMap<String, Value>,
    #[serde(default)]
    pub bulk_fields: Vec<String>,
    pub key: String,
    pub label: String,
    pub label_plural: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default = "default_owner_enabled")]
    pub owner_enabled: bool,
    #[serde(default)]
    pub permission_scope: Option<String>,
    #[serde(default)]
    pub internal: bool,
    #[serde(default)]
    pub title_field: Option<String>,
    #[serde(default)]
    pub fields: Vec<FieldDef>,
    #[serde(default)]
    pub relations: Vec<RelationDeclaration>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RelationDeclaration {
    pub key: String,
    pub field: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldDef {
    pub key: String,
    pub label: String,
    #[serde(rename = "type")]
    pub field_type: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub searchable: bool,
    #[serde(default)]
    pub unique: bool,
    #[serde(default)]
    pub personal: bool,
    #[serde(default)]
    pub sensitive: bool,
    #[serde(default)]
    pub min: Option<Value>,
    #[serde(default)]
    pub max: Option<Value>,
    #[serde(default)]
    pub decimals: Option<u32>,
    #[serde(default)]
    pub max_length: Option<u32>,
    #[serde(default)]
    pub pattern: Option<String>,
    #[serde(default)]
    pub mask: Option<String>,
    #[serde(default)]
    pub currency: Option<String>,
    #[serde(default)]
    pub default: Option<Value>,
    #[serde(default)]
    pub target: Option<String>,
    #[serde(default)]
    pub read_only: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub writers: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub published_by: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub translatable: bool,
    #[serde(skip)]
    pub translation_of: Option<String>,
    #[serde(default)]
    pub options: Vec<OptionDef>,
}

pub const DRAFT_PREFIX: &str = "draft_";
pub const PUBLISHED_AT_FIELD: &str = "published_at";
pub const REVISED_AT_FIELD: &str = "revised_at";
pub const PUBLICATION_STATES: [&str; 3] = ["draft", "published", "withdrawn"];

pub const TRANSLATIONS_SUFFIX: &str = "_translations";
pub const MAX_TRANSLATIONS_LENGTH: u32 = 1_000_000;

pub fn translations_field(key: &str) -> String {
    format!("{key}{TRANSLATIONS_SUFFIX}")
}

fn expand_translatable(fields: &[FieldDef]) -> Result<Vec<FieldDef>, crate::errors::AppError> {
    let mut expanded = Vec::with_capacity(fields.len());
    for field in fields {
        expanded.push(field.clone());
        if !field.translatable {
            continue;
        }
        if !["text", "long_text", "document"].contains(&field.field_type.as_str()) {
            return Err(crate::errors::AppError::Configuration(format!(
                "translatable field {} must be text, long_text or document",
                field.key
            )));
        }
        let mut translations = field.clone();
        translations.key = translations_field(&field.key);
        translations.field_type = "document".to_owned();
        translations.translatable = false;
        translations.translation_of = Some(field.key.clone());
        translations.required = false;
        translations.searchable = false;
        translations.unique = false;
        translations.default = None;
        translations.pattern = None;
        translations.mask = None;
        translations.max_length = Some(MAX_TRANSLATIONS_LENGTH);
        expanded.push(translations);
    }
    Ok(expanded)
}

pub fn publication_state_field(writer: &str) -> String {
    format!("{writer}_state")
}

fn valid_writer(writer: &str) -> bool {
    !writer.is_empty()
        && writer
            .chars()
            .all(|char| char.is_ascii_lowercase() || char.is_ascii_digit() || char == '_')
}

fn expand_published(
    fields: &[FieldDef],
) -> Result<(Vec<FieldDef>, Option<String>), crate::errors::AppError> {
    let fields = expand_translatable(fields)?;
    let fields = fields.as_slice();
    let mut expanded = Vec::with_capacity(fields.len());
    let mut writer: Option<String> = None;
    for field in fields {
        let Some(by) = &field.published_by else {
            expanded.push(field.clone());
            continue;
        };
        let problem = if !valid_writer(by) {
            Some("names an invalid module")
        } else if field.read_only || !field.writers.is_empty() {
            Some("cannot also be read_only or declare writers")
        } else if field.unique {
            Some("cannot be unique")
        } else if writer.as_deref().is_some_and(|current| current != by) {
            Some("uses a different module than the other published fields")
        } else {
            None
        };
        if let Some(problem) = problem {
            return Err(crate::errors::AppError::Configuration(format!(
                "published field {} {problem}",
                field.key
            )));
        }
        writer = Some(by.clone());
        let mut draft = field.clone();
        draft.key = format!("{DRAFT_PREFIX}{}", field.key);
        draft.published_by = None;
        draft.translation_of = field
            .translation_of
            .as_ref()
            .map(|base| format!("{DRAFT_PREFIX}{base}"));
        let mut published = field.clone();
        published.published_by = None;
        published.required = false;
        published.searchable = false;
        published.default = None;
        published.read_only = true;
        published.writers = vec![by.clone()];
        expanded.push(draft);
        expanded.push(published);
    }
    Ok((expanded, writer))
}

impl FieldDef {
    pub fn config(
        &self,
    ) -> Result<crate::domain::fields::FieldConfig, Vec<crate::application::fields::ConfigError>>
    {
        let mut object = serde_json::Map::new();
        let mut put = |key: &str, value: Option<Value>| {
            if let Some(value) = value {
                object.insert(key.to_owned(), value);
            }
        };
        put("min", self.min.clone());
        put("max", self.max.clone());
        put("decimals", self.decimals.map(Value::from));
        put("max_length", self.max_length.map(Value::from));
        put("pattern", self.pattern.clone().map(Value::String));
        put("mask", self.mask.clone().map(Value::String));
        put("currency", self.currency.clone().map(Value::String));
        put("default_value", self.default.clone());
        put("target_record_type", self.target.clone().map(Value::String));
        crate::application::fields::config_from_json(&Value::Object(object))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OptionDef {
    pub value: String,
    pub label: String,
    #[serde(default)]
    pub color: Option<String>,
}

fn default_owner_enabled() -> bool {
    true
}
fn dynamic_storage() -> super::EntityStorage {
    super::EntityStorage::Dynamic
}

impl EntityDeclaration {
    pub fn expanded_fields(&self) -> Result<Vec<FieldDef>, crate::errors::AppError> {
        expand_published(&self.fields).map(|(fields, _)| fields)
    }

    pub fn publisher(&self) -> Option<&str> {
        self.fields
            .iter()
            .find_map(|field| field.published_by.as_deref())
    }

    pub fn field_keys(&self) -> Vec<String> {
        let mut keys: Vec<String> = self
            .expanded_fields()
            .unwrap_or_else(|_| self.fields.clone())
            .into_iter()
            .map(|field| field.key)
            .collect();
        if let Some(writer) = self.publisher() {
            keys.extend([
                publication_state_field(writer),
                PUBLISHED_AT_FIELD.to_owned(),
                REVISED_AT_FIELD.to_owned(),
            ]);
        }
        keys
    }

    pub fn definition(
        &self,
        key: &str,
        read_permission: crate::domain::access::Permission,
    ) -> Result<super::EntityDefinition, crate::errors::AppError> {
        use crate::domain::fields::FieldType;
        let record_type = crate::application::shared::RecordType::parse(key)?;
        let permission_key = read_permission
            .as_str()
            .strip_suffix(".read")
            .unwrap_or(key)
            .to_owned();
        let mut definition =
            super::EntityDefinition::new(record_type, "custom_records", read_permission);
        definition.key = key.to_owned();
        definition.create_permission = Some(crate::domain::access::Permission::parse(format!(
            "{permission_key}.create"
        ))?);
        definition.update_permission = Some(crate::domain::access::Permission::parse(format!(
            "{permission_key}.update"
        ))?);
        definition.delete_permission = Some(crate::domain::access::Permission::parse(format!(
            "{permission_key}.delete"
        ))?);
        definition.storage = self.storage;
        definition.references = self.references.clone();
        for relation in &self.relations {
            let target = self
                .references
                .iter()
                .find(|reference| reference.field == relation.field)
                .and_then(|reference| match &reference.target {
                    super::ReferenceTarget::Entity { entity } => Some(entity.clone()),
                    _ => None,
                })
                .ok_or_else(|| {
                    crate::errors::AppError::Configuration(format!(
                        "relation {} needs a reference to an entity in field {}",
                        relation.key, relation.field
                    ))
                })?;
            let label = self
                .fields
                .iter()
                .find(|field| field.key == relation.field)
                .map(|field| field.label.clone())
                .unwrap_or_else(|| relation.key.clone());
            definition = definition.relation(
                super::RecordRelation::new(
                    &relation.key,
                    label,
                    crate::application::shared::RecordType::parse(target)?,
                )
                .path(super::RelationPath::BelongsTo {
                    column: relation.field.clone(),
                }),
            );
        }
        for reference in &self.references {
            let field = self
                .fields
                .iter()
                .find(|field| field.key == reference.field)
                .ok_or_else(|| {
                    crate::errors::AppError::Configuration("unknown reference field".to_owned())
                })?;
            if field.required && reference.on_delete == super::DeletePolicy::Clear {
                return Err(crate::errors::AppError::Configuration(
                    "required references cannot be cleared".to_owned(),
                ));
            }
        }
        for reference in &self.references {
            if let super::ReferenceTarget::Trait { type_field, .. } = &reference.target {
                let field = self
                    .fields
                    .iter()
                    .find(|field| field.key == *type_field)
                    .ok_or_else(|| {
                        crate::errors::AppError::Configuration(
                            "unknown reference type field".to_owned(),
                        )
                    })?;
                if reference.on_delete == super::DeletePolicy::Clear && field.required {
                    return Err(crate::errors::AppError::Configuration(
                        "required reference types cannot be cleared".to_owned(),
                    ));
                }
            }
        }
        definition.capabilities = super::EntityTraits::default()
            .importable()
            .exportable()
            .personal_data();
        if self.storage == super::EntityStorage::Table {
            return Err(crate::errors::AppError::Configuration(
                "declarative entities cannot select native tables".to_owned(),
            ));
        }
        if self.storage == super::EntityStorage::Managed {
            definition = definition
                .managed(key)
                .soft_delete_column("deleted_at")
                .archive_column("archived_at");
            let (fields, publisher) = expand_published(&self.fields)?;
            for field in &fields {
                if [
                    "id",
                    "created_at",
                    "updated_at",
                    "created_by",
                    "owner_id",
                    "deleted_at",
                    "archived_at",
                ]
                .contains(&field.key.as_str())
                    || publisher.as_deref().is_some_and(|writer| {
                        [
                            publication_state_field(writer).as_str(),
                            PUBLISHED_AT_FIELD,
                            REVISED_AT_FIELD,
                        ]
                        .contains(&field.key.as_str())
                    })
                {
                    return Err(crate::errors::AppError::Configuration(format!(
                        "reserved entity field {}",
                        field.key
                    )));
                }
                let kind = FieldType::parse(&field.field_type).ok_or_else(|| {
                    crate::errors::AppError::Configuration(format!(
                        "unknown field type {}",
                        field.field_type
                    ))
                })?;
                let mut system =
                    super::SystemField::new(&field.key, field.label.clone(), kind, &field.key)?;
                system.required = field.required;
                system.searchable = field.searchable;
                system.personal = field.personal;
                system.sensitive = field.sensitive;
                system.owner_managed = field.read_only;
                if !field.writers.is_empty() && !field.read_only {
                    return Err(crate::errors::AppError::Configuration(format!(
                        "field {} declares writers but is not read_only",
                        field.key
                    )));
                }
                if let Some(writer) = field.writers.iter().find(|writer| {
                    writer.is_empty()
                        || !writer.chars().all(|char| {
                            char.is_ascii_lowercase() || char.is_ascii_digit() || char == '_'
                        })
                }) {
                    return Err(crate::errors::AppError::Configuration(format!(
                        "invalid writer {writer:?} on field {}",
                        field.key
                    )));
                }
                system.writers = field.writers.clone();
                system.translation_of = field.translation_of.clone();
                system.config = field.config().map_err(|_| {
                    crate::errors::AppError::Configuration(format!(
                        "invalid configuration for {}",
                        field.key
                    ))
                })?;
                if !system.config.validate(kind).is_empty() {
                    return Err(crate::errors::AppError::Configuration(format!(
                        "invalid configuration for {}",
                        field.key
                    )));
                }
                for option in &field.options {
                    system = system.with_option(
                        &option.value,
                        option.label.clone(),
                        option.color.as_deref(),
                    );
                }
                definition = definition.system_field(system);
                if field.unique {
                    definition =
                        definition.index(&format!("{}_unique", field.key), &[&field.key], true);
                }
            }
            if let Some(writer) = &publisher {
                let protect = |mut field: super::SystemField| {
                    field.owner_managed = true;
                    field.writers = vec![writer.clone()];
                    field
                };
                let state_key = publication_state_field(writer);
                let mut state = protect(super::SystemField::new(
                    &state_key,
                    crate::labels("Publication status"),
                    FieldType::Select,
                    &state_key,
                )?);
                state.required = true;
                state.config.default_value = Some(crate::domain::fields::RawValue::Text(
                    PUBLICATION_STATES[0].to_owned(),
                ));
                for (value, label, color) in [
                    (PUBLICATION_STATES[0], "Draft", "gray"),
                    (PUBLICATION_STATES[1], "Published", "green"),
                    (PUBLICATION_STATES[2], "Withdrawn", "gray"),
                ] {
                    state = state.with_option(value, crate::labels(label), Some(color));
                }
                definition = definition.system_field(state);
                for (key, label) in [
                    (PUBLISHED_AT_FIELD, "Published at"),
                    (REVISED_AT_FIELD, "Revised at"),
                ] {
                    definition = definition.system_field(protect(super::SystemField::new(
                        key,
                        crate::labels(label),
                        FieldType::DateTime,
                        key,
                    )?));
                }
            }
            if self.owner_enabled {
                definition =
                    definition
                        .owner_column("owner_id")
                        .system_field(super::SystemField::new(
                            "owner_id",
                            "Owner",
                            FieldType::User,
                            "owner_id",
                        )?);
            }
            definition
                .system_fields
                .extend(super::builtin::timestamps()?);
            definition.system_fields.push(
                super::SystemField::new(
                    "created_by",
                    crate::labels("Created by"),
                    FieldType::User,
                    "created_by",
                )?
                .read_only(),
            );
            definition.indexes.extend(self.indexes.clone());
            let working = |key: &str| match self.fields.iter().find(|field| field.key == key) {
                Some(field) if field.published_by.is_some() => format!("{DRAFT_PREFIX}{key}"),
                _ => key.to_owned(),
            };
            if let Some(title) = &self.title_field {
                if !fields.iter().any(|field| &field.key == title) {
                    return Err(crate::errors::AppError::Configuration(format!(
                        "unknown title field {title}"
                    )));
                }
                definition.title_field = Some(working(title));
            } else if let Some(field) = self
                .fields
                .iter()
                .find(|field| field.key == "name")
                .or_else(|| {
                    self.fields
                        .iter()
                        .find(|field| ["text", "email"].contains(&field.field_type.as_str()))
                })
            {
                definition.title_field = Some(working(&field.key));
            }
        }
        definition.capabilities.traits.extend(self.traits.clone());
        definition.capabilities.bulk_fields = self.bulk_fields.clone();
        definition.check()?;
        Ok(definition)
    }
}
