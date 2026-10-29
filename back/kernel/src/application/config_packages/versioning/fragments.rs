use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::application::config_packages::PlanChange;

const STAGES: &str = "stages";
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
        "pipelines" => {
            let mut located = Vec::new();
            for pipeline in content.as_array().into_iter().flatten() {
                let Some(pipeline_key) = plain_key(pipeline) else {
                    continue;
                };
                located.push((
                    "pipeline".to_owned(),
                    pipeline_key.clone(),
                    without_stages(pipeline),
                ));
                for stage in pipeline
                    .get(STAGES)
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                {
                    if let Some(stage_key) = plain_key(stage) {
                        located.push((
                            "stage".to_owned(),
                            format!("{pipeline_key}.{stage_key}"),
                            stage.clone(),
                        ));
                    }
                }
            }
            located
        }
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

fn without_stages(pipeline: &Value) -> Value {
    let mut pipeline = pipeline.clone();
    if let Value::Object(object) = &mut pipeline {
        object.remove(STAGES);
    }
    pipeline
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
        ("pipelines", "pipeline") => {
            let replacement = replacement.map(|mut local| {
                let stages = content
                    .as_array()
                    .and_then(|pipelines| {
                        pipelines
                            .iter()
                            .find(|pipeline| plain_key(pipeline).as_deref() == Some(&fragment.key))
                    })
                    .and_then(|pipeline| pipeline.get(STAGES).cloned());
                if let (Value::Object(object), Some(stages)) = (&mut local, stages) {
                    object.insert(STAGES.to_owned(), stages);
                }
                local
            });
            replace_in(Some(content), &fragment.key, &plain_key, replacement)
        }
        ("pipelines", "stage") => {
            let Some((pipeline_key, stage_key)) = fragment.key.split_once('.') else {
                return;
            };
            let pipeline = content.as_array_mut().and_then(|pipelines| {
                pipelines
                    .iter_mut()
                    .find(|pipeline| plain_key(pipeline).as_deref() == Some(pipeline_key))
            });
            if let Some(pipeline) = pipeline {
                replace_in(pipeline.get_mut(STAGES), stage_key, &plain_key, replacement);
            }
        }
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
