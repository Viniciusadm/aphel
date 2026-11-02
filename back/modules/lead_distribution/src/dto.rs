use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use utoipa::{IntoParams, ToSchema};

use crate::domain::{
    AssignmentEntry, AssignmentSource, DistributionStrategy, Ineligibility, RouteSelection,
    RoutingFallback,
};
use crate::{
    AssignmentListQuery, Recipient, RouteInput, RoutingInput, RuleListQuery, RuleMemberInput,
    RuleView, SaveRuleCommand, SimulateCommand, Simulation, SkippedMember,
};
use aphel_kernel::api::dto::common::double_option;
use aphel_kernel::domain::{TeamId, UserId};

fn timestamp(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Secs, true)
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = LeadDistributionStrategy)]
pub(crate) enum LeadDistributionStrategySchema {
    RoundRobin,
    Weighted,
    LeastLoaded,
}

impl From<DistributionStrategy> for LeadDistributionStrategySchema {
    fn from(strategy: DistributionStrategy) -> Self {
        match strategy {
            DistributionStrategy::RoundRobin => Self::RoundRobin,
            DistributionStrategy::Weighted => Self::Weighted,
            DistributionStrategy::LeastLoaded => Self::LeastLoaded,
        }
    }
}

impl From<LeadDistributionStrategySchema> for DistributionStrategy {
    fn from(strategy: LeadDistributionStrategySchema) -> Self {
        match strategy {
            LeadDistributionStrategySchema::RoundRobin => Self::RoundRobin,
            LeadDistributionStrategySchema::Weighted => Self::Weighted,
            LeadDistributionStrategySchema::LeastLoaded => Self::LeastLoaded,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, ToSchema)]
#[schema(as = LeadDistributionFallback)]
pub(crate) enum LeadDistributionFallbackSchema {
    #[serde(rename = "members")]
    Members,
    #[serde(rename = "none")]
    None,
}

impl From<RoutingFallback> for LeadDistributionFallbackSchema {
    fn from(fallback: RoutingFallback) -> Self {
        match fallback {
            RoutingFallback::Members => Self::Members,
            RoutingFallback::Unassigned => Self::None,
        }
    }
}

impl From<LeadDistributionFallbackSchema> for RoutingFallback {
    fn from(fallback: LeadDistributionFallbackSchema) -> Self {
        match fallback {
            LeadDistributionFallbackSchema::Members => Self::Members,
            LeadDistributionFallbackSchema::None => Self::Unassigned,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = LeadDistributionSelection)]
pub(crate) enum LeadDistributionSelectionSchema {
    Members,
    Route,
    Fallback,
    Unassigned,
}

impl From<RouteSelection> for LeadDistributionSelectionSchema {
    fn from(selection: RouteSelection) -> Self {
        match selection {
            RouteSelection::Members => Self::Members,
            RouteSelection::Route(_) => Self::Route,
            RouteSelection::Fallback => Self::Fallback,
            RouteSelection::Unassigned => Self::Unassigned,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = LeadDistributionSkipReason)]
pub(crate) enum LeadDistributionSkipReasonSchema {
    Away,
    Inactive,
}

impl From<Ineligibility> for LeadDistributionSkipReasonSchema {
    fn from(reason: Ineligibility) -> Self {
        match reason {
            Ineligibility::Away => Self::Away,
            Ineligibility::Inactive => Self::Inactive,
        }
    }
}

#[derive(Debug, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct LeadDistributionRuleListParams {
    #[param(example = "deals.deal")]
    pub(crate) record_type: Option<String>,
    pub(crate) active: Option<bool>,
}

impl LeadDistributionRuleListParams {
    pub(crate) fn into_query(self) -> RuleListQuery {
        RuleListQuery {
            record_type: self.record_type,
            active: self.active,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = LeadDistributionAssignmentSource)]
pub(crate) enum LeadDistributionAssignmentSourceSchema {
    Automation,
    IncomingWebhook,
    Import,
    Scheduling,
    LeadForm,
    Plugin,
}

impl From<AssignmentSource> for LeadDistributionAssignmentSourceSchema {
    fn from(source: AssignmentSource) -> Self {
        match source {
            AssignmentSource::Automation => Self::Automation,
            AssignmentSource::IncomingWebhook => Self::IncomingWebhook,
            AssignmentSource::Import => Self::Import,
            AssignmentSource::Scheduling => Self::Scheduling,
            AssignmentSource::LeadForm => Self::LeadForm,
            AssignmentSource::Plugin => Self::Plugin,
        }
    }
}

impl From<LeadDistributionAssignmentSourceSchema> for AssignmentSource {
    fn from(source: LeadDistributionAssignmentSourceSchema) -> Self {
        match source {
            LeadDistributionAssignmentSourceSchema::Automation => Self::Automation,
            LeadDistributionAssignmentSourceSchema::IncomingWebhook => Self::IncomingWebhook,
            LeadDistributionAssignmentSourceSchema::Import => Self::Import,
            LeadDistributionAssignmentSourceSchema::Scheduling => Self::Scheduling,
            LeadDistributionAssignmentSourceSchema::LeadForm => Self::LeadForm,
            LeadDistributionAssignmentSourceSchema::Plugin => Self::Plugin,
        }
    }
}

#[derive(Debug, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct LeadDistributionAssignmentListParams {
    pub(crate) rule_id: Option<String>,
    pub(crate) user_id: Option<String>,
    #[param(example = "deals.deal")]
    pub(crate) record_type: Option<String>,
    pub(crate) source: Option<LeadDistributionAssignmentSourceSchema>,
}

impl LeadDistributionAssignmentListParams {
    pub(crate) fn into_query(self) -> AssignmentListQuery {
        AssignmentListQuery {
            rule_id: self.rule_id,
            user_id: self.user_id,
            record_type: self.record_type,
            source: self.source.map(Into::into),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct LeadDistributionAssignmentResponse {
    pub(crate) id: String,
    pub(crate) rule_id: String,
    pub(crate) rule_key: String,
    pub(crate) rule_name: String,
    pub(crate) user_id: Option<String>,
    pub(crate) user_name: Option<String>,
    #[schema(example = "deals.deal")]
    pub(crate) record_type: Option<String>,
    pub(crate) record_id: Option<String>,
    pub(crate) source: LeadDistributionAssignmentSourceSchema,
    #[schema(example = "2026-10-21T13:45:00Z")]
    pub(crate) created_at: String,
}

impl From<AssignmentEntry> for LeadDistributionAssignmentResponse {
    fn from(entry: AssignmentEntry) -> Self {
        Self {
            id: entry.id.to_string(),
            rule_id: entry.rule_id.to_string(),
            rule_key: entry.rule_key,
            rule_name: entry.rule_name,
            user_id: entry.user_id.map(UserId::into_inner),
            user_name: entry.user_name,
            record_type: entry.record_type,
            record_id: entry.record_id,
            source: entry.source.into(),
            created_at: timestamp(entry.created_at),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct LeadDistributionMemberRequest {
    pub(crate) user_id: String,
    #[schema(minimum = 1, maximum = 100, example = 1)]
    pub(crate) weight: Option<u32>,
    #[serde(default)]
    pub(crate) away: bool,
    #[schema(example = "2026-10-21T00:00:00Z")]
    pub(crate) away_from: Option<String>,
    #[schema(example = "2026-10-28T00:00:00Z")]
    pub(crate) away_until: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct LeadDistributionRouteRequest {
    #[schema(example = json!(["sp", "rj"]))]
    pub(crate) values: Vec<String>,
    pub(crate) team_id: Option<String>,
    #[serde(default)]
    pub(crate) user_ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct LeadDistributionRoutingRequest {
    #[schema(example = "source")]
    pub(crate) field: String,
    pub(crate) routes: Vec<LeadDistributionRouteRequest>,
    pub(crate) fallback: Option<LeadDistributionFallbackSchema>,
}

#[derive(Debug, Default, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct LeadDistributionRuleRequest {
    #[schema(example = "inbound_leads")]
    pub(crate) key: Option<String>,
    #[schema(example = "Website leads")]
    pub(crate) name: Option<String>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub(crate) description: Option<Option<String>>,
    #[schema(example = "deals.deal")]
    pub(crate) record_type: Option<String>,
    pub(crate) strategy: Option<LeadDistributionStrategySchema>,
    pub(crate) is_active: Option<bool>,
    pub(crate) members: Option<Vec<LeadDistributionMemberRequest>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<LeadDistributionRoutingRequest>)]
    pub(crate) routing: Option<Option<LeadDistributionRoutingRequest>>,
}

impl LeadDistributionRuleRequest {
    pub(crate) fn into_command(
        self,
        ip: Option<String>,
        expected_updated_at: Option<DateTime<Utc>>,
    ) -> SaveRuleCommand {
        SaveRuleCommand {
            key: self.key,
            name: self.name,
            description: self.description,
            record_type: self.record_type,
            strategy: self.strategy.map(Into::into),
            is_active: self.is_active,
            members: self.members.map(|members| {
                members
                    .into_iter()
                    .map(|member| RuleMemberInput {
                        user_id: member.user_id,
                        weight: member.weight,
                        away: member.away,
                        away_from: member.away_from,
                        away_until: member.away_until,
                    })
                    .collect()
            }),
            routing: self.routing.map(|routing| {
                routing.map(|routing| RoutingInput {
                    field: routing.field,
                    fallback: routing.fallback.map(Into::into),
                    routes: routing
                        .routes
                        .into_iter()
                        .map(|route| RouteInput {
                            values: route.values,
                            team_id: route.team_id,
                            user_ids: route.user_ids,
                        })
                        .collect(),
                })
            }),
            ip,
            expected_updated_at,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct LeadDistributionMemberResponse {
    pub(crate) user_id: String,
    pub(crate) name: Option<String>,
    pub(crate) weight: u32,
    pub(crate) away: bool,
    pub(crate) away_from: Option<String>,
    pub(crate) away_until: Option<String>,
    pub(crate) away_now: bool,
    pub(crate) active: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct LeadDistributionRouteResponse {
    pub(crate) values: Vec<String>,
    pub(crate) team_id: Option<String>,
    pub(crate) user_ids: Vec<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct LeadDistributionRoutingResponse {
    pub(crate) field: String,
    pub(crate) routes: Vec<LeadDistributionRouteResponse>,
    pub(crate) fallback: LeadDistributionFallbackSchema,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct LeadDistributionRuleResponse {
    pub(crate) id: String,
    pub(crate) key: String,
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) record_type: String,
    pub(crate) strategy: LeadDistributionStrategySchema,
    pub(crate) is_active: bool,
    pub(crate) members: Vec<LeadDistributionMemberResponse>,
    pub(crate) routing: Option<LeadDistributionRoutingResponse>,
    pub(crate) assignments: u64,
    pub(crate) last_user_id: Option<String>,
    pub(crate) last_assigned_at: Option<String>,
    pub(crate) can_manage: bool,
    pub(crate) created_by: Option<String>,
    #[schema(example = "2026-10-21T13:45:00Z")]
    pub(crate) created_at: String,
    #[schema(example = "2026-10-21T13:45:00Z")]
    pub(crate) updated_at: String,
}

impl From<RuleView> for LeadDistributionRuleResponse {
    fn from(view: RuleView) -> Self {
        let rule = view.rule;
        let users = view.users;
        let now = view.now;
        Self {
            id: rule.id.to_string(),
            key: rule.key,
            name: rule.name,
            description: rule.description,
            record_type: rule.record_type,
            strategy: rule.strategy.into(),
            is_active: rule.is_active,
            members: rule
                .members
                .into_iter()
                .map(|member| {
                    let status = users.get(&member.user_id);
                    LeadDistributionMemberResponse {
                        away_now: member.is_away_at(now),
                        name: status.map(|status| status.name.clone()),
                        active: status.is_some_and(|status| status.active),
                        user_id: member.user_id.into_inner(),
                        weight: member.weight,
                        away: member.away,
                        away_from: member.away_from.map(timestamp),
                        away_until: member.away_until.map(timestamp),
                    }
                })
                .collect(),
            routing: rule.routing.map(|routing| LeadDistributionRoutingResponse {
                field: routing.field,
                fallback: routing.fallback.into(),
                routes: routing
                    .routes
                    .into_iter()
                    .map(|route| LeadDistributionRouteResponse {
                        values: route.values,
                        team_id: route.team_id.map(TeamId::into_inner),
                        user_ids: route.user_ids.into_iter().map(UserId::into_inner).collect(),
                    })
                    .collect(),
            }),
            assignments: rule.stats.sequence,
            last_user_id: rule.stats.last_user_id.map(UserId::into_inner),
            last_assigned_at: rule.stats.last_assigned_at.map(timestamp),
            can_manage: view.can_manage,
            created_by: rule.created_by.map(UserId::into_inner),
            created_at: timestamp(rule.created_at),
            updated_at: timestamp(rule.updated_at),
        }
    }
}

#[derive(Debug, Default, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct LeadDistributionSimulateRequest {
    #[schema(minimum = 1, maximum = 50, example = 10)]
    pub(crate) count: Option<usize>,
    #[serde(default)]
    #[schema(value_type = std::collections::HashMap<String, Value>, example = json!({ "source": "google" }))]
    pub(crate) values: Map<String, Value>,
}

impl LeadDistributionSimulateRequest {
    pub(crate) fn into_command(self) -> SimulateCommand {
        SimulateCommand {
            count: self.count,
            values: self.values,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct LeadDistributionRecipientResponse {
    pub(crate) position: usize,
    pub(crate) user_id: String,
    pub(crate) name: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct LeadDistributionSkippedResponse {
    pub(crate) user_id: String,
    pub(crate) name: Option<String>,
    pub(crate) reason: LeadDistributionSkipReasonSchema,
}

impl From<SkippedMember> for LeadDistributionSkippedResponse {
    fn from(skipped: SkippedMember) -> Self {
        Self {
            user_id: skipped.user_id.into_inner(),
            name: skipped.name,
            reason: skipped.reason.into(),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct LeadDistributionSimulationResponse {
    pub(crate) selection: LeadDistributionSelectionSchema,
    pub(crate) route_index: Option<usize>,
    pub(crate) recipients: Vec<LeadDistributionRecipientResponse>,
    pub(crate) skipped: Vec<LeadDistributionSkippedResponse>,
}

impl From<Simulation> for LeadDistributionSimulationResponse {
    fn from(simulation: Simulation) -> Self {
        Self {
            selection: simulation.selection.into(),
            route_index: simulation.selection.route_index(),
            recipients: simulation
                .recipients
                .into_iter()
                .enumerate()
                .map(
                    |(index, Recipient { user_id, name })| LeadDistributionRecipientResponse {
                        position: index + 1,
                        user_id: user_id.into_inner(),
                        name,
                    },
                )
                .collect(),
            skipped: simulation.skipped.into_iter().map(Into::into).collect(),
        }
    }
}
