pub mod sql;
pub use aphel_contract_privacy::*;
use aphel_kernel::application::shared::REDACTED;
use aphel_kernel::{
    Ctx,
    application::{fields::LocalizedLabels, shared::RecordRef},
    database::transaction::Tx,
    errors::AppError,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use uuid::Uuid;
const MIN_REDACTED_VALUE_LENGTH: usize = 3;
const MIN_IDENTIFIER_DIGITS: usize = 6;
const MIN_NAME_WORD_LENGTH: usize = 2;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubjectMatcher {
    pub kind: SubjectKind,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub digits: Option<String>,
    pub text: Option<String>,
}

impl SubjectMatcher {
    pub fn canonical(&self) -> String {
        self.email
            .clone()
            .or_else(|| self.phone.clone())
            .or_else(|| self.digits.clone())
            .or_else(|| self.text.as_ref().map(|text| text.to_lowercase()))
            .unwrap_or_default()
    }

    pub fn label(&self) -> String {
        match self.kind {
            SubjectKind::Email => mask_email(self.email.as_deref().unwrap_or_default()),
            SubjectKind::Phone => mask_digits(
                self.phone
                    .as_deref()
                    .or(self.digits.as_deref())
                    .unwrap_or_default(),
            ),
            SubjectKind::Document => mask_digits(self.digits.as_deref().unwrap_or_default()),
            SubjectKind::Text => mask_text(self.text.as_deref().unwrap_or_default()),
        }
    }

    pub fn hash(&self) -> String {
        let digest =
            Sha256::digest(format!("{}:{}", self.kind.as_str(), self.canonical()).as_bytes());
        digest.iter().map(|byte| format!("{byte:02x}")).collect()
    }
}

fn is_phone_like(term: &str) -> bool {
    term.chars()
        .all(|character| character.is_ascii_digit() || " +()-./".contains(character))
}

fn find_whole(text: &str, value: &str, from: usize) -> Option<usize> {
    let guard_start = value.starts_with(char::is_alphanumeric);
    let guard_end = value.ends_with(char::is_alphanumeric);
    text[from..]
        .match_indices(value)
        .map(|(offset, _)| from + offset)
        .find(|&start| {
            let end = start + value.len();
            (!guard_start
                || text[..start]
                    .chars()
                    .next_back()
                    .is_none_or(|character| !character.is_alphanumeric()))
                && (!guard_end
                    || text[end..]
                        .chars()
                        .next()
                        .is_none_or(|character| !character.is_alphanumeric()))
        })
}

fn mask_part(value: &str) -> String {
    let first: String = value.chars().take(1).collect();
    format!("{first}***")
}

pub fn mask_email(email: &str) -> String {
    let Some((local, domain)) = email.split_once('@') else {
        return mask_text(email);
    };
    let (host, suffix) = match domain.rsplit_once('.') {
        Some((host, suffix)) => (host, format!(".{suffix}")),
        None => (domain, String::new()),
    };
    format!("{}@{}{suffix}", mask_part(local), mask_part(host))
}

pub fn mask_digits(value: &str) -> String {
    let digits: Vec<char> = value.chars().filter(char::is_ascii_digit).collect();
    let visible = digits.len().saturating_sub(digits.len().min(4));
    let tail: String = digits[visible..].iter().collect();
    format!("***{tail}")
}

pub fn mask_text(value: &str) -> String {
    value
        .split_whitespace()
        .map(mask_part)
        .collect::<Vec<_>>()
        .join(" ")
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Redaction {
    values: BTreeSet<String>,
    fields: BTreeSet<String>,
    mentions: BTreeSet<String>,
}

fn is_identifier(value: &str) -> bool {
    let digits = value.chars().filter(char::is_ascii_digit).count();
    digits >= MIN_IDENTIFIER_DIGITS && is_phone_like(value)
}

fn is_email_like(value: &str) -> bool {
    value
        .split_once('@')
        .is_some_and(|(local, domain)| !local.is_empty() && domain.contains('.'))
        && !value.contains(char::is_whitespace)
}

fn json_texts(value: &Value) -> Vec<&str> {
    match value {
        Value::String(text) => vec![text.as_str()],
        Value::Array(items) => items.iter().flat_map(json_texts).collect(),
        Value::Object(map) => map.values().flat_map(json_texts).collect(),
        _ => Vec::new(),
    }
}

impl Redaction {
    pub fn merge(&mut self, other: &Self) {
        self.values.extend(other.values.iter().cloned());
        self.fields.extend(other.fields.iter().cloned());
        self.mentions.extend(other.mentions.iter().cloned());
    }

    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_value(&mut self, value: &str) {
        let value = value.trim().to_lowercase();
        if value.chars().count() >= MIN_REDACTED_VALUE_LENGTH {
            self.values.insert(value);
        }
    }

    pub fn add_json(&mut self, value: &Value) {
        match value {
            Value::String(text) => self.add_value(text),
            Value::Array(items) => items.iter().for_each(|item| self.add_json(item)),
            Value::Object(map) => map.values().for_each(|item| self.add_json(item)),
            _ => {}
        }
    }

    pub fn add_field(&mut self, field: impl Into<String>) {
        self.fields.insert(field.into());
    }

    pub fn add_strong_json(&mut self, value: &Value) {
        for text in json_texts(value) {
            let text = text.trim().to_lowercase();
            if text.chars().count() >= MIN_REDACTED_VALUE_LENGTH {
                self.mentions.insert(text.clone());
            }
            let digits: String = text.chars().filter(char::is_ascii_digit).collect();
            if digits.len() >= MIN_IDENTIFIER_DIGITS && is_phone_like(&text) {
                self.mentions.insert(digits);
            }
        }
    }

    pub fn add_identifiers_json(&mut self, value: &Value) {
        for text in json_texts(value) {
            let text = text.trim();
            if is_identifier(text) || is_email_like(text) {
                self.add_strong_json(&Value::String(text.to_owned()));
            }
        }
    }

    pub fn add_full_name(&mut self, value: &str) {
        let words: Vec<&str> = value.split_whitespace().collect();
        if words.len() >= 2
            && words
                .iter()
                .all(|word| word.chars().count() >= MIN_NAME_WORD_LENGTH)
        {
            self.mentions.insert(words.join(" ").to_lowercase());
        }
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty() && self.fields.is_empty()
    }

    pub fn mentions_only(&self) -> Self {
        Self {
            values: self.mentions.clone(),
            fields: BTreeSet::new(),
            mentions: BTreeSet::new(),
        }
    }

    pub fn values(&self) -> impl Iterator<Item = &str> {
        self.values.iter().map(String::as_str)
    }

    pub fn apply_text(&self, text: &str) -> Option<String> {
        if let Ok(mut parsed) = serde_json::from_str::<Value>(text) {
            return self.apply(&mut parsed).then(|| parsed.to_string());
        }
        let mut result = text.to_owned();
        let mut changed = false;
        for value in &self.values {
            let mut cursor = 0;
            loop {
                let lowered = result.to_lowercase();
                if lowered.len() != result.len() {
                    return Some(REDACTED.to_owned());
                }
                let Some(start) = find_whole(&lowered, value, cursor) else {
                    break;
                };
                result.replace_range(start..start + value.len(), REDACTED);
                cursor = start + REDACTED.len();
                changed = true;
            }
        }
        changed.then_some(result)
    }

    fn matches_text(&self, text: &str) -> bool {
        if text == REDACTED {
            return false;
        }
        let lowered = text.to_lowercase();
        self.values
            .iter()
            .any(|value| find_whole(&lowered, value, 0).is_some())
    }

    pub fn apply(&self, value: &mut Value) -> bool {
        match value {
            Value::String(text) if self.matches_text(text) => {
                *text = REDACTED.to_owned();
                true
            }
            Value::Array(items) => {
                let mut changed = false;
                for item in items.iter_mut() {
                    changed |= self.apply(item);
                }
                changed
            }
            Value::Object(map) => {
                let field_redacted = map
                    .get("field")
                    .and_then(Value::as_str)
                    .is_some_and(|field| self.fields.contains(field));
                let mut changed = false;
                for (key, item) in map.iter_mut() {
                    let redact_whole = (field_redacted && (key == "old" || key == "new"))
                        || self.fields.contains(key.as_str());
                    if redact_whole && !item.is_null() {
                        if item.as_str() != Some(REDACTED) {
                            *item = Value::String(REDACTED.to_owned());
                            changed = true;
                        }
                    } else {
                        changed = self.apply(item) || changed;
                    }
                }
                changed
            }
            _ => false,
        }
    }
}

pub struct DataSubjectRequest<'a> {
    pub id: Uuid,
    pub action: DataSubjectAction,
    pub records: &'a [RecordRef],
}

pub struct DataSubjectOutcome {
    pub source: String,
    pub result: Result<Value, AppError>,
}

#[async_trait]
pub trait DataSubjectSource: Send + Sync {
    async fn run(&self, request: &DataSubjectRequest<'_>) -> Vec<DataSubjectOutcome>;
}

pub fn affected_items(data: &Value) -> u64 {
    data.get("affected").and_then(Value::as_u64).unwrap_or(0)
}

#[derive(Debug, Clone)]
pub struct PrivacyItem {
    pub target: SourceTarget,
    pub record: Option<RecordRef>,
    pub title: String,
    pub excerpt: Option<String>,
    pub reason: String,
    pub metadata: Value,
}
#[derive(Debug, Clone, Default)]
pub struct SourceSnapshot {
    pub data: Vec<Value>,
    pub identifiers: Vec<Value>,
}
#[derive(Debug, Clone, Default)]
pub struct SourceOutcome {
    pub affected: u64,
    pub storage_keys: Vec<String>,
}
pub struct SourceRequest<'a> {
    pub id: Uuid,
    pub action: DataSubjectAction,
    pub records: &'a [RecordRef],
    pub items: &'a [String],
    pub redaction: &'a Redaction,
    pub at: DateTime<Utc>,
}
#[async_trait]
pub trait PrivacySource: Send + Sync {
    fn key(&self) -> &'static str;
    fn entity_type(&self) -> Option<aphel_kernel::application::shared::RecordType> {
        None
    }
    async fn available(&self, _ctx: &Ctx<'_>) -> Result<bool, AppError> {
        Ok(true)
    }
    async fn restrict_inactive(
        &self,
        _ctx: &Ctx<'_>,
        _builder: &mut aphel_kernel::database::SqlBuilder,
        _record_type: &aphel_kernel::application::shared::RecordType,
        _threshold: DateTime<Utc>,
    ) -> Result<(), AppError> {
        Ok(())
    }
    fn labels(&self) -> LocalizedLabels;
    async fn search(
        &self,
        _ctx: &Ctx<'_>,
        _matcher: &SubjectMatcher,
        _records: &[RecordRef],
        _limit: u32,
    ) -> Result<Vec<PrivacyItem>, AppError> {
        Ok(Vec::new())
    }
    async fn existing(&self, _ctx: &Ctx<'_>, _ids: &[String]) -> Result<Vec<String>, AppError> {
        Ok(Vec::new())
    }
    async fn snapshot(
        &self,
        _ctx: &Ctx<'_>,
        _records: &[RecordRef],
        _ids: &[String],
    ) -> Result<SourceSnapshot, AppError> {
        Ok(SourceSnapshot::default())
    }
    async fn erase(
        &self,
        _ctx: &Ctx<'_>,
        _tx: &mut Tx,
        _request: &SourceRequest<'_>,
    ) -> Result<SourceOutcome, AppError> {
        Ok(SourceOutcome::default())
    }
    async fn redact(
        &self,
        _ctx: &Ctx<'_>,
        _tx: &mut Tx,
        _redaction: &Redaction,
    ) -> Result<u64, AppError> {
        Ok(0)
    }
}

pub trait PrivacyTraits {
    fn privacy_policy(self, policy: EntityPrivacyPolicy) -> Self;
    fn retention_mapping(self, mapping: RetentionMapping) -> Self;
    fn privacy_policy_data(&self) -> Result<EntityPrivacyPolicy, AppError>;
    fn retention_mapping_data(&self) -> Result<RetentionMapping, AppError>;
}
impl PrivacyTraits for crate::fields::EntityTraits {
    fn privacy_policy(self, policy: EntityPrivacyPolicy) -> Self {
        self.with_trait_data(
            "privacy.fields",
            serde_json::to_value(policy).expect("privacy policy metadata"),
        )
    }
    fn retention_mapping(self, mapping: RetentionMapping) -> Self {
        self.with_trait_data(
            "privacy.retention",
            serde_json::to_value(mapping).expect("retention metadata"),
        )
    }
    fn privacy_policy_data(&self) -> Result<EntityPrivacyPolicy, AppError> {
        self.trait_data("privacy.fields")
            .map(|value| {
                serde_json::from_value(value.clone()).map_err(|error| {
                    AppError::Configuration(format!("invalid entity privacy policy: {error}"))
                })
            })
            .transpose()
            .map(Option::unwrap_or_default)
    }
    fn retention_mapping_data(&self) -> Result<RetentionMapping, AppError> {
        self.trait_data("privacy.retention")
            .map(|value| {
                serde_json::from_value(value.clone()).map_err(|error| {
                    AppError::Configuration(format!("invalid entity retention mapping: {error}"))
                })
            })
            .transpose()
            .map(Option::unwrap_or_default)
    }
}
