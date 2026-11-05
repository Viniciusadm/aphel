use std::collections::{HashMap, HashSet};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::{Map, Value};
use uuid::Uuid;

use crate::domain::{
    AssignmentSource, Candidate, Counter, DistributionRule, DistributionStrategy, Ineligibility,
    MIN_WEIGHT, RouteSelection, Routing, pick, select_route, simulate,
};
use crate::store::{DistributionStore, PickRecord};
use aphel_contract_automation::application::AutomationGateway;
use aphel_contract_automation::application::templates::CUSTOM_FIELDS_KEY;
use aphel_kernel::Ctx;
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;
use aphel_kernel::records::PendingCommit;
use aphel_sdk::platform::application::shared::RecordRef;
use aphel_sdk::platform::database::transaction::MySqlTx;
use aphel_sdk::platform::domain::access::AccessScope;

pub const RULE_NOT_FOUND: &str = "assignment_rule_not_found";
pub(crate) const RULE_INACTIVE: &str = "assignment_rule_inactive";
pub(crate) const RECORD_TYPE_MISMATCH: &str = "assignment_record_type_mismatch";
pub(crate) const NO_ELIGIBLE_MEMBER: &str = "assignment_no_eligible_member";
pub(crate) const MAX_SIMULATION: usize = 50;

pub(crate) fn rule_not_found() -> AppError {
    AppError::not_found_code(RULE_NOT_FOUND, "lead distribution rule was not found")
}

pub(crate) fn parse_rule_id(value: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(value.trim()).map_err(|_| rule_not_found())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoolMember {
    pub user_id: UserId,
    pub(crate) name: String,
    pub weight: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkippedMember {
    pub(crate) user_id: UserId,
    pub(crate) name: Option<String>,
    pub(crate) reason: Ineligibility,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pool {
    pub(crate) selection: RouteSelection,
    pub members: Vec<PoolMember>,
    pub(crate) skipped: Vec<SkippedMember>,
}

impl Pool {
    fn name_of(&self, user_id: &UserId) -> String {
        self.members
            .iter()
            .find(|member| &member.user_id == user_id)
            .map(|member| member.name.clone())
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recipient {
    pub(crate) user_id: UserId,
    pub(crate) name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Simulation {
    pub(crate) selection: RouteSelection,
    pub(crate) recipients: Vec<Recipient>,
    pub(crate) skipped: Vec<SkippedMember>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleStatus {
    pub(crate) name: String,
    pub record_type: String,
    pub is_active: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipCause {
    RuleInactive,
    Unrouted,
    NoEligibleMember,
}

impl SkipCause {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::RuleInactive => "rule_inactive",
            Self::Unrouted => "no_route",
            Self::NoEligibleMember => "no_eligible_member",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssignOutcome {
    Assigned {
        recipient: Recipient,
        selection: RouteSelection,
        reused: bool,
    },
    Skipped(SkipCause),
}

impl AssignOutcome {
    pub fn user_id(&self) -> Option<&UserId> {
        match self {
            Self::Assigned { recipient, .. } => Some(&recipient.user_id),
            Self::Skipped(_) => None,
        }
    }
}

pub struct AssignRequest<'a> {
    pub rule_id: &'a str,
    pub record_type: &'a str,
    pub record: Option<RecordRef>,
    pub values: &'a Map<String, Value>,
    pub source: AssignmentSource,
    pub idempotency_key: Option<String>,
}

pub(crate) fn flatten_values(record: &Value) -> Map<String, Value> {
    let mut values = record.as_object().cloned().unwrap_or_default();
    if let Some(Value::Object(custom)) = values.remove(CUSTOM_FIELDS_KEY) {
        for (key, value) in custom {
            values.entry(key).or_insert(value);
        }
    }
    values
}

fn scalar_texts(value: &Value, texts: &mut Vec<String>) {
    match value {
        Value::String(text) => texts.push(text.trim().to_lowercase()),
        Value::Number(number) => texts.push(number.to_string()),
        Value::Bool(flag) => texts.push(flag.to_string()),
        Value::Array(items) => {
            for item in items {
                scalar_texts(item, texts);
            }
        }
        Value::Object(object) => {
            for key in ["value", "id", "key"] {
                if let Some(inner) = object.get(key) {
                    scalar_texts(inner, texts);
                }
            }
        }
        Value::Null => {}
    }
}

pub(crate) fn routing_values(
    routing: Option<&Routing>,
    values: &Map<String, Value>,
) -> Vec<String> {
    let mut texts = Vec::new();
    if let Some(value) = routing.and_then(|routing| values.get(&routing.field)) {
        scalar_texts(value, &mut texts);
    }
    texts.retain(|text| !text.is_empty());
    texts
}

pub async fn resolve_pool(
    store: &DistributionStore,
    rule: &DistributionRule,
    values: &Map<String, Value>,
    at: DateTime<Utc>,
) -> Result<Pool, AppError> {
    let selection = select_route(
        rule.routing.as_ref(),
        &routing_values(rule.routing.as_ref(), values),
    );
    let mut user_ids: Vec<UserId> = match selection {
        RouteSelection::Members | RouteSelection::Fallback => rule
            .members
            .iter()
            .map(|member| member.user_id.clone())
            .collect(),
        RouteSelection::Route(index) => {
            let route = rule
                .routing
                .as_ref()
                .and_then(|routing| routing.routes.get(index))
                .ok_or_else(|| AppError::Internal("route index out of bounds".to_owned()))?;
            let mut ids = route.user_ids.clone();
            if let Some(team_id) = &route.team_id {
                ids.extend(store.team_members(team_id).await?);
            }
            ids
        }
        RouteSelection::Unassigned => Vec::new(),
    };
    let mut seen = HashSet::new();
    user_ids.retain(|id| seen.insert(id.clone()));
    let statuses: HashMap<UserId, (String, bool)> = if user_ids.is_empty() {
        HashMap::new()
    } else {
        store
            .user_statuses(&user_ids)
            .await?
            .into_iter()
            .map(|status| (status.user_id, (status.name, status.active)))
            .collect()
    };
    let mut pool = Pool {
        selection,
        members: Vec::new(),
        skipped: Vec::new(),
    };
    for user_id in user_ids {
        let member = rule.members.iter().find(|member| member.user_id == user_id);
        let status = statuses.get(&user_id);
        let name = status.map(|(name, _)| name.clone());
        let reason = match status {
            Some((_, true)) if member.is_some_and(|member| member.is_away_at(at)) => {
                Some(Ineligibility::Away)
            }
            Some((_, true)) => None,
            _ => Some(Ineligibility::Inactive),
        };
        match reason {
            Some(reason) => pool.skipped.push(SkippedMember {
                user_id,
                name,
                reason,
            }),
            None => pool.members.push(PoolMember {
                weight: member.map_or(MIN_WEIGHT, |member| member.weight),
                name: name.unwrap_or_default(),
                user_id,
            }),
        }
    }
    Ok(pool)
}

pub async fn workload_counts(
    ctx: &Ctx<'_>,
    rule: &DistributionRule,
    users: &[UserId],
) -> Result<HashMap<UserId, u64>, AppError> {
    use aphel_sdk::assignment::AssignmentTraits;
    use aphel_sdk::platform::application::query::{
        AggregateEngine, AggregateQuery, Filter, FilterOperator, GroupBySpec,
    };
    use rust_decimal::prelude::ToPrimitive;
    if rule.strategy != DistributionStrategy::LeastLoaded || users.is_empty() {
        return Ok(HashMap::new());
    }
    let definition = ctx.registry().require(&rule.record_type)?;
    let column = definition
        .owner_column
        .as_deref()
        .ok_or_else(|| AppError::Validation("the assignment target has no owner".into()))?;
    let field = definition
        .system_fields
        .iter()
        .find(|field| field.column == column)
        .map(|field| field.key.as_str())
        .unwrap_or(column);
    let mut conditions = vec![Filter::condition(
        field,
        FilterOperator::In,
        serde_json::json!(users.iter().map(|user| user.as_str()).collect::<Vec<_>>()),
    )];
    if let Some(mapping) = definition.capabilities.workload_mapping()?
        && !mapping.active_states.is_empty()
    {
        conditions.push(Filter::condition(
            mapping.state_field,
            FilterOperator::In,
            serde_json::json!(mapping.active_states),
        ));
    }
    let mut query = AggregateQuery::new(&rule.record_type)
        .filter(Filter::and(conditions))
        .group(GroupBySpec::field(field));
    query.limit = Some(users.len() as u32);
    let result = AggregateEngine::new(&ctx.query())
        .run(&AccessScope::system(), query)
        .await?;
    if result.truncated {
        return Err(AppError::Configuration(
            "assignment workload aggregation was truncated".into(),
        ));
    }
    result
        .rows
        .into_iter()
        .filter_map(|row| {
            row.groups
                .first()
                .and_then(|group| group.value.clone())
                .map(|id| {
                    (
                        id,
                        row.values
                            .first()
                            .copied()
                            .flatten()
                            .and_then(|value| value.to_u64())
                            .unwrap_or_default(),
                    )
                })
        })
        .map(|(id, count)| {
            Ok((
                UserId::new(id)
                    .map_err(|_| AppError::Internal("invalid workload owner ID".into()))?,
                count,
            ))
        })
        .collect()
}

fn candidates(pool: &Pool, counters: Vec<Counter>, loads: &HashMap<UserId, u64>) -> Vec<Candidate> {
    let counters: HashMap<UserId, Counter> = counters
        .into_iter()
        .map(|counter| (counter.user_id.clone(), counter))
        .collect();
    pool.members
        .iter()
        .map(|member| {
            let counter = counters.get(&member.user_id);
            Candidate {
                user_id: member.user_id.clone(),
                weight: member.weight,
                last_sequence: counter.map_or(0, |counter| counter.last_sequence),
                credit: counter.map_or(0, |counter| counter.credit),
                load: loads.get(&member.user_id).copied().unwrap_or(0),
            }
        })
        .collect()
}

pub(crate) async fn simulate_rule(
    ctx: &Ctx<'_>,
    rule: &DistributionRule,
    values: &Map<String, Value>,
    count: usize,
) -> Result<Simulation, AppError> {
    let store = DistributionStore::of(ctx);
    let pool = resolve_pool(&store, rule, values, ctx.now()).await?;
    let users: Vec<UserId> = pool
        .members
        .iter()
        .map(|member| member.user_id.clone())
        .collect();
    let recipients = if users.is_empty() {
        Vec::new()
    } else {
        let counters = store.counters(rule.id, &users).await?;
        let loads = workload_counts(ctx, rule, &users).await?;
        simulate(
            rule.strategy,
            candidates(&pool, counters, &loads),
            rule.stats.sequence,
            count.min(MAX_SIMULATION),
        )
        .into_iter()
        .map(|user_id| Recipient {
            name: pool.name_of(&user_id),
            user_id,
        })
        .collect()
    };
    Ok(Simulation {
        selection: pool.selection,
        recipients,
        skipped: pool.skipped,
    })
}

async fn find_rule(store: &DistributionStore, rule_id: &str) -> Result<DistributionRule, AppError> {
    store
        .find(parse_rule_id(rule_id)?)
        .await?
        .ok_or_else(rule_not_found)
}

pub async fn check(ctx: &Ctx<'_>, rule_id: &str) -> Result<Option<RuleStatus>, AppError> {
    let Ok(id) = parse_rule_id(rule_id) else {
        return Ok(None);
    };
    Ok(DistributionStore::of(ctx)
        .find(id)
        .await?
        .map(|rule| RuleStatus {
            name: rule.name,
            record_type: rule.record_type,
            is_active: rule.is_active,
        }))
}

pub(crate) async fn preview(
    ctx: &Ctx<'_>,
    rule_id: &str,
    values: &Map<String, Value>,
) -> Result<Simulation, AppError> {
    let rule = find_rule(&DistributionStore::of(ctx), rule_id).await?;
    simulate_rule(ctx, &rule, values, 1).await
}

#[async_trait]
pub trait OwnerWriter: Send + Sync {
    async fn write(
        &self,
        tx: &mut MySqlTx,
        user_id: UserId,
    ) -> Result<Option<PendingCommit>, AppError>;
}

pub struct PickOnly;

#[async_trait]
impl OwnerWriter for PickOnly {
    async fn write(
        &self,
        _tx: &mut MySqlTx,
        _user_id: UserId,
    ) -> Result<Option<PendingCommit>, AppError> {
        Ok(None)
    }
}

async fn commit(
    ctx: &Ctx<'_>,
    tx: MySqlTx,
    pending: Option<PendingCommit>,
) -> Result<(), AppError> {
    tx.commit().await?;
    if let Some(pending) = pending {
        pending.committed(ctx).await;
    }
    Ok(())
}

pub async fn assign(
    ctx: &Ctx<'_>,
    request: AssignRequest<'_>,
    owner: &dyn OwnerWriter,
) -> Result<AssignOutcome, AppError> {
    let rule_id = parse_rule_id(request.rule_id)?;
    let store = DistributionStore::of(ctx);
    let mut tx = ctx.begin().await?;
    let sequence = store
        .lock_state(&mut tx, rule_id)
        .await?
        .ok_or_else(rule_not_found)?;
    let rule = store.find(rule_id).await?.ok_or_else(rule_not_found)?;
    if rule.record_type != request.record_type {
        return Err(AppError::validation_code(
            RECORD_TYPE_MISMATCH,
            format!(
                "the rule distributes {} records, not {}",
                rule.record_type, request.record_type
            ),
        ));
    }
    if let Some(key) = &request.idempotency_key
        && let Some(user_id) = store.find_assignment(&mut tx, key).await?
    {
        let name = store
            .user_statuses(std::slice::from_ref(&user_id))
            .await?
            .into_iter()
            .next()
            .map(|status| status.name)
            .unwrap_or_default();
        let pending = owner.write(&mut tx, user_id.clone()).await?;
        commit(ctx, tx, pending).await?;
        return Ok(AssignOutcome::Assigned {
            recipient: Recipient { user_id, name },
            selection: RouteSelection::Members,
            reused: true,
        });
    }
    if !rule.is_active {
        return Ok(AssignOutcome::Skipped(SkipCause::RuleInactive));
    }
    let now = ctx.now();
    let pool = resolve_pool(&store, &rule, request.values, now).await?;
    if pool.selection == RouteSelection::Unassigned {
        return Ok(AssignOutcome::Skipped(SkipCause::Unrouted));
    }
    let users: Vec<UserId> = pool
        .members
        .iter()
        .map(|member| member.user_id.clone())
        .collect();
    if users.is_empty() {
        return Ok(AssignOutcome::Skipped(SkipCause::NoEligibleMember));
    }
    let counters = store.locked_counters(&mut tx, rule_id, &users).await?;
    let loads = workload_counts(ctx, &rule, &users).await?;
    let mut candidates = candidates(&pool, counters, &loads);
    let next = sequence + 1;
    let index = pick(rule.strategy, &mut candidates, next)
        .ok_or(AppError::Internal("no candidate was picked".to_owned()))?;
    let chosen = candidates[index].user_id.clone();
    let pending = owner.write(&mut tx, chosen.clone()).await?;
    let credits = if rule.strategy == DistributionStrategy::Weighted {
        candidates
            .iter()
            .map(|candidate| (candidate.user_id.clone(), candidate.credit))
            .collect()
    } else {
        Vec::new()
    };
    store
        .record_pick(
            &mut tx,
            &PickRecord {
                rule_id,
                sequence: next,
                user_id: chosen.clone(),
                credits,
                record: request.record,
                source: request.source,
                idempotency_key: request.idempotency_key,
                at: now,
            },
        )
        .await?;
    commit(ctx, tx, pending).await?;
    Ok(AssignOutcome::Assigned {
        recipient: Recipient {
            name: pool.name_of(&chosen),
            user_id: chosen,
        },
        selection: pool.selection,
        reused: false,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DistributionMode {
    Check,
    Preview,
    Assign { idempotency_key: String },
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DistributionRequest {
    pub(crate) rule_id: String,
    pub(crate) record: RecordRef,
    pub(crate) field: String,
    pub(crate) values: Map<String, Value>,
    pub(crate) mode: DistributionMode,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct DistributionOutcome {
    pub(crate) rule: Option<RuleStatus>,
    pub(crate) user_id: Option<String>,
    pub(crate) user_name: Option<String>,
    pub(crate) selection: Option<RouteSelection>,
    pub(crate) reused: bool,
    pub(crate) skipped: Option<SkipCause>,
}

fn recipient(recipient: Option<Recipient>) -> (Option<String>, Option<String>) {
    match recipient {
        Some(Recipient { user_id, name }) => (Some(user_id.into_inner()), Some(name)),
        None => (None, None),
    }
}

struct AutomationOwner<'a> {
    ctx: &'a Ctx<'a>,
    scope: &'a AccessScope,
    record: &'a RecordRef,
    field: &'a str,
}

#[async_trait]
impl OwnerWriter for AutomationOwner<'_> {
    async fn write(
        &self,
        tx: &mut MySqlTx,
        user_id: UserId,
    ) -> Result<Option<PendingCommit>, AppError> {
        let mut values = Map::new();
        values.insert(self.field.to_owned(), Value::String(user_id.into_inner()));
        self.ctx
            .registry()
            .handler(self.record.record_type.as_str())?
            .update_in_tx(
                &aphel_kernel::records::RecordCtx::new(self.scope, &self.ctx.state()),
                tx,
                self.record.record_id,
                Value::Object(values),
            )
            .await
    }
}

pub(crate) async fn distribute(
    ctx: &Ctx<'_>,
    _gateway: &dyn AutomationGateway,
    request: DistributionRequest,
) -> Result<DistributionOutcome, AppError> {
    let record_type = request.record.record_type.as_str().to_owned();
    match request.mode {
        DistributionMode::Check => Ok(DistributionOutcome {
            rule: check(ctx, &request.rule_id).await?,
            ..DistributionOutcome::default()
        }),
        DistributionMode::Preview => {
            let simulation = preview(ctx, &request.rule_id, &request.values).await?;
            let (user_id, user_name) = recipient(simulation.recipients.into_iter().next());
            Ok(DistributionOutcome {
                user_id,
                user_name,
                selection: Some(simulation.selection),
                ..DistributionOutcome::default()
            })
        }
        DistributionMode::Assign { idempotency_key } => {
            let owner = AutomationOwner {
                ctx,
                scope: ctx.scope,
                record: &request.record,
                field: &request.field,
            };
            let outcome = assign(
                ctx,
                AssignRequest {
                    rule_id: &request.rule_id,
                    record_type: &record_type,
                    record: Some(request.record.clone()),
                    values: &request.values,
                    source: AssignmentSource::Automation,
                    idempotency_key: Some(idempotency_key),
                },
                &owner,
            )
            .await?;
            Ok(match outcome {
                AssignOutcome::Assigned {
                    recipient: assigned,
                    selection,
                    reused,
                } => {
                    let (user_id, user_name) = recipient(Some(assigned));
                    DistributionOutcome {
                        user_id,
                        user_name,
                        selection: Some(selection),
                        reused,
                        ..DistributionOutcome::default()
                    }
                }
                AssignOutcome::Skipped(cause) => DistributionOutcome {
                    skipped: Some(cause),
                    ..DistributionOutcome::default()
                },
            })
        }
    }
}
