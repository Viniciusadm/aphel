use crate::domain::{DuplicateFamily, DuplicateRule};
use aphel_kernel::application::fields::{
    LabelAlias, RecordTypeDefinition, RecordTypeRegistry, SystemField,
};
use aphel_kernel::application::shared::RecordType;
use aphel_kernel::domain::access::Permission;
use aphel_kernel::errors::{AppError, FieldViolation};

pub(crate) const DUPLICATE_RECORD_TYPE_MESSAGE: &str =
    "is not a record type with duplicate detection";
pub(crate) const RECORD_NOT_FOUND: &str = "record_not_found";

const UNMERGEABLE_KEYS: &[&str] = &["id", "created_by", "created_at", "updated_at"];

#[derive(Debug, Clone)]
pub(crate) struct DuplicateType {
    pub(crate) definition: RecordTypeDefinition,
    pub(crate) family: DuplicateFamily,
    pub(crate) read: Permission,
    pub(crate) create: Permission,
    pub(crate) update: Permission,
    pub(crate) delete: Permission,
}

impl DuplicateType {
    pub(crate) fn from_definition(definition: &RecordTypeDefinition) -> Option<Self> {
        definition.title_column()?;
        definition.deleted_column.as_ref()?;
        let family = DuplicateFamily::of(definition.record_type.as_str());
        if family == DuplicateFamily::Generic
            && definition.capabilities.label_aliases.is_empty()
            && !definition.is_custom()
        {
            return None;
        }
        Some(Self {
            read: definition.read_permission.clone(),
            create: definition.action_permission("create")?,
            update: definition.action_permission("update")?,
            delete: definition.action_permission("delete")?,
            definition: definition.clone(),
            family,
        })
    }

    pub(crate) fn record_type(&self) -> &RecordType {
        &self.definition.record_type
    }

    pub(crate) fn as_str(&self) -> &str {
        self.definition.record_type.as_str()
    }

    pub(crate) fn table(&self) -> &str {
        &self.definition.table
    }

    pub(crate) fn title_column(&self) -> &str {
        self.definition.title_column().unwrap_or("id")
    }

    pub(crate) fn deleted_column(&self) -> &str {
        self.definition
            .deleted_column
            .as_deref()
            .unwrap_or("deleted_at")
    }

    pub(crate) fn identifiers(&self) -> &[LabelAlias] {
        match self.family {
            DuplicateFamily::Generic => &self.definition.capabilities.label_aliases,
            _ => &[],
        }
    }

    pub(crate) fn rules(&self) -> Vec<DuplicateRule> {
        self.family.rules(!self.identifiers().is_empty())
    }

    pub(crate) fn supports(&self, rule: DuplicateRule) -> bool {
        self.rules().contains(&rule)
    }

    pub(crate) fn summary_columns(&self) -> Vec<String> {
        let capabilities = &self.definition.capabilities;
        if !capabilities.contact_columns.is_empty() {
            return capabilities.contact_columns.clone();
        }
        capabilities
            .label_aliases
            .iter()
            .map(|alias| alias.column.clone())
            .collect()
    }

    pub(crate) fn field_for_column(&self, column: &str) -> Option<&SystemField> {
        self.definition
            .system_fields
            .iter()
            .find(|field| field.column == column && !field.is_computed())
    }

    pub(crate) fn generic_merge_fields(&self) -> Vec<&SystemField> {
        let definition = &self.definition;
        let reserved = [
            definition.deleted_column.as_deref(),
            definition.archived_column.as_deref(),
            definition
                .partition
                .as_ref()
                .map(|partition| partition.column.as_str()),
        ];
        definition
            .system_fields
            .iter()
            .filter(|field| !field.is_computed() && field.contributed_by.is_none())
            .filter(|field| !UNMERGEABLE_KEYS.contains(&field.key.as_str()))
            .filter(|field| !reserved.contains(&Some(field.column.as_str())))
            .filter(|field| {
                !definition
                    .capabilities
                    .locked_fields
                    .iter()
                    .any(|locked| locked == field.key.as_str())
            })
            .collect()
    }
}

pub(crate) fn duplicate_type(registry: &RecordTypeRegistry, value: &str) -> Option<DuplicateType> {
    let record_type = RecordType::parse(value.trim()).ok()?;
    registry
        .get(&record_type)
        .and_then(DuplicateType::from_definition)
}

pub(crate) fn duplicate_types(registry: &RecordTypeRegistry) -> Vec<DuplicateType> {
    let mut types: Vec<DuplicateType> = registry
        .definitions()
        .filter_map(DuplicateType::from_definition)
        .collect();
    types.sort_by_key(|kind| {
        (
            match kind.family {
                DuplicateFamily::Contact => 0,
                DuplicateFamily::Company => 1,
                DuplicateFamily::Generic => 2,
            },
            kind.as_str().to_owned(),
        )
    });
    types
}

pub(crate) fn invalid_duplicate_type() -> AppError {
    AppError::invalid_fields(
        "invalid record type",
        vec![FieldViolation::new(
            "record_type",
            DUPLICATE_RECORD_TYPE_MESSAGE,
        )],
    )
}

pub(crate) fn parse_duplicate_type(
    registry: &RecordTypeRegistry,
    value: &str,
) -> Result<DuplicateType, AppError> {
    duplicate_type(registry, value).ok_or_else(invalid_duplicate_type)
}

pub(crate) fn record_not_found() -> AppError {
    AppError::not_found_code(RECORD_NOT_FOUND, "the record was not found")
}
