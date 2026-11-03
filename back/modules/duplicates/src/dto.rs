use std::collections::BTreeMap;

use chrono::{DateTime, Utc};

use aphel_kernel::records::PossibleDuplicate;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::domain::DuplicateRule;
use crate::service::DuplicateTypeInfo;
use crate::{
    CandidateQuery, CandidateView, DuplicateMatch, DuplicateProbeInput, MergeCommand, MergeOutcome,
    RecordSummary, RuleSetting,
};
use aphel_kernel::api::dto::common::timestamp;

fn rule_keys(rules: &[DuplicateRule]) -> Vec<String> {
    rules.iter().map(|rule| rule.as_str().to_owned()).collect()
}

pub(crate) use aphel_kernel::api::dto::common::DuplicateMatchResponse;

pub(crate) fn match_response(found: DuplicateMatch) -> DuplicateMatchResponse {
    PossibleDuplicate::from(found).into()
}

impl From<DuplicateMatch> for PossibleDuplicate {
    fn from(found: DuplicateMatch) -> Self {
        Self {
            record_id: found.record_id.to_string(),
            title: found.title,
            rules: rule_keys(&found.rules),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct DuplicateRecordSummaryResponse {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) subtitle: Option<String>,
    pub(crate) owner_id: Option<String>,
    pub(crate) created_at: String,
}

impl From<RecordSummary> for DuplicateRecordSummaryResponse {
    fn from(summary: RecordSummary) -> Self {
        Self {
            id: summary.id.to_string(),
            title: summary.title,
            subtitle: summary.subtitle,
            owner_id: summary.owner_id.map(|id| id.into_inner()),
            created_at: timestamp(summary.created_at),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct DuplicateCandidateResponse {
    pub(crate) id: String,
    #[schema(example = "people.person")]
    pub(crate) record_type: String,
    pub(crate) record: DuplicateRecordSummaryResponse,
    pub(crate) duplicate: DuplicateRecordSummaryResponse,
    #[schema(example = json!(["email", "phone"]))]
    pub(crate) rules: Vec<String>,
    #[schema(example = "pending")]
    pub(crate) status: String,
    pub(crate) detected_at: String,
    pub(crate) resolved_at: Option<String>,
}

impl From<CandidateView> for DuplicateCandidateResponse {
    fn from(candidate: CandidateView) -> Self {
        Self {
            id: candidate.id.to_string(),
            record_type: candidate.record_type.as_str().to_owned(),
            record: candidate.record.into(),
            duplicate: candidate.duplicate.into(),
            rules: rule_keys(&candidate.rules),
            status: candidate.status.as_str().to_owned(),
            detected_at: timestamp(candidate.detected_at),
            resolved_at: candidate.resolved_at.map(timestamp),
        }
    }
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct DuplicateListParams {
    #[param(example = "people.person")]
    pub(crate) record_type: Option<String>,
    #[param(example = "pending")]
    pub(crate) status: Option<String>,
}

impl From<DuplicateListParams> for CandidateQuery {
    fn from(params: DuplicateListParams) -> Self {
        Self {
            record_type: params.record_type,
            status: params.status,
        }
    }
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct DuplicateRulesParams {
    #[param(example = "people.person")]
    pub(crate) record_type: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct DuplicateRuleResponse {
    #[schema(example = "email")]
    pub(crate) rule: String,
    pub(crate) enabled: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct DuplicateRulesResponse {
    pub(crate) record_type: String,
    pub(crate) rules: Vec<DuplicateRuleResponse>,
}

impl DuplicateRulesResponse {
    pub(crate) fn new(record_type: String, settings: Vec<RuleSetting>) -> Self {
        Self {
            record_type,
            rules: settings
                .into_iter()
                .map(|setting| DuplicateRuleResponse {
                    rule: setting.rule.as_str().to_owned(),
                    enabled: setting.enabled,
                })
                .collect(),
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct UpdateDuplicateRulesRequest {
    #[schema(example = "people.person")]
    pub(crate) record_type: String,
    #[schema(example = json!({ "email": true, "phone": false }))]
    pub(crate) rules: BTreeMap<String, bool>,
}

#[derive(Debug, Default, Deserialize, ToSchema)]
pub(crate) struct DuplicateCheckRequest {
    #[schema(example = "people.person")]
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
    #[serde(default)]
    #[schema(example = json!({ "code": "SKU-1" }))]
    pub(crate) values: BTreeMap<String, String>,
}

impl From<DuplicateCheckRequest> for DuplicateProbeInput {
    fn from(request: DuplicateCheckRequest) -> Self {
        Self {
            record_type: request.record_type,
            exclude_id: request.exclude_id,
            name: request.name,
            first_name: request.first_name,
            last_name: request.last_name,
            email: request.email,
            phone: request.phone,
            mobile: request.mobile,
            company_id: request.company_id,
            domain: request.domain,
            website: request.website,
            values: request.values,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct MergeRequest {
    #[schema(example = "people.person")]
    pub(crate) record_type: String,
    pub(crate) primary_id: String,
    pub(crate) duplicate_id: String,
    #[serde(default)]
    #[schema(example = json!({ "email": "duplicate", "phone": "primary" }))]
    pub(crate) fields: BTreeMap<String, String>,
}

impl MergeRequest {
    pub(crate) fn into_command(
        self,
        expected_version: Option<DateTime<Utc>>,
        ip: Option<String>,
    ) -> MergeCommand {
        MergeCommand {
            record_type: self.record_type,
            primary_id: self.primary_id,
            duplicate_id: self.duplicate_id,
            fields: self.fields,
            expected_version,
            ip,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct MergeResponse {
    pub(crate) record_type: String,
    pub(crate) survivor_id: String,
    pub(crate) merged_id: String,
    #[schema(example = json!({ "tasks": 2, "notes": 1 }))]
    pub(crate) moved: BTreeMap<String, u64>,
}

impl From<MergeOutcome> for MergeResponse {
    fn from(outcome: MergeOutcome) -> Self {
        Self {
            record_type: outcome.record_type.as_str().to_owned(),
            survivor_id: outcome.survivor_id.to_string(),
            merged_id: outcome.merged_id.to_string(),
            moved: outcome.moved,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct DuplicateTypePermissionsResponse {
    #[schema(example = "people.read")]
    pub(crate) read: String,
    #[schema(example = "people.update")]
    pub(crate) update: String,
    #[schema(example = "people.delete")]
    pub(crate) delete: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct DuplicateTypeResponse {
    #[schema(example = "people.person")]
    pub(crate) record_type: String,
    pub(crate) label: Option<String>,
    pub(crate) custom: bool,
    pub(crate) rules: Vec<DuplicateRuleResponse>,
    #[schema(example = json!(["first_name", "email"]))]
    pub(crate) merge_fields: Vec<String>,
    pub(crate) permissions: DuplicateTypePermissionsResponse,
}

impl From<DuplicateTypeInfo> for DuplicateTypeResponse {
    fn from(info: DuplicateTypeInfo) -> Self {
        let definition = &info.kind.definition;
        let label = definition
            .custom
            .as_ref()
            .map(|labels| labels.plural.clone())
            .or_else(|| {
                definition
                    .capabilities
                    .plural_labels_value()
                    .map(|labels| labels.default_text().to_owned())
            });
        Self {
            record_type: info.kind.as_str().to_owned(),
            label,
            custom: definition.is_custom(),
            rules: info
                .rules
                .into_iter()
                .map(|setting| DuplicateRuleResponse {
                    rule: setting.rule.as_str().to_owned(),
                    enabled: setting.enabled,
                })
                .collect(),
            merge_fields: info.merge_fields,
            permissions: DuplicateTypePermissionsResponse {
                read: info.kind.read.as_str().to_owned(),
                update: info.kind.update.as_str().to_owned(),
                delete: info.kind.delete.as_str().to_owned(),
            },
        }
    }
}
