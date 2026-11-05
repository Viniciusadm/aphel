use std::collections::{BTreeSet, HashMap};
use std::sync::LazyLock;

use regex::{Captures, Regex};
use serde_json::Value;

use crate::compose::html::{escape_html, html_to_text, sanitize_html};
use crate::templates::{RenderedEmail, VariableField};
use aphel_sdk::platform::application::fields::RelatedRecord;

pub(crate) const RECORD_PREFIX: &str = "record";
pub(crate) const USER_PREFIX: &str = "user";
const CUSTOM_FIELDS_KEY: &str = "custom_fields";

static VARIABLE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\{\{\s*([A-Za-z][A-Za-z0-9_]*(?:\.[A-Za-z0-9_]+)+)\s*\}\}").expect("valid regex")
});

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct VariableValues {
    values: HashMap<String, String>,
}

impl VariableValues {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn insert(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.values.insert(key.into(), value.into());
    }

    pub(crate) fn get(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(String::as_str)
    }

    pub(crate) fn extend_record(&mut self, prefix: &str, record: &Value, fields: &[VariableField]) {
        for field in fields {
            let value = record
                .get(&field.key)
                .filter(|value| !value.is_null())
                .or_else(|| {
                    record
                        .get(CUSTOM_FIELDS_KEY)
                        .and_then(|custom| custom.get(&field.key))
                        .filter(|value| !value.is_null())
                });
            if let Some(text) = value.and_then(value_text) {
                self.insert(format!("{prefix}.{}", field.key), text);
            }
        }
    }
}

pub(crate) fn value_text(value: &Value) -> Option<String> {
    match value {
        Value::Null => None,
        Value::String(text) => Some(text.clone()),
        Value::Bool(flag) => Some(flag.to_string()),
        Value::Number(number) => Some(number.to_string()),
        Value::Array(items) => {
            let parts: Vec<String> = items.iter().filter_map(value_text).collect();
            (!parts.is_empty()).then(|| parts.join(", "))
        }
        Value::Object(object) => object
            .get("name")
            .or_else(|| object.get("display_name"))
            .or_else(|| object.get("label"))
            .and_then(value_text),
    }
}

pub(crate) fn render_template(
    text: &str,
    values: &VariableValues,
    escape: bool,
    missing: &mut BTreeSet<String>,
) -> String {
    VARIABLE
        .replace_all(text, |captures: &Captures| {
            let key = &captures[1];
            match values.get(key) {
                Some(value) if escape => escape_html(value),
                Some(value) => value.to_owned(),
                None => {
                    missing.insert(key.to_owned());
                    String::new()
                }
            }
        })
        .into_owned()
}

pub(crate) fn related_records(record_type: &str) -> &'static [RelatedRecord] {
    aphel_sdk::platform::modules::app()
        .record_types()
        .capabilities(record_type)
        .map_or(&[], |capabilities| capabilities.related_records.as_slice())
}

pub(crate) fn related_record_types(record_type: &str) -> Vec<String> {
    std::iter::once(record_type.to_owned())
        .chain(
            related_records(record_type)
                .iter()
                .map(|related| related.record_type.as_str().to_owned()),
        )
        .collect()
}

pub(crate) fn render_email(
    subject: &str,
    body_html: &str,
    values: &VariableValues,
) -> RenderedEmail {
    let mut missing = BTreeSet::new();
    let subject = render_template(subject, values, false, &mut missing)
        .replace(['\r', '\n'], " ")
        .trim()
        .to_owned();
    let html = sanitize_html(&render_template(body_html, values, true, &mut missing));
    let text = html_to_text(&html);
    RenderedEmail {
        subject,
        html,
        text,
        missing_variables: missing.into_iter().collect(),
    }
}
