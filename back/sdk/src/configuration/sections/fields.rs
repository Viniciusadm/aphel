use crate::configuration::{ConfigPackage, PackageIssue, parse_section};
use aphel_kernel::application::fields::{
    EntityDefinition, FieldSet, MAX_OPTIONS, bounded_label, config_from_json, config_to_json,
    is_option_value, normalize_color,
};
use aphel_kernel::domain::fields::{
    FORMULA_FIELD_TYPE, FieldDefinition, FieldKey, FieldOption, FieldType, resolve_declared_type,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;
use uuid::Uuid;
pub const FIELDS_SECTION: &str = "fields";

pub const FIELD_TYPE_MISMATCH: &str = "field_type_mismatch";
pub const FIELD_DUPLICATE_VALUES: &str = "field_duplicate_values";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageField {
    pub record_type: String,
    pub key: String,
    pub label: String,
    #[serde(rename = "type")]
    pub field_type: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub required: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub unique: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub searchable: bool,
    #[serde(default, skip_serializing_if = "is_empty_config")]
    pub config: Value,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<PackageOption>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageOption {
    pub value: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

fn is_false(value: &bool) -> bool {
    !*value
}

fn is_empty_config(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::Object(object) => object.is_empty(),
        _ => false,
    }
}

pub fn parse_fields(content: &Value) -> Result<Vec<PackageField>, PackageIssue> {
    parse_section(content)
}

pub fn package_fields(package: &ConfigPackage) -> Vec<PackageField> {
    package
        .section(FIELDS_SECTION)
        .and_then(|content| parse_fields(content).ok())
        .unwrap_or_default()
}

pub fn field_definition_from_package(
    field: &PackageField,
    id: Uuid,
    position: i32,
) -> Result<FieldDefinition, Vec<PackageIssue>> {
    let mut issues = Vec::new();
    let key = FieldKey::new(field.key.trim()).ok();
    if key.is_none() {
        issues.push(PackageIssue::new(
            "key",
            "invalid_key",
            "must be snake_case: lowercase letters, digits and single underscores, starting with a letter (max 64)",
        ));
    }
    let label = bounded_label(&field.label);
    if label.is_none() {
        issues.push(PackageIssue::new(
            "label",
            "invalid_label",
            "must have between 1 and 140 characters",
        ));
    }
    let config = match config_from_json(&field.config) {
        Ok(config) => Some(config),
        Err(errors) => {
            issues.extend(errors.into_iter().map(|error| {
                PackageIssue::new(
                    format!("config.{}", error.property),
                    "invalid_config",
                    error.message,
                )
            }));
            None
        }
    };
    let declared = field.field_type.trim();
    let formula = config.as_ref().and_then(|config| config.formula.as_ref());
    let field_type = resolve_declared_type(declared, formula);
    if field_type.is_none() {
        if declared == FORMULA_FIELD_TYPE {
            if config.is_some() {
                issues.push(PackageIssue::new(
                    "config.expression",
                    "invalid_config",
                    "is required for formula fields",
                ));
            }
        } else {
            issues.push(PackageIssue::new(
                "type",
                "invalid_field_type",
                "is not a known field type",
            ));
        }
    } else if formula.is_some() && declared != FORMULA_FIELD_TYPE {
        issues.push(PackageIssue::new(
            "config.expression",
            "invalid_config",
            "is only accepted by formula fields",
        ));
    }
    let options = package_options(field, field_type, &mut issues);
    match (key, label, field_type, config) {
        (Some(key), Some(label), Some(field_type), Some(config)) if issues.is_empty() => {
            Ok(FieldDefinition {
                id,
                record_type: field.record_type.clone(),
                key,
                label,
                field_type,
                is_system: false,
                column_name: None,
                is_required: field.required,
                is_unique: field.unique,
                is_searchable: field.searchable,
                is_personal: false,
                is_sensitive: false,
                module_key: None,
                is_active: true,
                position,
                config,
                options,
            })
        }
        _ => Err(issues),
    }
}

fn package_options(
    field: &PackageField,
    field_type: Option<FieldType>,
    issues: &mut Vec<PackageIssue>,
) -> Vec<FieldOption> {
    if field_type.is_some_and(|field_type| !field_type.has_options()) {
        if !field.options.is_empty() {
            issues.push(PackageIssue::new(
                "options",
                "options_not_supported",
                "are only accepted by select and multi_select fields",
            ));
        }
        return Vec::new();
    }
    if field.options.len() > MAX_OPTIONS {
        issues.push(PackageIssue::new(
            "options",
            "too_many_options",
            format!("a field can have at most {MAX_OPTIONS} options"),
        ));
        return Vec::new();
    }
    let mut seen = BTreeSet::new();
    let mut options = Vec::new();
    for (index, option) in field.options.iter().enumerate() {
        let path = format!("options.{index}");
        let value = option.value.trim();
        if !is_option_value(value) {
            issues.push(PackageIssue::new(
                format!("{path}.value"),
                "invalid_option_value",
                "must be snake_case: lowercase letters, digits and single underscores, starting with a letter (max 64)",
            ));
        } else if !seen.insert(value.to_owned()) {
            issues.push(PackageIssue::new(
                format!("{path}.value"),
                "duplicated_option",
                "is duplicated",
            ));
        }
        let label = bounded_label(&option.label);
        if label.is_none() {
            issues.push(PackageIssue::new(
                format!("{path}.label"),
                "invalid_label",
                "must have between 1 and 140 characters",
            ));
        }
        let color = match option.color.as_deref() {
            None => None,
            Some(color) => {
                let normalized = normalize_color(color);
                if normalized.is_none() {
                    issues.push(PackageIssue::new(
                        format!("{path}.color"),
                        "invalid_color",
                        "must be a hex color such as #1f6feb",
                    ));
                }
                normalized
            }
        };
        options.push(FieldOption {
            id: Uuid::now_v7(),
            value: value.to_owned(),
            label: label.unwrap_or_default(),
            color,
            position: index as i32,
            is_active: true,
        });
    }
    options
}

pub fn export_field(field: &FieldDefinition) -> PackageField {
    let mut options: Vec<&FieldOption> = field
        .options
        .iter()
        .filter(|option| option.is_active)
        .collect();
    options.sort_by_key(|option| (option.position, option.id));
    PackageField {
        record_type: field.record_type.clone(),
        key: field.key.as_str().to_owned(),
        label: field.label.clone(),
        field_type: field.type_name().to_owned(),
        required: field.is_required,
        unique: field.is_unique,
        searchable: field.is_searchable,
        config: config_to_json(&field.config),
        options: options
            .into_iter()
            .map(|option| PackageOption {
                value: option.value.clone(),
                label: option.label.clone(),
                color: option.color.clone(),
            })
            .collect(),
    }
}

pub fn projected_field_set(
    definition: &EntityDefinition,
    existing: Vec<FieldDefinition>,
    package: &[PackageField],
) -> FieldSet {
    let mut definitions = existing;
    let mut position = definitions
        .iter()
        .map(|field| field.position)
        .max()
        .unwrap_or(0);
    for field in package
        .iter()
        .filter(|field| field.record_type == definition.record_type.as_str())
    {
        position += 1;
        let Ok(projected) = field_definition_from_package(field, Uuid::now_v7(), position) else {
            continue;
        };
        match definitions
            .iter_mut()
            .find(|existing| existing.key == projected.key)
        {
            Some(existing) if existing.is_system => {}
            Some(existing) => {
                existing.is_active = true;
                existing.is_required = projected.is_required;
                for option in projected.options {
                    if !existing
                        .options
                        .iter()
                        .any(|current| current.value == option.value)
                    {
                        existing.options.push(option);
                    }
                }
                for option in existing.options.iter_mut() {
                    option.is_active = option.is_active
                        || field
                            .options
                            .iter()
                            .any(|package| package.value == option.value);
                }
            }
            None => definitions.push(projected),
        }
    }
    FieldSet::new(definition, definitions)
}
