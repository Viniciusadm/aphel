use chrono::{DateTime, Utc};
use uuid::Uuid;

use aphel_kernel::domain::{TeamId, UserId};

pub(crate) const MIN_WEIGHT: u32 = 1;
pub(crate) const MAX_WEIGHT: u32 = 100;
pub(crate) const MAX_MEMBERS: usize = 200;
pub(crate) const MAX_ROUTES: usize = 50;
pub(crate) const MAX_ROUTE_VALUES: usize = 50;
pub(crate) const MAX_ROUTE_USERS: usize = 100;
pub(crate) const MAX_ROUTE_VALUE_LENGTH: usize = 255;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DistributionStrategy {
    RoundRobin,
    Weighted,
    LeastLoaded,
}

impl DistributionStrategy {
    pub(crate) const ALL: [Self; 3] = [Self::RoundRobin, Self::Weighted, Self::LeastLoaded];

    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::RoundRobin => "round_robin",
            Self::Weighted => "weighted",
            Self::LeastLoaded => "least_loaded",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|strategy| strategy.as_str() == value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum RoutingFallback {
    Members,
    Unassigned,
}

impl RoutingFallback {
    pub(crate) const ALL: [Self; 2] = [Self::Members, Self::Unassigned];

    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Members => "members",
            Self::Unassigned => "none",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|fallback| fallback.as_str() == value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleMember {
    pub user_id: UserId,
    pub(crate) weight: u32,
    pub away: bool,
    pub away_from: Option<DateTime<Utc>>,
    pub away_until: Option<DateTime<Utc>>,
}

impl RuleMember {
    pub(crate) fn new(user_id: UserId) -> Self {
        Self {
            user_id,
            weight: MIN_WEIGHT,
            away: false,
            away_from: None,
            away_until: None,
        }
    }

    pub(crate) fn is_away_at(&self, at: DateTime<Utc>) -> bool {
        self.away
            && self.away_from.is_none_or(|from| from <= at)
            && self.away_until.is_none_or(|until| at < until)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Route {
    pub(crate) values: Vec<String>,
    pub(crate) team_id: Option<TeamId>,
    pub(crate) user_ids: Vec<UserId>,
}

impl Route {
    pub(crate) fn matches(&self, record_values: &[String]) -> bool {
        self.values.iter().any(|value| {
            let value = normalize_route_value(value);
            record_values.contains(&value)
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Routing {
    pub(crate) field: String,
    pub(crate) routes: Vec<Route>,
    pub(crate) fallback: RoutingFallback,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteSelection {
    Members,
    Route(usize),
    Fallback,
    Unassigned,
}

impl RouteSelection {
    pub(crate) fn route_index(&self) -> Option<usize> {
        match self {
            Self::Route(index) => Some(*index),
            _ => None,
        }
    }

    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Members => "members",
            Self::Route(_) => "route",
            Self::Fallback => "fallback",
            Self::Unassigned => "unassigned",
        }
    }
}

pub(crate) fn normalize_route_value(value: &str) -> String {
    value.trim().to_lowercase()
}

pub(crate) fn select_route(routing: Option<&Routing>, record_values: &[String]) -> RouteSelection {
    let Some(routing) = routing else {
        return RouteSelection::Members;
    };
    match routing
        .routes
        .iter()
        .position(|route| route.matches(record_values))
    {
        Some(index) => RouteSelection::Route(index),
        None => match routing.fallback {
            RoutingFallback::Members => RouteSelection::Fallback,
            RoutingFallback::Unassigned => RouteSelection::Unassigned,
        },
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Ineligibility {
    Away,
    Inactive,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub user_id: UserId,
    pub weight: u32,
    pub last_sequence: u64,
    pub credit: i64,
    pub load: u64,
}

pub fn pick(
    strategy: DistributionStrategy,
    candidates: &mut [Candidate],
    sequence: u64,
) -> Option<usize> {
    if candidates.is_empty() {
        return None;
    }
    let index = match strategy {
        DistributionStrategy::RoundRobin => candidates
            .iter()
            .enumerate()
            .min_by_key(|(index, candidate)| (candidate.last_sequence, *index))
            .map(|(index, _)| index)?,
        DistributionStrategy::LeastLoaded => candidates
            .iter()
            .enumerate()
            .min_by_key(|(index, candidate)| (candidate.load, candidate.last_sequence, *index))
            .map(|(index, _)| index)?,
        DistributionStrategy::Weighted => {
            let total: i64 = candidates
                .iter()
                .map(|candidate| i64::from(candidate.weight.max(MIN_WEIGHT)))
                .sum();
            for candidate in candidates.iter_mut() {
                candidate.credit = candidate.credit.clamp(-total, total)
                    + i64::from(candidate.weight.max(MIN_WEIGHT));
            }
            let index = candidates
                .iter()
                .enumerate()
                .max_by_key(|(index, candidate)| {
                    (
                        candidate.credit,
                        std::cmp::Reverse(candidate.last_sequence),
                        std::cmp::Reverse(*index),
                    )
                })
                .map(|(index, _)| index)?;
            candidates[index].credit -= total;
            index
        }
    };
    let chosen = &mut candidates[index];
    chosen.last_sequence = sequence;
    chosen.load += 1;
    Some(index)
}

pub(crate) fn simulate(
    strategy: DistributionStrategy,
    mut candidates: Vec<Candidate>,
    start_sequence: u64,
    count: usize,
) -> Vec<UserId> {
    let mut picks = Vec::with_capacity(count);
    for offset in 0..count as u64 {
        match pick(strategy, &mut candidates, start_sequence + offset + 1) {
            Some(index) => picks.push(candidates[index].user_id.clone()),
            None => break,
        }
    }
    picks
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RuleStats {
    pub(crate) sequence: u64,
    pub(crate) last_user_id: Option<UserId>,
    pub(crate) last_assigned_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DistributionRule {
    pub id: Uuid,
    pub(crate) key: String,
    pub name: String,
    pub(crate) description: Option<String>,
    pub(crate) record_type: String,
    pub strategy: DistributionStrategy,
    pub is_active: bool,
    pub members: Vec<RuleMember>,
    pub(crate) routing: Option<Routing>,
    pub(crate) created_by: Option<UserId>,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) updated_at: DateTime<Utc>,
    pub(crate) stats: RuleStats,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserStatus {
    pub(crate) user_id: UserId,
    pub(crate) name: String,
    pub(crate) active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Counter {
    pub user_id: UserId,
    pub last_sequence: u64,
    pub credit: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignmentSource {
    Automation,
    IncomingWebhook,
    Import,
    Scheduling,
    LeadForm,
    Plugin,
}

impl AssignmentSource {
    pub(crate) const ALL: [Self; 6] = [
        Self::Automation,
        Self::IncomingWebhook,
        Self::Import,
        Self::Scheduling,
        Self::LeadForm,
        Self::Plugin,
    ];

    pub(crate) fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|source| source.as_str() == value)
    }

    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Automation => "automation",
            Self::IncomingWebhook => "incoming_webhook",
            Self::Import => "import",
            Self::Scheduling => "scheduling",
            Self::LeadForm => "lead_form",
            Self::Plugin => "plugin",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AssignmentEntry {
    pub(crate) id: Uuid,
    pub(crate) rule_id: Uuid,
    pub(crate) rule_key: String,
    pub(crate) rule_name: String,
    pub(crate) user_id: Option<UserId>,
    pub(crate) user_name: Option<String>,
    pub(crate) record_type: Option<String>,
    pub(crate) record_id: Option<String>,
    pub(crate) source: AssignmentSource,
    pub(crate) created_at: DateTime<Utc>,
}
