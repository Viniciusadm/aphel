use std::collections::HashMap;

use uuid::Uuid;

use crate::application::ValueKind;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::fields::{
    EntityDefinition, FieldDefinitionReader, FieldSet, ItemTable, RecordTypeRegistry, RelationPath,
    ResolvedField,
};
use aphel_sdk::platform::application::shared::RecordType;
use aphel_sdk::platform::domain::fields::{FieldDefinition, FieldType};

pub fn kind_of(field_type: FieldType) -> Option<ValueKind> {
    match field_type.base_type() {
        FieldType::Number | FieldType::Currency => Some(ValueKind::Number),
        FieldType::Date | FieldType::DateTime => Some(ValueKind::Date),
        FieldType::Text
        | FieldType::LongText
        | FieldType::Url
        | FieldType::Email
        | FieldType::Phone
        | FieldType::Select => Some(ValueKind::Text),
        FieldType::Checkbox => Some(ValueKind::Bool),
        FieldType::MultiSelect | FieldType::User | FieldType::Record => None,
        FieldType::Custom { .. } => unreachable!("native field representation"),
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FormulaField {
    pub field: ResolvedField,
    pub kind: Option<ValueKind>,
}

impl FormulaField {
    pub fn key(&self) -> &str {
        self.field.key()
    }

    pub fn definition(&self) -> &FieldDefinition {
        &self.field.definition
    }

    pub fn is_formula(&self) -> bool {
        self.field.definition.is_formula()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AggregateColumn {
    ItemColumn(String),
    System(String),
    Custom(Uuid),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelatedField {
    pub key: String,
    pub label: String,
    pub column: AggregateColumn,
    pub is_formula: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AggregateSource {
    Items(Box<ItemTable>),
    HasMany {
        target: Box<EntityDefinition>,
        column: String,
    },
    Relationship {
        target: Box<EntityDefinition>,
        relationship_id: Uuid,
        inverse: bool,
    },
}

impl AggregateSource {
    pub fn target_type(&self) -> Option<&RecordType> {
        match self {
            Self::Items(_) => None,
            Self::HasMany { target, .. } | Self::Relationship { target, .. } => {
                Some(&target.record_type)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FormulaRelation {
    pub key: String,
    pub label: String,
    pub source: AggregateSource,
    pub fields: Vec<RelatedField>,
}

impl FormulaRelation {
    pub fn field(&self, key: &str) -> Option<&RelatedField> {
        self.fields.iter().find(|field| field.key == key)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FormulaContext {
    pub record_type: EntityDefinition,
    pub fields: Vec<FormulaField>,
    pub relations: Vec<FormulaRelation>,
}

impl FormulaContext {
    pub fn field(&self, key: &str) -> Option<&FormulaField> {
        self.fields.iter().find(|field| field.key() == key)
    }

    pub fn relation(&self, key: &str) -> Option<&FormulaRelation> {
        self.relations.iter().find(|relation| relation.key == key)
    }

    pub fn formulas(&self) -> impl Iterator<Item = &FormulaField> {
        self.fields.iter().filter(|field| field.is_formula())
    }
}

pub struct FormulaWorkspace<'a, D: ?Sized> {
    registry: &'a RecordTypeRegistry,
    definitions: &'a D,
    cache: HashMap<RecordType, Vec<FieldDefinition>>,
    overrides: Vec<FieldDefinition>,
}

impl<'a, D> FormulaWorkspace<'a, D>
where
    D: FieldDefinitionReader + ?Sized,
{
    pub fn new(registry: &'a RecordTypeRegistry, definitions: &'a D) -> Self {
        Self {
            registry,
            definitions,
            cache: HashMap::new(),
            overrides: Vec::new(),
        }
    }

    pub fn registry(&self) -> &'a RecordTypeRegistry {
        self.registry
    }

    pub fn with_override(mut self, field: FieldDefinition) -> Self {
        self.overrides.push(field);
        self
    }

    pub async fn field_set(&mut self, record_type: &RecordType) -> Result<FieldSet, AppError> {
        let definition = self.registry.require(record_type.as_str())?;
        if !self.cache.contains_key(record_type) {
            let listed = self.definitions.list(record_type).await?;
            self.cache.insert(record_type.clone(), listed);
        }
        let mut definitions = self.cache.get(record_type).cloned().unwrap_or_default();
        for field in &self.overrides {
            if field.record_type != record_type.as_str() {
                continue;
            }
            match definitions
                .iter_mut()
                .find(|existing| existing.id == field.id)
            {
                Some(existing) => *existing = field.clone(),
                None => definitions.push(field.clone()),
            }
        }
        Ok(FieldSet::new(definition, definitions))
    }

    pub async fn context(&mut self, record_type: &RecordType) -> Result<FormulaContext, AppError> {
        let definition = self.registry.require(record_type.as_str())?.clone();
        let fields = self
            .field_set(record_type)
            .await?
            .all()
            .iter()
            .filter(|field| field.definition.is_active)
            .map(|field| FormulaField {
                kind: kind_of(field.definition.field_type),
                field: field.clone(),
            })
            .collect();
        let item_tables: Vec<&ItemTable> = definition
            .item_tables
            .iter()
            .filter(|table| table.is_available())
            .collect();
        let mut relations: Vec<FormulaRelation> = item_tables
            .iter()
            .map(|table| FormulaRelation {
                key: table.key.clone(),
                label: table.label.clone(),
                source: AggregateSource::Items(Box::new((*table).clone())),
                fields: table
                    .columns
                    .iter()
                    .map(|column| RelatedField {
                        key: column.column.clone(),
                        label: column.label.clone(),
                        column: AggregateColumn::ItemColumn(column.column.clone()),
                        is_formula: false,
                    })
                    .collect(),
            })
            .collect();
        for relation in &definition.relations {
            if item_tables.iter().any(|table| table.key == relation.key) {
                continue;
            }
            let Some(target) = self.registry.get(&relation.target).cloned() else {
                continue;
            };
            let source = match relation.paths.as_slice() {
                [RelationPath::HasMany { column }] => AggregateSource::HasMany {
                    target: Box::new(target.clone()),
                    column: column.clone(),
                },
                [
                    RelationPath::Relationship {
                        relationship_id,
                        inverse,
                        ..
                    },
                ] => AggregateSource::Relationship {
                    target: Box::new(target.clone()),
                    relationship_id: *relationship_id,
                    inverse: *inverse,
                },
                _ => continue,
            };
            let target_fields = self.field_set(&target.record_type).await?;
            let fields = target_fields
                .all()
                .iter()
                .filter(|field| {
                    field.definition.is_active
                        && kind_of(field.definition.field_type) == Some(ValueKind::Number)
                        && field
                            .system
                            .as_ref()
                            .is_none_or(|system| !system.is_computed())
                })
                .map(|field| RelatedField {
                    key: field.key().to_owned(),
                    label: field.definition.label.clone(),
                    column: match &field.system {
                        Some(system) => AggregateColumn::System(system.column.clone()),
                        None => AggregateColumn::Custom(field.definition.id),
                    },
                    is_formula: field.definition.is_formula(),
                })
                .collect();
            relations.push(FormulaRelation {
                key: relation.key.clone(),
                label: relation.label.clone(),
                source,
                fields,
            });
        }
        Ok(FormulaContext {
            record_type: definition,
            fields,
            relations,
        })
    }
}
