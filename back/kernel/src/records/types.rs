use std::sync::Arc;

use crate::application::fields::{
    EntityDefinition, FieldSpec, ItemTable, ManagedField, MemberAccess, RecordHierarchy,
    RecordRelation, RecordTypeRegistry, SystemField,
};
use crate::application::shared::RecordType;
use crate::errors::AppError;
use crate::modules::Contribution;
use crate::records::handler::{RecordHandler, RecordHandlerFactory, RecordTypeHandler};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordContributionInfo {
    pub module: &'static str,
    pub registers: Option<RecordType>,
    pub description: String,
}

pub struct RecordTypeExtension {
    module: &'static str,
    record_type: String,
    system_fields: Vec<SystemField>,
    side_table: Option<crate::application::fields::ExtensionColumn>,
    relations: Vec<RecordRelation>,
    item_tables: Vec<ItemTable>,
    hierarchy: Option<String>,
    member_access: Option<MemberAccess>,
    managed_fields: Vec<FieldSpec>,
    handler: Option<RecordHandler>,
}

impl RecordTypeExtension {
    pub fn new(module: &'static str, record_type: &str) -> Self {
        Self {
            module,
            record_type: record_type.to_owned(),
            system_fields: Vec::new(),
            side_table: None,
            relations: Vec::new(),
            item_tables: Vec::new(),
            hierarchy: None,
            member_access: None,
            managed_fields: Vec::new(),
            handler: None,
        }
    }

    pub fn record_type(&self) -> &str {
        &self.record_type
    }

    pub fn side_table(&mut self, table: &str, record_column: &str) -> &mut Self {
        self.side_table = Some(crate::application::fields::ExtensionColumn {
            table: table.to_owned(),
            record_column: record_column.to_owned(),
        });
        self
    }

    pub fn system_field(&mut self, field: SystemField) -> &mut Self {
        self.system_fields.push(field);
        self
    }

    pub fn computed(&mut self, field: SystemField) -> Result<&mut Self, AppError> {
        if !field.is_computed() {
            return Err(AppError::Configuration(format!(
                "field {} of {} is not computed",
                field.key.as_str(),
                self.record_type
            )));
        }
        Ok(self.system_field(field))
    }

    pub fn relation(&mut self, relation: RecordRelation) -> &mut Self {
        self.relations.push(relation);
        self
    }

    pub fn item_table(&mut self, table: ItemTable) -> &mut Self {
        self.item_tables.push(table);
        self
    }

    pub fn hierarchy(&mut self, parent_column: &str) -> &mut Self {
        self.hierarchy = Some(parent_column.to_owned());
        self
    }

    pub fn member_access(
        &mut self,
        table: &str,
        record_column: &str,
        user_column: &str,
    ) -> &mut Self {
        self.member_access = Some(MemberAccess {
            table: table.to_owned(),
            record_column: record_column.to_owned(),
            user_column: user_column.to_owned(),
            record_key: "id".to_owned(),
        });
        self
    }

    pub fn managed_field(&mut self, spec: FieldSpec) -> &mut Self {
        self.managed_fields.push(spec);
        self
    }

    pub fn handler(&mut self, handler: Arc<dyn RecordTypeHandler>) -> &mut Self {
        self.handler = Some(RecordHandler::new(handler));
        self
    }

    fn describe(&self) -> String {
        let mut parts = Vec::new();
        if !self.system_fields.is_empty() {
            parts.push(format!("{} system fields", self.system_fields.len()));
        }
        if !self.relations.is_empty() {
            parts.push(format!("{} relations", self.relations.len()));
        }
        if let Some(column) = &self.hierarchy {
            parts.push(format!("hierarchy by {column}"));
        }
        for table in &self.item_tables {
            parts.push(format!("item table {}", table.key));
        }
        if self.member_access.is_some() {
            parts.push("member access".to_owned());
        }
        for spec in &self.managed_fields {
            parts.push(format!("managed field {}", spec.key.as_str()));
        }
        if self.handler.is_some() {
            parts.push("handler".to_owned());
        }
        format!("extends {}: {}", self.record_type, parts.join(", "))
    }

    fn apply(self, registry: &mut RecordTypeRegistry) -> Result<(), AppError> {
        let module = self.module;
        if self
            .side_table
            .as_ref()
            .is_some_and(|table| !table.table.starts_with(&format!("{module}_")))
        {
            return Err(AppError::Configuration(
                "extension tables must belong to the extender".to_owned(),
            ));
        }
        let side_table = self.side_table;
        let record_type = self.record_type;
        let unknown = || {
            AppError::Configuration(format!(
                "module {module} extends the unknown record type {record_type}"
            ))
        };
        let definition_type = registry
            .definition_mut(&record_type)
            .ok_or_else(unknown)?
            .record_type
            .clone();
        for relation in self.relations {
            registry.add_relation(&definition_type, relation)?;
        }
        let definition = registry.definition_mut(&record_type).ok_or_else(unknown)?;
        definition
            .system_fields
            .extend(self.system_fields.into_iter().map(|mut field| {
                field.contributed_by = Some(module);
                if !field.is_computed() {
                    field.extension = side_table.clone();
                }
                field
            }));
        definition
            .item_tables
            .extend(self.item_tables.into_iter().map(|mut table| {
                table.contributed_by = Some(module);
                table
            }));
        if let Some(parent_column) = self.hierarchy {
            definition.hierarchy = Some(RecordHierarchy {
                parent_column,
                contributed_by: Some(module),
            });
        }
        if let Some(access) = self.member_access {
            definition.member_access = Some(access);
        }
        for spec in self.managed_fields {
            if definition.system_field_by_key(spec.key.as_str()).is_some()
                || definition.managed_field_by_key(spec.key.as_str()).is_some()
            {
                return Err(AppError::Configuration(format!(
                    "module {module} manages the field {} of {record_type}, which already exists",
                    spec.key.as_str()
                )));
            }
            definition
                .managed_fields
                .push(ManagedField { module, spec });
        }
        if let Some(handler) = self.handler {
            definition.handler = Some(handler);
        }
        definition.check()
    }
}

pub fn describe_records(
    definitions: &[Contribution<EntityDefinition>],
    extensions: &[Contribution<RecordTypeExtension>],
) -> Vec<RecordContributionInfo> {
    definitions
        .iter()
        .map(|contribution| RecordContributionInfo {
            module: contribution.module,
            registers: Some(contribution.item.record_type.clone()),
            description: format!(
                "registers {}{}",
                contribution.item.record_type,
                if contribution.item.handler.is_some() {
                    " (handler)"
                } else {
                    ""
                }
            ),
        })
        .chain(
            extensions
                .iter()
                .map(|contribution| RecordContributionInfo {
                    module: contribution.module,
                    registers: None,
                    description: contribution.item.describe(),
                }),
        )
        .collect()
}

pub fn assemble_registry(
    definitions: Vec<Contribution<EntityDefinition>>,
    extensions: Vec<Contribution<RecordTypeExtension>>,
    transforms: Vec<crate::application::fields::RegistryTransform>,
    custom_handler: Option<RecordHandlerFactory>,
) -> Result<RecordTypeRegistry, AppError> {
    let mut registry = RecordTypeRegistry::empty();
    for contribution in definitions {
        registry = registry.register(contribution.item).map_err(|error| {
            AppError::Configuration(format!(
                "module {} registers an invalid record type: {error}",
                contribution.module
            ))
        })?;
    }
    for contribution in extensions {
        contribution.item.apply(&mut registry)?;
    }
    for transform in transforms {
        registry = registry.with_transform(transform)?;
    }
    Ok(registry
        .validate_relations()?
        .with_custom_handler(custom_handler))
}
