use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::application::config_packages::PlanChange;

const WHOLE_SECTION_KIND: &str = "section";
const WHOLE_SECTION_KEY: &str = "*";

#[derive(Debug, Clone, PartialEq)]
pub struct PackageFragment {
    pub id: String,
    pub section: String,
    pub kind: String,
    pub key: String,
    pub value: Value,
}

impl PackageFragment {
    pub fn hash(&self) -> String {
        canonical_hash(&self.value)
    }
}

pub fn item_id(section: &str, kind: &str, key: &str) -> String {
    format!("{section}:{kind}:{key}")
}

pub fn section_item_id(section: &str) -> String {
    item_id(section, WHOLE_SECTION_KIND, WHOLE_SECTION_KEY)
}

pub fn fragments(sections: &BTreeMap<String, Value>) -> BTreeMap<String, PackageFragment> {
    let mut fragments = BTreeMap::new();
    for (section, content) in sections {
        if content.is_null() {
            continue;
        }
        for (kind, key, value) in locate(section, content) {
            let id = item_id(section, &kind, &key);
            fragments.insert(
                id.clone(),
                PackageFragment {
                    id,
                    section: section.clone(),
                    kind,
                    key,
                    value,
                },
            );
        }
    }
    fragments
}

pub fn fragment_hash(
    fragments: &BTreeMap<String, PackageFragment>,
    sections: &BTreeMap<String, Value>,
    id: &str,
    section: &str,
) -> Option<String> {
    fragments
        .get(id)
        .or_else(|| fragments.get(&section_item_id(section)))
        .map(PackageFragment::hash)
        .or_else(|| {
            sections
                .get(section)
                .filter(|content| !content.is_null())
                .map(canonical_hash)
        })
}

fn locate(section: &str, content: &Value) -> Vec<(String, String, Value)> {
    if let Some(layout) = nested_layout(section) {
        return locate_nested(content, &layout);
    }
    let keyed = |kind: &str, entries: Option<&Value>, key: &dyn Fn(&Value) -> Option<String>| {
        entries
            .and_then(Value::as_array)
            .map(|entries| {
                entries
                    .iter()
                    .filter_map(|entry| key(entry).map(|key| (kind.to_owned(), key, entry.clone())))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    };
    match section {
        "custom_objects" => {
            let mut located = keyed("custom_object", content.get("objects"), &plain_key);
            located.extend(keyed(
                "relationship",
                content.get("relationships"),
                &plain_key,
            ));
            located
        }
        "fields" => keyed("field", Some(content), &|entry| {
            joined_key(entry, "record_type", "key")
        }),
        "lost_reasons" => keyed("lost_reason", Some(content), &plain_key),
        "activity_types" => keyed("activity_type", Some(content), &plain_key),
        "tags" => keyed("tag", Some(content), &plain_key),
        "automations" => keyed("automation", Some(content), &plain_key),
        "layouts" => keyed("layout", Some(content), &|entry| {
            joined_key(entry, "record_type", "context")
        }),
        "views" => keyed("view", Some(content), &|entry| {
            joined_key(entry, "record_type", "key")
        }),
        "dashboard" => vec![(
            "dashboard_layout".to_owned(),
            "default".to_owned(),
            content.clone(),
        )],
        _ => vec![(
            WHOLE_SECTION_KIND.to_owned(),
            WHOLE_SECTION_KEY.to_owned(),
            content.clone(),
        )],
    }
}

fn plain_key(entry: &Value) -> Option<String> {
    entry.get("key").and_then(Value::as_str).map(str::to_owned)
}

fn joined_key(entry: &Value, first: &str, second: &str) -> Option<String> {
    let first = entry.get(first).and_then(Value::as_str)?;
    let second = entry.get(second).and_then(Value::as_str)?;
    Some(format!("{first}.{second}"))
}

fn without_children(parent: &Value, children_field: &str) -> Value {
    let mut parent = parent.clone();
    if let Value::Object(object) = &mut parent {
        object.remove(children_field);
    }
    parent
}

pub fn replace_fragment(
    sections: &mut BTreeMap<String, Value>,
    fragment: &PackageFragment,
    replacement: Option<Value>,
) {
    let section = fragment.section.as_str();
    let Some(content) = sections.get_mut(section) else {
        return;
    };
    if let Some(layout) = nested_layout(section) {
        replace_nested(content, fragment, replacement, &layout);
        return;
    }
    match (section, fragment.kind.as_str()) {
        ("custom_objects", "custom_object") => replace_in(
            content.get_mut("objects"),
            &fragment.key,
            &plain_key,
            replacement,
        ),
        ("custom_objects", "relationship") => replace_in(
            content.get_mut("relationships"),
            &fragment.key,
            &plain_key,
            replacement,
        ),
        ("fields", _) | ("views", _) => replace_in(
            Some(content),
            &fragment.key,
            &|entry| joined_key(entry, "record_type", "key"),
            replacement,
        ),
        ("layouts", _) => replace_in(
            Some(content),
            &fragment.key,
            &|entry| joined_key(entry, "record_type", "context"),
            replacement,
        ),
        ("lost_reasons", _) | ("activity_types", _) | ("tags", _) | ("automations", _) => {
            replace_in(Some(content), &fragment.key, &plain_key, replacement)
        }
        _ => match replacement {
            Some(value) => *content = value,
            None => {
                sections.remove(section);
            }
        },
    }
}

fn replace_in(
    entries: Option<&mut Value>,
    key: &str,
    key_of: &dyn Fn(&Value) -> Option<String>,
    replacement: Option<Value>,
) {
    let Some(entries) = entries.and_then(Value::as_array_mut) else {
        return;
    };
    let Some(index) = entries
        .iter()
        .position(|entry| key_of(entry).as_deref() == Some(key))
    else {
        return;
    };
    match replacement {
        Some(value) => entries[index] = value,
        None => {
            entries.remove(index);
        }
    }
}

pub fn package_changes(previous: Option<&Value>, proposed: Option<&Value>) -> Vec<PlanChange> {
    match (previous, proposed) {
        (Some(Value::Object(previous)), Some(Value::Object(proposed))) => {
            let properties: BTreeSet<&String> = previous.keys().chain(proposed.keys()).collect();
            properties
                .into_iter()
                .filter_map(|property| {
                    let old = previous.get(property).cloned().unwrap_or(Value::Null);
                    let new = proposed.get(property).cloned().unwrap_or(Value::Null);
                    (canonical(&old) != canonical(&new))
                        .then(|| PlanChange::new(property.clone(), old, new))
                })
                .collect()
        }
        (previous, proposed) => {
            let old = previous.cloned().unwrap_or(Value::Null);
            let new = proposed.cloned().unwrap_or(Value::Null);
            if canonical(&old) == canonical(&new) {
                Vec::new()
            } else {
                vec![PlanChange::new("value", old, new)]
            }
        }
    }
}

pub fn canonical(value: &Value) -> String {
    let mut output = String::new();
    write_canonical(value, &mut output);
    output
}

fn write_canonical(value: &Value, output: &mut String) {
    match value {
        Value::Object(object) => {
            let mut keys: Vec<&String> = object.keys().collect();
            keys.sort();
            output.push('{');
            for (index, key) in keys.into_iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                output.push_str(&Value::String(key.clone()).to_string());
                output.push(':');
                write_canonical(&object[key], output);
            }
            output.push('}');
        }
        Value::Array(items) => {
            output.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                write_canonical(item, output);
            }
            output.push(']');
        }
        other => output.push_str(&other.to_string()),
    }
}

pub fn canonical_hash(value: &Value) -> String {
    sha256_hex(canonical(value).as_bytes())
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[derive(Debug, Clone)]
pub struct NestedFragmentLayout {
    pub section: &'static str,
    pub parent_kind: &'static str,
    pub child_kind: &'static str,
    pub children_field: &'static str,
}

fn nested_layout(section: &str) -> Option<std::sync::Arc<NestedFragmentLayout>> {
    crate::modules::try_app()?
        .contributions::<NestedFragmentLayout>()
        .into_iter()
        .find(|layout| layout.section == section)
}

fn locate_nested(content: &Value, layout: &NestedFragmentLayout) -> Vec<(String, String, Value)> {
    let mut located = Vec::new();
    for parent in content.as_array().into_iter().flatten() {
        let Some(parent_key) = plain_key(parent) else {
            continue;
        };
        located.push((
            layout.parent_kind.to_owned(),
            parent_key.clone(),
            without_children(parent, layout.children_field),
        ));
        for stage in parent
            .get(layout.children_field)
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            if let Some(stage_key) = plain_key(stage) {
                located.push((
                    layout.child_kind.to_owned(),
                    format!("{parent_key}.{stage_key}"),
                    stage.clone(),
                ));
            }
        }
    }
    located
}

fn replace_nested(
    content: &mut Value,
    fragment: &PackageFragment,
    replacement: Option<Value>,
    layout: &NestedFragmentLayout,
) {
    match fragment.kind.as_str() {
        kind if kind == layout.parent_kind => {
            let replacement = replacement.map(|mut local| {
                let stages = content
                    .as_array()
                    .and_then(|parents| {
                        parents
                            .iter()
                            .find(|parent| plain_key(parent).as_deref() == Some(&fragment.key))
                    })
                    .and_then(|parent| parent.get(layout.children_field).cloned());
                if let (Value::Object(object), Some(stages)) = (&mut local, stages) {
                    object.insert(layout.children_field.to_owned(), stages);
                }
                local
            });
            replace_in(Some(content), &fragment.key, &plain_key, replacement)
        }
        kind if kind == layout.child_kind => {
            let Some((parent_key, stage_key)) = fragment.key.split_once('.') else {
                return;
            };
            let parent = content.as_array_mut().and_then(|parents| {
                parents
                    .iter_mut()
                    .find(|parent| plain_key(parent).as_deref() == Some(parent_key))
            });
            if let Some(parent) = parent {
                replace_in(
                    parent.get_mut(layout.children_field),
                    stage_key,
                    &plain_key,
                    replacement,
                );
            }
        }

        _ => {}
    }
}
