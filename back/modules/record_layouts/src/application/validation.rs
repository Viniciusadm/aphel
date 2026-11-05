use std::collections::BTreeSet;

use crate::application::{FormLayout, LayoutContext, LayoutField};
use aphel_kernel::errors::FieldViolation;
use aphel_sdk::platform::application::fields::FieldSet;
use aphel_sdk::platform::application::query::{Filter, ResolveContext, validate_view_query};

pub const MAX_LAYOUT_TABS: usize = 10;
pub const MAX_LAYOUT_SECTIONS: usize = 20;
pub const MAX_LAYOUT_COLUMNS: usize = 3;
pub const MAX_LAYOUT_FIELDS: usize = 300;
pub const MAX_LAYOUT_LABEL_LENGTH: usize = 80;
pub const MAX_LAYOUT_ID_LENGTH: usize = 64;

pub fn validate_layout(
    layout: &FormLayout,
    fields: &FieldSet,
    context: LayoutContext,
    resolve: &ResolveContext,
) -> Vec<FieldViolation> {
    let mut violations = Vec::new();
    let mut push =
        |path: String, message: String| violations.push(FieldViolation::new(path, message));
    if layout.tabs.is_empty() || layout.tabs.len() > MAX_LAYOUT_TABS {
        push(
            "layout.tabs".to_owned(),
            format!("must have between 1 and {MAX_LAYOUT_TABS} tabs"),
        );
    }
    let mut tab_ids = BTreeSet::new();
    let mut section_ids = BTreeSet::new();
    let mut keys = BTreeSet::new();
    let mut placed: Vec<(String, &LayoutField)> = Vec::new();
    for (tab_index, tab) in layout.tabs.iter().enumerate() {
        let tab_path = format!("layout.tabs.{tab_index}");
        check_id(&tab.id, &format!("{tab_path}.id"), &mut tab_ids, &mut push);
        check_label(&tab.label, &format!("{tab_path}.label"), &mut push);
        if tab.sections.is_empty() || tab.sections.len() > MAX_LAYOUT_SECTIONS {
            push(
                format!("{tab_path}.sections"),
                format!("must have between 1 and {MAX_LAYOUT_SECTIONS} sections"),
            );
        }
        for (section_index, section) in tab.sections.iter().enumerate() {
            let section_path = format!("{tab_path}.sections.{section_index}");
            check_id(
                &section.id,
                &format!("{section_path}.id"),
                &mut section_ids,
                &mut push,
            );
            check_label(&section.label, &format!("{section_path}.label"), &mut push);
            if section.columns.is_empty() || section.columns.len() > MAX_LAYOUT_COLUMNS {
                push(
                    format!("{section_path}.columns"),
                    format!("must have between 1 and {MAX_LAYOUT_COLUMNS} columns"),
                );
            }
            for (column_index, column) in section.columns.iter().enumerate() {
                for (field_index, field) in column.fields.iter().enumerate() {
                    let path =
                        format!("{section_path}.columns.{column_index}.fields.{field_index}");
                    match fields.find(&field.key) {
                        None => push(format!("{path}.key"), "is not a known field".to_owned()),
                        Some(resolved) if !resolved.definition.is_active => {
                            push(format!("{path}.key"), "is inactive".to_owned())
                        }
                        Some(_) if !keys.insert(field.key.as_str()) => {
                            push(format!("{path}.key"), "is duplicated".to_owned())
                        }
                        Some(_) => placed.push((path, field)),
                    }
                }
            }
        }
    }
    if keys.len() > MAX_LAYOUT_FIELDS {
        push(
            "layout".to_owned(),
            format!("a layout can have at most {MAX_LAYOUT_FIELDS} fields"),
        );
    }

    for (path, field) in &placed {
        let required = fields
            .find(&field.key)
            .is_some_and(|resolved| resolved.definition.is_required);
        if let Some(filter) = &field.visible_when {
            if required && context.accepts_required_rules() {
                push(
                    format!("{path}.visible_when"),
                    "a required field cannot be hidden".to_owned(),
                );
            }
            check_rule(
                filter,
                &format!("{path}.visible_when"),
                &keys,
                fields,
                resolve,
                &mut push,
            );
        }
        if let Some(filter) = &field.required_when {
            if !context.accepts_required_rules() {
                push(
                    format!("{path}.required_when"),
                    "is not supported by view layouts".to_owned(),
                );
            }
            check_rule(
                filter,
                &format!("{path}.required_when"),
                &keys,
                fields,
                resolve,
                &mut push,
            );
        }
    }
    violations
}

pub fn with_required_fields(mut layout: FormLayout, fields: &FieldSet) -> FormLayout {
    let missing: Vec<LayoutField> = fields
        .active()
        .filter(|field| field.definition.is_required && !layout.contains(field.key()))
        .map(|field| LayoutField::new(field.key()))
        .collect();
    if let Some(column) = layout
        .tabs
        .first_mut()
        .and_then(|tab| tab.sections.first_mut())
        .and_then(|section| section.columns.first_mut())
    {
        column.fields.extend(missing);
    }
    layout
}

pub fn condition_fields(filter: &Filter, keys: &mut Vec<String>) {
    match filter {
        Filter::Group(group) => {
            for child in &group.conditions {
                condition_fields(child, keys);
            }
        }
        Filter::Condition(condition) => keys.push(condition.field.clone()),
        Filter::Related(_) => {}
    }
}

fn check_rule(
    filter: &Filter,
    path: &str,
    placed: &BTreeSet<&str>,
    fields: &FieldSet,
    resolve: &ResolveContext,
    push: &mut impl FnMut(String, String),
) {
    let violations = validate_view_query(fields, resolve, Some(filter), &[]);
    let invalid = !violations.is_empty();
    for violation in violations {
        let suffix = violation
            .field
            .strip_prefix("filter")
            .unwrap_or(&violation.field);
        push(format!("{path}{suffix}"), violation.message);
    }
    if invalid {
        return;
    }
    let mut keys = Vec::new();
    condition_fields(filter, &mut keys);
    if keys.iter().any(|key| !placed.contains(key.as_str())) {
        push(
            path.to_owned(),
            "can only reference fields placed in this layout".to_owned(),
        );
    }
}

fn check_id(
    id: &str,
    path: &str,
    seen: &mut BTreeSet<String>,
    push: &mut impl FnMut(String, String),
) {
    let valid = !id.is_empty()
        && id.len() <= MAX_LAYOUT_ID_LENGTH
        && id
            .chars()
            .all(|char| char.is_ascii_alphanumeric() || char == '_' || char == '-');
    if !valid {
        push(
            path.to_owned(),
            format!("must have between 1 and {MAX_LAYOUT_ID_LENGTH} letters, digits, '-' or '_'"),
        );
    } else if !seen.insert(id.to_owned()) {
        push(path.to_owned(), "is duplicated".to_owned());
    }
}

fn check_label(label: &str, path: &str, push: &mut impl FnMut(String, String)) {
    if label.trim().chars().count() > MAX_LAYOUT_LABEL_LENGTH {
        push(
            path.to_owned(),
            format!("must have at most {MAX_LAYOUT_LABEL_LENGTH} characters"),
        );
    }
}
