use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::configuration::{ConfigPackage, parse_section};
use aphel_kernel::application::custom_objects::custom_record_definition;
use aphel_kernel::application::fields::RecordTypeRegistry;
use aphel_kernel::application::shared::RecordType;
use aphel_kernel::domain::custom_objects::{
    CustomObject, CustomObjectData, DEFAULT_TITLE_FIELD_KEY, OnDelete, is_object_key,
    object_record_type,
};

pub const CUSTOM_OBJECTS_SECTION: &str = "custom_objects";
pub const MAX_PACKAGE_CUSTOM_OBJECTS: usize = 50;
pub const MAX_PACKAGE_RELATIONSHIPS: usize = 200;

fn default_true() -> bool {
    true
}

fn is_true(value: &bool) -> bool {
    *value
}

fn is_false(value: &bool) -> bool {
    !*value
}

fn default_title_key() -> String {
    DEFAULT_TITLE_FIELD_KEY.to_owned()
}

fn default_on_delete() -> String {
    OnDelete::Restrict.as_str().to_owned()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageCustomObject {
    pub key: String,
    pub label_singular: String,
    pub label_plural: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub owner_enabled: bool,
    #[serde(default = "default_title_key")]
    pub title_field_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageRelationship {
    pub key: String,
    pub inverse_key: String,
    pub from_type: String,
    pub to_type: String,
    pub cardinality: String,
    pub from_label: String,
    pub to_label: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub required: bool,
    #[serde(default = "default_on_delete")]
    pub on_delete: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageCustomObjects {
    #[serde(default)]
    pub objects: Vec<PackageCustomObject>,
    #[serde(default)]
    pub relationships: Vec<PackageRelationship>,
}

pub fn package_custom_objects(package: &ConfigPackage) -> PackageCustomObjects {
    package
        .sections
        .get(CUSTOM_OBJECTS_SECTION)
        .and_then(|content| parse_section(content).ok())
        .unwrap_or_default()
}

fn placeholder_object(entry: &PackageCustomObject) -> CustomObject {
    CustomObject {
        id: Uuid::nil(),
        key: entry.key.clone(),
        data: CustomObjectData {
            label_singular: entry.label_singular.clone(),
            label_plural: entry.label_plural.clone(),
            description: entry.description.clone(),
            icon: entry.icon.clone(),
            color: entry.color.clone(),
            title_field_key: DEFAULT_TITLE_FIELD_KEY.to_owned(),
            owner_enabled: entry.owner_enabled,
            is_active: true,
        },
        position: 0,
        module_key: None,
        entity_definition: None,
        is_internal: false,
        created_by: None,
        created_at: DateTime::<Utc>::UNIX_EPOCH,
        updated_at: DateTime::<Utc>::UNIX_EPOCH,
    }
}

pub fn package_registry(base: &RecordTypeRegistry, package: &ConfigPackage) -> RecordTypeRegistry {
    let mut registry = base.clone();
    for entry in package_custom_objects(package).objects {
        if !is_object_key(&entry.key) {
            continue;
        }
        let Ok(record_type) = RecordType::parse(object_record_type(&entry.key)) else {
            continue;
        };
        if registry.get(&record_type).is_some() {
            continue;
        }
        if let Ok(extended) = custom_record_definition(&placeholder_object(&entry))
            .and_then(|definition| registry.clone().register(definition))
        {
            registry = extended;
        }
    }
    registry
}
