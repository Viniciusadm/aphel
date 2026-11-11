use std::collections::BTreeMap;
use std::sync::Arc;

use uuid::Uuid;

use crate::application::custom_objects::CustomSchema;
use crate::application::fields::capabilities::{EntityTraits, ManagedField};
use crate::application::fields::computed::ComputedColumn;
use crate::application::shared::RecordType;
use crate::domain::access::{Permission, is_known_permission};
use crate::domain::fields::{FieldConfig, FieldKey, FieldType, is_identifier};
use crate::errors::AppError;
use crate::records::{
    RecordHandler, RecordHandlerFactory, RecordTypeHandler, operation_not_supported,
};

pub const RECORD_TYPE_NOT_FOUND: &str = "record_type_not_found";
pub const MAX_IDENTIFIER_LENGTH: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemFieldOption {
    pub value: String,
    pub label: String,
    pub labels: super::LocalizedLabels,
    pub color: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionColumn {
    pub table: String,
    pub record_column: String,
}

impl ExtensionColumn {
    pub fn alias(&self) -> String {
        format!("ext_{}", self.table)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SystemField {
    pub key: FieldKey,
    pub writable: bool,
    pub extension: Option<ExtensionColumn>,
    pub label: String,
    pub labels: super::LocalizedLabels,
    pub field_type: FieldType,
    pub column: String,
    pub filterable: bool,
    pub sortable: bool,
    pub searchable: bool,
    pub required: bool,
    pub personal: bool,
    pub sensitive: bool,
    pub config: FieldConfig,
    pub options: Vec<SystemFieldOption>,
    pub computed: Option<ComputedColumn>,
    pub contributed_by: Option<&'static str>,
    pub owner_managed: bool,
    pub writers: Vec<String>,
}

impl SystemField {
    pub fn new(
        key: &str,
        label: impl Into<super::LocalizedLabels>,
        field_type: FieldType,
        column: impl Into<String>,
    ) -> Result<Self, AppError> {
        let labels = label.into();
        Ok(Self {
            key: FieldKey::new(key).map_err(|_| {
                AppError::Configuration(format!("invalid system field key {key:?}"))
            })?,
            label: labels.default_text().to_owned(),
            writable: true,
            extension: None,
            labels,
            field_type,
            column: column.into(),
            filterable: true,
            sortable: field_type.is_sortable(),
            searchable: false,
            required: false,
            personal: false,
            sensitive: false,
            config: FieldConfig::default(),
            options: Vec::new(),
            computed: None,
            contributed_by: None,
            owner_managed: false,
            writers: Vec::new(),
        })
    }

    pub fn owner_managed(mut self) -> Self {
        self.owner_managed = true;
        self
    }

    pub fn read_only(mut self) -> Self {
        self.writable = false;
        self
    }

    pub fn computed(mut self, computed: ComputedColumn) -> Self {
        self.computed = Some(computed);
        self.searchable = false;
        self.required = false;
        self
    }

    pub fn is_computed(&self) -> bool {
        self.computed.is_some()
    }

    pub fn is_available(&self) -> bool {
        self.contributed_by
            .is_none_or(|module| crate::modules::try_app().is_none_or(|app| app.is_enabled(module)))
    }

    pub fn stored_column(&self) -> Option<&str> {
        match self.computed {
            Some(_) => None,
            None => Some(self.column.as_str()),
        }
    }

    pub fn searchable(mut self) -> Self {
        self.searchable = true;
        self
    }

    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    pub fn personal(mut self) -> Self {
        self.personal = true;
        self
    }

    pub fn sensitive(mut self) -> Self {
        self.personal = true;
        self.sensitive = true;
        self
    }

    pub fn sortable(mut self, sortable: bool) -> Self {
        self.sortable = sortable;
        self
    }

    pub fn filterable(mut self, filterable: bool) -> Self {
        self.filterable = filterable;
        self
    }

    pub fn with_config(mut self, config: FieldConfig) -> Self {
        self.config = config;
        self
    }

    pub fn with_option(
        mut self,
        value: impl Into<String>,
        label: impl Into<super::LocalizedLabels>,
        color: Option<&str>,
    ) -> Self {
        let labels = label.into();
        self.options.push(SystemFieldOption {
            value: value.into(),
            label: labels.default_text().to_owned(),
            labels,
            color: color.map(str::to_owned),
        });
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RelationPath {
    BelongsTo {
        column: String,
    },
    HasMany {
        column: String,
    },
    Polymorphic {
        type_column: String,
        id_column: String,
    },
    PolymorphicMany {
        type_column: String,
        id_column: String,
    },
    Linked {
        table: String,
        source_type_column: Option<String>,
        source_id_column: String,
        target_type_column: Option<String>,
        target_id_column: String,
    },
    Relationship {
        relationship_id: Uuid,
        inverse: bool,
        single: bool,
    },
}

impl RelationPath {
    pub fn link_columns(inverse: bool) -> (&'static str, &'static str) {
        if inverse {
            ("to_id", "from_id")
        } else {
            ("from_id", "to_id")
        }
    }

    fn identifiers(&self) -> Vec<&str> {
        match self {
            Self::BelongsTo { column } | Self::HasMany { column } => vec![column.as_str()],
            Self::Polymorphic {
                type_column,
                id_column,
            }
            | Self::PolymorphicMany {
                type_column,
                id_column,
            } => vec![type_column.as_str(), id_column.as_str()],
            Self::Linked {
                table,
                source_type_column,
                source_id_column,
                target_type_column,
                target_id_column,
            } => [
                Some(table.as_str()),
                source_type_column.as_deref(),
                Some(source_id_column.as_str()),
                target_type_column.as_deref(),
                Some(target_id_column.as_str()),
            ]
            .into_iter()
            .flatten()
            .collect(),
            Self::Relationship { .. } => Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordRelation {
    pub key: String,
    pub label: String,
    pub labels: super::LocalizedLabels,
    pub target: RecordType,
    pub paths: Vec<RelationPath>,
}

impl RecordRelation {
    pub fn new(key: &str, label: impl Into<super::LocalizedLabels>, target: RecordType) -> Self {
        let labels = label.into();
        Self {
            key: key.to_owned(),
            label: labels.default_text().to_owned(),
            labels,
            target,
            paths: Vec::new(),
        }
    }

    pub fn path(mut self, path: RelationPath) -> Self {
        self.paths.push(path);
        self
    }

    pub fn belongs_to(
        key: &str,
        label: impl Into<super::LocalizedLabels>,
        target: RecordType,
        column: impl Into<String>,
    ) -> Self {
        Self::new(key, label, target).path(RelationPath::BelongsTo {
            column: column.into(),
        })
    }

    pub fn has_many(
        key: &str,
        label: impl Into<super::LocalizedLabels>,
        target: RecordType,
        column: impl Into<String>,
    ) -> Self {
        Self::new(key, label, target).path(RelationPath::HasMany {
            column: column.into(),
        })
    }

    pub fn single_join(&self) -> Option<SingleJoin> {
        match self.paths.as_slice() {
            [RelationPath::BelongsTo { column }] => Some(SingleJoin::Column(column.clone())),
            [
                RelationPath::Relationship {
                    relationship_id,
                    inverse,
                    single: true,
                },
            ] => Some(SingleJoin::Relationship {
                relationship_id: *relationship_id,
                inverse: *inverse,
            }),
            _ => None,
        }
    }

    pub fn is_single(&self) -> bool {
        self.single_join().is_some()
    }

    pub fn relationship(&self) -> Option<(Uuid, bool)> {
        self.paths.iter().find_map(|path| match path {
            RelationPath::Relationship {
                relationship_id,
                inverse,
                ..
            } => Some((*relationship_id, *inverse)),
            _ => None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SingleJoin {
    Column(String),
    Relationship {
        relationship_id: Uuid,
        inverse: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordPartition {
    pub column: String,
    pub value: Uuid,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberAccess {
    pub table: String,
    pub record_column: String,
    pub user_column: String,
    pub record_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomTypeLabels {
    pub singular: String,
    pub plural: String,
    pub icon: Option<String>,
    pub color: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemColumn {
    pub column: String,
    pub label: String,
    pub labels: super::LocalizedLabels,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemTable {
    pub key: String,
    pub label: String,
    pub labels: super::LocalizedLabels,
    pub table: String,
    pub parent_column: String,
    pub columns: Vec<ItemColumn>,
    pub partition: Option<(String, String)>,
    pub contributed_by: Option<&'static str>,
}

impl ItemTable {
    pub fn new(
        key: &str,
        label: impl Into<super::LocalizedLabels>,
        table: &str,
        parent_column: &str,
    ) -> Self {
        let labels = label.into();
        Self {
            key: key.to_owned(),
            label: labels.default_text().to_owned(),
            labels,
            table: table.to_owned(),
            parent_column: parent_column.to_owned(),
            columns: Vec::new(),
            partition: None,
            contributed_by: None,
        }
    }

    pub fn partition(mut self, column: &str, value: &str) -> Self {
        self.partition = Some((column.to_owned(), value.to_owned()));
        self
    }

    pub fn column(mut self, column: &str, label: impl Into<super::LocalizedLabels>) -> Self {
        let labels = label.into();
        self.columns.push(ItemColumn {
            column: column.to_owned(),
            label: labels.default_text().to_owned(),
            labels,
        });
        self
    }

    pub fn is_available(&self) -> bool {
        self.contributed_by
            .is_none_or(|module| crate::modules::try_app().is_none_or(|app| app.is_enabled(module)))
    }

    fn identifiers(&self) -> impl Iterator<Item = &str> {
        [
            self.key.as_str(),
            self.table.as_str(),
            self.parent_column.as_str(),
        ]
        .into_iter()
        .chain(self.partition.iter().map(|(column, _)| column.as_str()))
        .chain(self.columns.iter().map(|column| column.column.as_str()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordHierarchy {
    pub parent_column: String,
    pub contributed_by: Option<&'static str>,
}

impl RecordHierarchy {
    pub fn is_available(&self) -> bool {
        self.contributed_by
            .is_none_or(|module| crate::modules::try_app().is_none_or(|app| app.is_enabled(module)))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EntityDefinition {
    pub key: String,
    pub event_prefix: String,
    pub storage: super::EntityStorage,
    pub generic_writes: bool,
    pub indexes: Vec<super::EntityIndex>,
    pub references: Vec<super::SoftReference>,
    pub record_type: RecordType,
    pub table: String,
    pub owner_column: Option<String>,
    pub member_access: Option<MemberAccess>,
    pub deleted_column: Option<String>,
    pub archived_column: Option<String>,
    pub read_permission: Permission,
    pub create_permission: Option<Permission>,
    pub delete_permission: Option<Permission>,
    pub update_permission: Option<Permission>,
    pub system_fields: Vec<SystemField>,
    pub relations: Vec<RecordRelation>,
    pub item_tables: Vec<ItemTable>,
    pub hierarchy: Option<RecordHierarchy>,
    pub partition: Option<RecordPartition>,
    pub custom: Option<CustomTypeLabels>,
    pub title_field: Option<String>,
    pub capabilities: EntityTraits,
    pub managed_fields: Vec<ManagedField>,
    pub handler: Option<RecordHandler>,
}

impl EntityDefinition {
    pub fn new(
        record_type: RecordType,
        table: impl Into<String>,
        read_permission: Permission,
    ) -> Self {
        Self {
            key: record_type.as_str().to_owned(),
            event_prefix: record_type.as_str().to_owned(),
            storage: super::EntityStorage::Table,
            generic_writes: false,
            indexes: Vec::new(),
            references: Vec::new(),
            record_type,
            table: table.into(),
            owner_column: None,
            member_access: None,
            deleted_column: None,
            archived_column: None,
            read_permission,
            create_permission: None,
            delete_permission: None,
            update_permission: None,
            system_fields: Vec::new(),
            relations: Vec::new(),
            item_tables: Vec::new(),
            hierarchy: None,
            partition: None,
            custom: None,
            title_field: None,
            capabilities: EntityTraits::default(),
            managed_fields: Vec::new(),
            handler: None,
        }
    }

    pub fn events(mut self, prefix: &str) -> Self {
        self.event_prefix = prefix.to_owned();
        self
    }

    pub fn reference(mut self, reference: super::SoftReference) -> Self {
        self.references.push(reference);
        self
    }

    pub fn generic_mutations(mut self, create: Permission, delete: Permission) -> Self {
        self.generic_writes = true;
        self.create_permission = Some(create);
        self.delete_permission = Some(delete);
        self
    }

    pub fn managed(mut self, key: &str) -> Self {
        self.key = key.to_owned();
        self.storage = super::EntityStorage::Managed;
        self.generic_writes = true;
        self.table = super::managed_table(key);
        self
    }

    pub fn index(mut self, key: &str, fields: &[&str], unique: bool) -> Self {
        self.indexes.push(super::EntityIndex {
            key: key.to_owned(),
            fields: fields.iter().map(|field| (*field).to_owned()).collect(),
            unique,
        });
        self
    }

    pub fn with_trait<T: serde::Serialize>(mut self, key: &str, data: T) -> Self {
        self.capabilities.traits.insert(
            key.to_owned(),
            serde_json::to_value(data).expect("serializable entity trait"),
        );
        self
    }

    pub fn capabilities(mut self, capabilities: EntityTraits) -> Self {
        self.capabilities = capabilities;
        self
    }

    pub fn handler(mut self, handler: Arc<dyn RecordTypeHandler>) -> Self {
        self.handler = Some(RecordHandler::new(handler));
        self
    }

    pub fn managed_field_by_key(&self, key: &str) -> Option<&ManagedField> {
        self.managed_fields
            .iter()
            .find(|field| field.spec.key.as_str() == key)
    }

    pub fn title_field(mut self, key: &str) -> Self {
        self.title_field = Some(key.to_owned());
        self
    }

    pub fn title_column(&self) -> Option<&str> {
        let key = self.title_field.as_deref()?;
        self.system_field_by_key(key)
            .map(|field| field.column.as_str())
    }

    pub fn partitioned(mut self, column: impl Into<String>, value: Uuid) -> Self {
        self.partition = Some(RecordPartition {
            column: column.into(),
            value,
        });
        self
    }

    pub fn custom_labels(mut self, labels: CustomTypeLabels) -> Self {
        self.custom = Some(labels);
        self
    }

    pub fn is_custom(&self) -> bool {
        self.custom.is_some()
    }

    pub fn action_permission(&self, action: &str) -> Option<Permission> {
        let resource = self.read_permission.as_str().strip_suffix(".read")?;
        let permission = Permission::parse(format!("{resource}.{action}")).ok()?;
        (self.is_custom() || is_known_permission(&permission)).then_some(permission)
    }

    pub fn update_permission(mut self, permission: Permission) -> Self {
        self.update_permission = Some(permission);
        self
    }

    pub fn owner_column(mut self, column: impl Into<String>) -> Self {
        self.owner_column = Some(column.into());
        self
    }

    pub fn member_access(
        mut self,
        table: impl Into<String>,
        record_column: impl Into<String>,
        user_column: impl Into<String>,
    ) -> Self {
        self.member_access = Some(MemberAccess {
            table: table.into(),
            record_column: record_column.into(),
            user_column: user_column.into(),
            record_key: "id".to_owned(),
        });
        self
    }

    pub fn soft_delete_column(mut self, column: impl Into<String>) -> Self {
        self.deleted_column = Some(column.into());
        self
    }

    pub fn archive_column(mut self, column: impl Into<String>) -> Self {
        self.archived_column = Some(column.into());
        self
    }

    pub fn system_field(mut self, field: SystemField) -> Self {
        self.system_fields.push(field);
        self
    }

    pub fn system_fields(mut self, fields: Vec<SystemField>) -> Self {
        self.system_fields.extend(fields);
        self
    }

    pub fn relation(mut self, relation: RecordRelation) -> Self {
        self.relations.push(relation);
        self
    }

    pub fn hierarchy(mut self, parent_column: impl Into<String>) -> Self {
        self.hierarchy = Some(RecordHierarchy {
            parent_column: parent_column.into(),
            contributed_by: None,
        });
        self
    }

    pub fn available_hierarchy(&self) -> Option<&RecordHierarchy> {
        self.hierarchy
            .as_ref()
            .filter(|hierarchy| hierarchy.is_available())
    }

    pub fn relation_by_key(&self, key: &str) -> Option<&RecordRelation> {
        self.relations.iter().find(|relation| relation.key == key)
    }

    pub fn system_field_by_key(&self, key: &str) -> Option<&SystemField> {
        self.system_fields
            .iter()
            .find(|field| field.key.as_str() == key)
    }

    pub fn check(&self) -> Result<(), AppError> {
        self.validate()
    }

    fn validate(&self) -> Result<(), AppError> {
        let invalid = |what: &str, value: &str| {
            AppError::Configuration(format!(
                "record type {}: invalid {what} {value:?}",
                self.record_type
            ))
        };
        RecordType::parse(self.key.clone()).map_err(|_| invalid("entity key", &self.key))?;
        for field in &self.system_fields {
            if let Some(extension) = &field.extension {
                if !is_sql_identifier(&extension.table)
                    || extension.table.len() > 58
                    || !is_sql_identifier(&extension.record_column)
                    || field.column == extension.record_column
                    || field.required
                {
                    return Err(invalid("extension column", field.key.as_str()));
                }
                if self
                    .system_fields
                    .iter()
                    .any(|other| other.extension.is_none() && other.column == field.column)
                {
                    return Err(invalid("shadowed extension column", &field.column));
                }
            }
        }
        let mut reference_fields = std::collections::BTreeSet::new();
        for reference in &self.references {
            if !reference_fields.insert(&reference.field) || !is_identifier(&reference.field, 64) {
                return Err(invalid("reference field", &reference.field));
            }
            if self.storage != super::EntityStorage::Dynamic
                && self.system_field_by_key(&reference.field).is_none()
            {
                return Err(invalid("unknown reference field", &reference.field));
            }
            if reference.on_delete == super::DeletePolicy::Clear
                && self
                    .system_field_by_key(&reference.field)
                    .is_some_and(|field| field.required || !field.writable)
            {
                return Err(invalid("reference clear policy", &reference.field));
            }
            if let super::ReferenceTarget::Trait {
                trait_key,
                type_field,
            } = &reference.target
                && (!aphel_contract::is_valid_key(trait_key)
                    || !is_identifier(type_field, 64)
                    || (self.storage != super::EntityStorage::Dynamic
                        && self.system_field_by_key(type_field).is_none()))
            {
                return Err(invalid("reference trait target", trait_key));
            }
        }
        let identifiers = std::iter::once(("table", self.table.as_str()))
            .chain(
                self.owner_column
                    .as_deref()
                    .map(|column| ("owner column", column)),
            )
            .chain(self.member_access.iter().flat_map(|access| {
                [
                    ("member table", access.table.as_str()),
                    ("member record column", access.record_column.as_str()),
                    ("member user column", access.user_column.as_str()),
                    ("member record key", access.record_key.as_str()),
                ]
            }))
            .chain(
                self.deleted_column
                    .as_deref()
                    .map(|column| ("deleted column", column)),
            )
            .chain(
                self.archived_column
                    .as_deref()
                    .map(|column| ("archived column", column)),
            )
            .chain(
                self.partition
                    .as_ref()
                    .map(|partition| ("partition column", partition.column.as_str())),
            )
            .chain(
                self.system_fields
                    .iter()
                    .map(|field| ("column", field.column.as_str())),
            )
            .chain(self.system_fields.iter().flat_map(|field| {
                field
                    .computed
                    .iter()
                    .flat_map(ComputedColumn::identifiers)
                    .map(|identifier| ("computed column", identifier))
            }))
            .chain(self.relations.iter().flat_map(|relation| {
                std::iter::once(("relation key", relation.key.as_str())).chain(
                    relation
                        .paths
                        .iter()
                        .flat_map(RelationPath::identifiers)
                        .map(|identifier| ("relation column", identifier)),
                )
            }))
            .chain(
                self.hierarchy
                    .as_ref()
                    .map(|hierarchy| ("hierarchy parent column", hierarchy.parent_column.as_str())),
            )
            .chain(
                self.item_tables
                    .iter()
                    .flat_map(ItemTable::identifiers)
                    .map(|identifier| ("item table identifier", identifier)),
            );
        for (what, value) in identifiers {
            if !is_sql_identifier(value) {
                return Err(invalid(what, value));
            }
        }
        let mut relations = std::collections::BTreeSet::new();
        for relation in &self.relations {
            if !relations.insert(relation.key.as_str())
                || self.system_field_by_key(&relation.key).is_some()
                || relation.paths.is_empty()
            {
                return Err(invalid("duplicated relation", &relation.key));
            }
        }
        let mut keys = std::collections::BTreeSet::new();
        for field in &self.system_fields {
            if !keys.insert(field.key.as_str()) {
                return Err(invalid("duplicated system field", field.key.as_str()));
            }
            if field.field_type.base_type() == FieldType::MultiSelect
                && self.storage != super::EntityStorage::Managed
            {
                return Err(invalid("multi_select system field", field.key.as_str()));
            }
            if field.is_computed() && (field.searchable || field.required) {
                return Err(invalid("computed system field", field.key.as_str()));
            }
            if !field.config.validate(field.field_type).is_empty() {
                return Err(invalid("system field config", field.key.as_str()));
            }
        }
        Ok(())
    }
}

pub fn is_sql_identifier(value: &str) -> bool {
    let mut characters = value.chars();
    value.len() <= MAX_IDENTIFIER_LENGTH
        && characters
            .next()
            .is_some_and(|first| first.is_ascii_lowercase() || first == '_')
        && characters.all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
        })
}

pub fn is_option_value(value: &str) -> bool {
    is_identifier(value, MAX_IDENTIFIER_LENGTH)
}

#[derive(Debug, Clone)]
pub struct RegistryTransform {
    pub module: &'static str,
    pub key: &'static str,
    pub apply: fn(&mut EntityDefinition) -> Result<(), AppError>,
}
impl PartialEq for RegistryTransform {
    fn eq(&self, other: &Self) -> bool {
        self.module == other.module
            && self.key == other.key
            && std::ptr::fn_addr_eq(self.apply, other.apply)
    }
}
impl RegistryTransform {
    fn transform(&self, definition: &mut EntityDefinition) -> Result<(), AppError> {
        let existing: std::collections::BTreeSet<_> = definition
            .system_fields
            .iter()
            .map(|field| field.key.clone())
            .collect();
        (self.apply)(definition)?;
        for field in &mut definition.system_fields {
            if !existing.contains(&field.key) {
                field.contributed_by = Some(self.module);
            }
        }
        definition.validate()
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct RecordTypeRegistry {
    transforms: Vec<RegistryTransform>,
    types: BTreeMap<RecordType, EntityDefinition>,
    custom_schema: std::sync::Arc<CustomSchema>,
    custom_handler: Option<RecordHandlerFactory>,
}

impl RecordTypeRegistry {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn builtin() -> Self {
        crate::modules::app().record_types().as_ref().clone()
    }

    pub fn with_custom_handler(mut self, handler: Option<RecordHandlerFactory>) -> Self {
        self.custom_handler = handler;
        self
    }

    pub fn definition_mut(&mut self, record_type: &str) -> Option<&mut EntityDefinition> {
        let record_type = RecordType::parse(record_type).ok()?;
        let key = self
            .types
            .values()
            .find(|definition| definition.key == record_type.as_str())
            .map(|definition| definition.record_type.clone())
            .unwrap_or(record_type);
        self.types.get_mut(&key)
    }

    pub fn handler(&self, record_type: &str) -> Result<Arc<dyn RecordTypeHandler>, AppError> {
        let definition = self.require(record_type)?;
        if let Some(handler) = &definition.handler {
            return Ok(handler.get().clone());
        }
        if definition.storage != super::EntityStorage::Dynamic {
            return Ok(Arc::new(crate::records::GenericRecordHandler::new(
                definition.record_type.clone(),
            )));
        }
        self.custom_handler
            .as_ref()
            .map(|factory| factory.handler(&definition.record_type).get().clone())
            .ok_or_else(|| operation_not_supported(record_type, "dynamic writes"))
    }

    pub fn with_trait(&self, key: &str) -> impl Iterator<Item = &EntityDefinition> {
        self.types
            .values()
            .filter(move |definition| definition.capabilities.has_trait(key))
    }

    pub fn capabilities(&self, record_type: &str) -> Option<&EntityTraits> {
        RecordType::parse(record_type)
            .ok()
            .and_then(|record_type| self.get(&record_type))
            .map(|definition| &definition.capabilities)
    }

    pub fn register(mut self, mut definition: EntityDefinition) -> Result<Self, AppError> {
        for transform in &self.transforms {
            transform.transform(&mut definition)?;
        }
        definition.validate()?;
        if self.types.contains_key(&definition.record_type)
            || self
                .types
                .values()
                .any(|existing| existing.key == definition.key)
        {
            return Err(AppError::Configuration(format!(
                "record type {} is already registered",
                definition.record_type
            )));
        }
        self.types
            .insert(definition.record_type.clone(), definition);
        Ok(self)
    }

    pub fn with_transform(mut self, transform: RegistryTransform) -> Result<Self, AppError> {
        if self
            .transforms
            .iter()
            .any(|existing| existing.module == transform.module && existing.key == transform.key)
        {
            return Err(AppError::Configuration(format!(
                "duplicate entity transform {}:{}",
                transform.module, transform.key
            )));
        }
        for definition in self.types.values_mut() {
            transform.transform(definition)?;
        }
        self.transforms.push(transform);
        Ok(self)
    }

    pub fn validate_relations(self) -> Result<Self, AppError> {
        for definition in self.types.values() {
            for reference in &definition.references {
                if let super::ReferenceTarget::Entity { entity } = &reference.target {
                    self.require(entity).map_err(|_| {
                        AppError::Configuration(format!(
                            "entity {} references unknown entity {entity}",
                            definition.key
                        ))
                    })?;
                }
            }
            for relation in &definition.relations {
                if self.get(&relation.target).is_none() {
                    return Err(AppError::Configuration(format!(
                        "record type {}: relation {} targets unknown record type {}",
                        definition.record_type, relation.key, relation.target
                    )));
                }
            }
        }
        Ok(self)
    }

    pub fn add_relation(
        &mut self,
        record_type: &RecordType,
        relation: RecordRelation,
    ) -> Result<(), AppError> {
        let Some(definition) = self.types.get_mut(record_type) else {
            return Err(AppError::Configuration(format!(
                "relation {} on unknown record type {record_type}",
                relation.key
            )));
        };
        if definition.relation_by_key(&relation.key).is_some()
            || definition.system_field_by_key(&relation.key).is_some()
            || !is_sql_identifier(&relation.key)
        {
            return Err(AppError::Configuration(format!(
                "record type {record_type}: duplicated relation {}",
                relation.key
            )));
        }
        definition.relations.push(relation);
        Ok(())
    }

    pub fn get(&self, record_type: &RecordType) -> Option<&EntityDefinition> {
        self.types.get(record_type).or_else(|| {
            self.types
                .values()
                .find(|definition| definition.key == record_type.as_str())
        })
    }

    pub fn require(&self, record_type: &str) -> Result<&EntityDefinition, AppError> {
        RecordType::parse(record_type)
            .ok()
            .and_then(|record_type| self.get(&record_type))
            .ok_or_else(|| {
                AppError::not_found_code(
                    RECORD_TYPE_NOT_FOUND,
                    format!("record type {record_type:?} is not registered"),
                )
            })
    }

    pub fn with_custom_schema(mut self, schema: std::sync::Arc<CustomSchema>) -> Self {
        self.custom_schema = schema;
        self
    }

    pub fn custom_schema(&self) -> &CustomSchema {
        &self.custom_schema
    }

    pub fn definitions(&self) -> impl Iterator<Item = &EntityDefinition> {
        self.types.values()
    }
}
