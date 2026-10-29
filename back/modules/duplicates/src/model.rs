use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde_json::Value;
use uuid::Uuid;

use crate::DuplicateType;
use crate::domain::{CandidateStatus, DuplicateFamily, DuplicateRule, ordered_pair};
use aphel_kernel::application::fields::LabelNormalization;
use aphel_kernel::application::shared::RecordType;
use aphel_kernel::domain::UserId;
use aphel_kernel::domain::fields::{normalize_email, normalize_phone};
use aphel_kernel::errors::AppError;
use aphel_kernel::records::normalize_domain;
use companies::domain::domain_from_website;

pub(crate) const CANDIDATE_NOT_FOUND: &str = "duplicate_candidate_not_found";
pub(crate) const CANDIDATE_RESOLVED: &str = "duplicate_candidate_resolved";

pub(crate) const DUPLICATE_DISMISSED: &str = "duplicate.dismissed";
pub(crate) const DUPLICATE_RULES_UPDATED: &str = "duplicate_rules.updated";

pub(crate) const MERGE_SAME_RECORD: &str = "merge_same_record";

pub(crate) const DUPLICATES_SCAN_JOB: &str = "duplicates.scan";
pub(crate) const DUPLICATES_SCAN_CRON: &str = "0 15 3 * * *";
pub(crate) const DUPLICATES_CLEANUP_JOB: &str = "duplicates.cleanup";
pub(crate) const DUPLICATES_CLEANUP_CRON: &str = "0 45 2 * * *";
pub(crate) const CLEANUP_BATCH_SIZE: u32 = 500;
pub(crate) const SCAN_BATCH_SIZE: u32 = 1000;
pub(crate) const SCAN_WINDOW: usize = 25;
pub(crate) const CANDIDATE_FLUSH_SIZE: usize = 500;
pub(crate) const MAX_MATCHES: usize = 10;

const PROBE_COUNTRY: &str = "BR";

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct RecordFingerprint {
    pub(crate) id: Uuid,
    pub(crate) name: String,
    pub(crate) email: Option<String>,
    pub(crate) phones: Vec<String>,
    pub(crate) company_id: Option<Uuid>,
    pub(crate) domain: Option<String>,
    pub(crate) identifiers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RuleSetting {
    pub(crate) rule: DuplicateRule,
    pub(crate) enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DuplicateMatch {
    pub(crate) record_id: Uuid,
    pub(crate) title: String,
    pub(crate) rules: Vec<DuplicateRule>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NewCandidate {
    pub(crate) id: Uuid,
    pub(crate) record_id: Uuid,
    pub(crate) duplicate_id: Uuid,
    pub(crate) rules: Vec<DuplicateRule>,
    pub(crate) detected_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RecordSummary {
    pub(crate) id: Uuid,
    pub(crate) title: String,
    pub(crate) subtitle: Option<String>,
    pub(crate) owner_id: Option<UserId>,
    pub(crate) created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CandidateView {
    pub(crate) id: Uuid,
    pub(crate) record_type: RecordType,
    pub(crate) record: RecordSummary,
    pub(crate) duplicate: RecordSummary,
    pub(crate) rules: Vec<DuplicateRule>,
    pub(crate) status: CandidateStatus,
    pub(crate) detected_at: DateTime<Utc>,
    pub(crate) resolved_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct CandidateQuery {
    pub(crate) record_type: Option<String>,
    pub(crate) status: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct DuplicateProbeInput {
    pub(crate) record_type: String,
    pub(crate) exclude_id: Option<String>,
    pub(crate) name: Option<String>,
    pub(crate) first_name: Option<String>,
    pub(crate) last_name: Option<String>,
    pub(crate) email: Option<String>,
    pub(crate) phone: Option<String>,
    pub(crate) mobile: Option<String>,
    pub(crate) company_id: Option<String>,
    pub(crate) domain: Option<String>,
    pub(crate) website: Option<String>,
    pub(crate) values: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct MergeCommand {
    pub(crate) record_type: String,
    pub(crate) primary_id: String,
    pub(crate) duplicate_id: String,
    pub(crate) fields: BTreeMap<String, String>,
    pub(crate) expected_version: Option<DateTime<Utc>>,
    pub(crate) ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MergeOutcome {
    pub(crate) record_type: RecordType,
    pub(crate) survivor_id: Uuid,
    pub(crate) merged_id: Uuid,
    pub(crate) moved: BTreeMap<String, u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CandidateResolution {
    pub(crate) resolved_by: UserId,
    pub(crate) resolved_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct ScanSummary {
    pub(crate) records: usize,
    pub(crate) candidates: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct CleanupSummary {
    pub(crate) removed: usize,
    pub(crate) updated: usize,
}

pub(crate) fn candidate_not_found() -> AppError {
    AppError::not_found_code(CANDIDATE_NOT_FOUND, "duplicate candidate was not found")
}

fn clean(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

pub(crate) fn merged_event(record_type: &str) -> String {
    format!("{record_type}.merged")
}

pub(crate) fn updated_event(record_type: &str) -> String {
    format!("{record_type}.updated")
}

pub(crate) fn deleted_event(record_type: &str) -> String {
    format!("{record_type}.deleted")
}

pub(crate) fn normalize_identifier(
    value: &str,
    normalization: LabelNormalization,
) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    match normalization {
        LabelNormalization::Exact => Some(value.to_owned()),
        LabelNormalization::Lowercase => Some(value.to_lowercase()),
        LabelNormalization::Domain => normalize_domain(value),
    }
}

pub(crate) fn identifier_key(column: &str, value: &str) -> String {
    format!("{column}:{value}")
}

pub(crate) fn split_identifier(identifier: &str) -> Option<(&str, &str)> {
    identifier.split_once(':')
}

fn probe_name(kind: &DuplicateType, input: &DuplicateProbeInput) -> String {
    if kind.family == DuplicateFamily::Contact {
        let joined = [input.first_name.as_deref(), input.last_name.as_deref()]
            .into_iter()
            .flatten()
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        if !joined.is_empty() {
            return joined;
        }
    }
    clean(input.name.as_deref())
        .or_else(|| {
            kind.definition
                .title_field
                .as_deref()
                .and_then(|key| input.values.get(key))
                .and_then(|value| clean(Some(value)))
        })
        .unwrap_or_default()
}

fn probe_identifiers(kind: &DuplicateType, input: &DuplicateProbeInput) -> Vec<String> {
    kind.identifiers()
        .iter()
        .filter_map(|alias| {
            let raw = kind
                .field_for_column(&alias.column)
                .and_then(|field| input.values.get(field.key.as_str()))
                .or_else(|| input.values.get(&alias.column))?;
            normalize_identifier(raw, alias.normalization)
                .map(|value| identifier_key(&alias.column, &value))
        })
        .collect()
}

pub(crate) fn probe_fingerprint(
    kind: &DuplicateType,
    input: &DuplicateProbeInput,
) -> RecordFingerprint {
    let phones = [input.phone.as_deref(), input.mobile.as_deref()]
        .into_iter()
        .flatten()
        .filter_map(|phone| normalize_phone(phone, PROBE_COUNTRY).ok())
        .collect();
    let domain = clean(input.domain.as_deref())
        .and_then(|domain| normalize_domain(&domain))
        .or_else(|| clean(input.website.as_deref()).and_then(|site| domain_from_website(&site)));
    RecordFingerprint {
        id: input
            .exclude_id
            .as_deref()
            .and_then(|id| Uuid::parse_str(id.trim()).ok())
            .unwrap_or_else(Uuid::nil),
        name: probe_name(kind, input),
        email: clean(input.email.as_deref()).and_then(|email| normalize_email(&email).ok()),
        phones,
        company_id: input
            .company_id
            .as_deref()
            .and_then(|id| Uuid::parse_str(id.trim()).ok()),
        domain: if kind.family == DuplicateFamily::Company {
            domain
        } else {
            None
        },
        identifiers: probe_identifiers(kind, input),
    }
}

pub(crate) fn new_candidate(
    left: Uuid,
    right: Uuid,
    rules: Vec<DuplicateRule>,
    detected_at: DateTime<Utc>,
) -> NewCandidate {
    let (record_id, duplicate_id) = ordered_pair(left, right);
    NewCandidate {
        id: Uuid::now_v7(),
        record_id,
        duplicate_id,
        rules,
        detected_at,
    }
}

pub(crate) fn rules_json(rules: &[DuplicateRule]) -> Value {
    Value::Array(
        rules
            .iter()
            .map(|rule| Value::String(rule.as_str().to_owned()))
            .collect(),
    )
}
