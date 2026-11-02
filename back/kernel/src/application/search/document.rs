use std::collections::HashMap;

use uuid::Uuid;

use crate::application::fields::{EntityDefinition, FieldSet, ResolvedField};
use crate::application::search::contents::is_content_key;
use crate::application::search::text::{
    MAX_VALUE_LENGTH, normalize_content, phone_content, truncate,
};
use crate::application::shared::RecordRef;
use crate::domain::fields::FieldType;

pub const TITLE_WEIGHT: u8 = 3;
pub const SYSTEM_WEIGHT: u8 = 2;
pub const CUSTOM_WEIGHT: u8 = 1;
pub const MAX_TITLE_LENGTH: usize = 255;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexedField {
    pub key: String,
    pub field_id: Option<Uuid>,
    pub column: Option<String>,
    pub field_type: FieldType,
    pub position: i32,
    pub title_candidate: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexEntry {
    pub field_key: String,
    pub position: i32,
    pub weight: u8,
    pub title_candidate: bool,
    pub value: String,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Display {
    pub title: Option<String>,
    pub title_field: Option<String>,
    pub subtitle: Option<String>,
    pub subtitle_field: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexDocument {
    pub record: RecordRef,
    pub owner_id: Option<String>,
    pub display: Display,
    pub entries: Vec<IndexEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceRow {
    pub id: Uuid,
    pub owner_id: Option<String>,
    pub removed: bool,
    pub values: HashMap<String, String>,
}

pub fn is_text_like(field_type: FieldType) -> bool {
    matches!(
        field_type,
        FieldType::Text
            | FieldType::LongText
            | FieldType::Email
            | FieldType::Phone
            | FieldType::Url
    )
}

fn is_indexable(field: &ResolvedField) -> bool {
    let searchable = match &field.system {
        Some(system) => system.searchable && !system.sensitive,
        None => field.definition.is_searchable,
    };
    searchable
        && field.definition.is_active
        && !field.definition.is_sensitive
        && is_text_like(field.definition.field_type)
}

pub fn indexed_fields(fields: &FieldSet) -> Vec<IndexedField> {
    let definition = &fields.record_type;
    let mut indexed: Vec<IndexedField> = Vec::new();
    for (position, system) in definition.system_fields.iter().enumerate() {
        let Some(field) = fields.find(system.key.as_str()) else {
            continue;
        };
        if !field.is_system() || !is_indexable(field) {
            continue;
        }
        indexed.push(IndexedField {
            key: system.key.as_str().to_owned(),
            field_id: None,
            column: Some(system.column.clone()),
            field_type: system.field_type,
            position: i32::try_from(position).unwrap_or(i32::MAX),
            title_candidate: system.field_type.base_type() == FieldType::Text,
        });
    }
    let offset = i32::try_from(definition.system_fields.len()).unwrap_or(i32::MAX);
    for field in fields.all() {
        if field.is_system() || !is_indexable(field) {
            continue;
        }
        indexed.push(IndexedField {
            key: field.key().to_owned(),
            field_id: Some(field.definition.id),
            column: None,
            field_type: field.definition.field_type,
            position: offset.saturating_add(field.definition.position.max(0)),
            title_candidate: false,
        });
    }
    indexed
}

pub fn system_columns(fields: &[IndexedField]) -> Vec<(String, String)> {
    fields
        .iter()
        .filter_map(|field| {
            field
                .column
                .as_ref()
                .map(|column| (field.key.clone(), column.clone()))
        })
        .collect()
}

fn entry(field: &IndexedField, raw: &str) -> Option<IndexEntry> {
    let value = truncate(raw.trim(), MAX_VALUE_LENGTH);
    if value.is_empty() {
        return None;
    }
    let content = match field.field_type.base_type() {
        FieldType::Phone => {
            let digits = phone_content(&value);
            if digits.is_empty() {
                normalize_content(&value)
            } else {
                digits
            }
        }
        _ => normalize_content(&value),
    };
    if content.is_empty() {
        return None;
    }
    let weight = if field.title_candidate {
        TITLE_WEIGHT
    } else if field.column.is_some() {
        SYSTEM_WEIGHT
    } else {
        CUSTOM_WEIGHT
    };
    Some(IndexEntry {
        field_key: field.key.clone(),
        position: field.position,
        weight,
        title_candidate: field.title_candidate,
        value,
        content,
    })
}

pub fn build_document(
    definition: &EntityDefinition,
    fields: &[IndexedField],
    row: &SourceRow,
    custom: &HashMap<Uuid, String>,
    contents: &[(String, String)],
) -> IndexDocument {
    let mut entries: Vec<IndexEntry> = fields
        .iter()
        .filter_map(|field| {
            let raw = match field.field_id {
                Some(field_id) => custom.get(&field_id),
                None => row.values.get(&field.key),
            }?;
            entry(field, raw)
        })
        .collect();
    let offset = i32::MAX - i32::try_from(contents.len()).unwrap_or(i32::MAX);
    for (index, (key, text)) in contents.iter().enumerate() {
        if let Some(entry) = content_entry(key, text, offset + index as i32) {
            entries.push(entry);
        }
    }
    entries.sort_by_key(|entry| entry.position);
    IndexDocument {
        record: RecordRef::new(definition.record_type.clone(), row.id),
        owner_id: row.owner_id.clone(),
        display: compose_display(&entries),
        entries,
    }
}

fn content_entry(key: &str, text: &str, position: i32) -> Option<IndexEntry> {
    let content = normalize_content(text);
    if content.is_empty() {
        return None;
    }
    Some(IndexEntry {
        field_key: key.to_owned(),
        position,
        weight: CUSTOM_WEIGHT,
        title_candidate: false,
        value: truncate(text.trim(), MAX_VALUE_LENGTH),
        content,
    })
}

pub fn compose_display(entries: &[IndexEntry]) -> Display {
    let entries: Vec<&IndexEntry> = entries
        .iter()
        .filter(|entry| !is_content_key(&entry.field_key))
        .collect();
    let title = entries
        .iter()
        .find(|entry| entry.title_candidate)
        .or_else(|| entries.first());
    let Some(title) = title else {
        return Display::default();
    };
    let folded_title = title.value.to_lowercase();
    let subtitle = entries.iter().find(|entry| {
        entry.field_key != title.field_key && !folded_title.contains(&entry.value.to_lowercase())
    });
    Display {
        title: Some(truncate(&title.value, MAX_TITLE_LENGTH)),
        title_field: Some(title.field_key.clone()),
        subtitle: subtitle.map(|entry| truncate(&entry.value, MAX_TITLE_LENGTH)),
        subtitle_field: subtitle.map(|entry| entry.field_key.clone()),
    }
}
