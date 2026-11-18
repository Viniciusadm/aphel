use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde_json::Map;
use uuid::Uuid;

use crate::domain::{Candidate, DistributionStrategy, pick};
use crate::{
    AssignmentSource, DistributionRule, DistributionStore, PickRecord, Pool, RuleListFilter,
    resolve_pool,
};
use aphel_kernel::Ctx;
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::organization_admin::ListWindow;
use aphel_sdk::platform::database::Db;
use aphel_sdk::platform::database::transaction::Tx;

fn rules(ctx: &Ctx<'_>) -> DistributionStore {
    DistributionStore::of(ctx)
}

pub(crate) async fn find_rule(
    ctx: &Ctx<'_>,
    rule_id: Uuid,
) -> Result<Option<DistributionRule>, AppError> {
    rules(ctx).find(rule_id).await
}

pub(crate) async fn active_rules(ctx: &Ctx<'_>) -> Result<Vec<DistributionRule>, AppError> {
    Ok(rules(ctx)
        .list(&RuleListFilter {
            record_type: None,
            active: Some(true),
            window: ListWindow {
                offset: 0,
                limit: 200,
            },
        })
        .await?
        .items)
}

pub(crate) async fn rule_pool(
    ctx: &Ctx<'_>,
    rule: &DistributionRule,
    at: DateTime<Utc>,
) -> Result<Pool, AppError> {
    resolve_pool(&rules(ctx), rule, &Map::new(), at).await
}

use aphel_sdk::assignment::{HostPick, WeightedHost};

pub(crate) async fn pick_user(
    ctx: &Ctx<'_>,
    tx: &mut Tx,
    rule_id: Uuid,
    hosts: &[WeightedHost],
) -> Result<HostPick, AppError> {
    let rules = rules(ctx);
    let Some(sequence) = rules.lock_state(tx, rule_id).await? else {
        return Ok(HostPick::Untracked);
    };
    let Some(rule) = rules.find(rule_id).await?.filter(|rule| rule.is_active) else {
        return Ok(HostPick::Unavailable);
    };
    let users: Vec<UserId> = hosts.iter().map(|host| host.user_id.clone()).collect();
    let counters: HashMap<UserId, (u64, i64)> = rules
        .locked_counters(tx, rule_id, &users)
        .await?
        .into_iter()
        .map(|counter| (counter.user_id, (counter.last_sequence, counter.credit)))
        .collect();
    let loads = if rule.strategy == DistributionStrategy::LeastLoaded {
        crate::workload_counts(ctx, &rule, &users).await?
    } else {
        HashMap::new()
    };
    let mut options: Vec<Candidate> = hosts
        .iter()
        .map(|host| {
            let (last_sequence, credit) = counters.get(&host.user_id).copied().unwrap_or((0, 0));
            Candidate {
                user_id: host.user_id.clone(),
                weight: host.weight,
                last_sequence,
                credit,
                load: loads.get(&host.user_id).copied().unwrap_or(0),
            }
        })
        .collect();
    let next = sequence + 1;
    let Some(index) = pick(rule.strategy, &mut options, next) else {
        return Ok(HostPick::Unavailable);
    };
    let chosen = options[index].user_id.clone();
    let credits = if rule.strategy == DistributionStrategy::Weighted {
        options
            .iter()
            .map(|option| (option.user_id.clone(), option.credit))
            .collect()
    } else {
        Vec::new()
    };
    rules
        .record_pick(
            tx,
            &PickRecord {
                rule_id,
                sequence: next,
                user_id: chosen.clone(),
                credits,
                record: None,
                source: AssignmentSource::Booking,
                idempotency_key: None,
                at: ctx.now(),
            },
        )
        .await?;
    Ok(HostPick::Picked(chosen))
}

pub(crate) struct HostAssignments;
fn snapshot(rule: DistributionRule) -> aphel_sdk::assignment::HostRule {
    aphel_sdk::assignment::HostRule {
        id: rule.id,
        name: rule.name,
        is_active: rule.is_active,
        members: rule
            .members
            .into_iter()
            .map(|member| aphel_sdk::assignment::HostRuleMember {
                user_id: member.user_id,
                away: member.away,
                away_from: member.away_from,
                away_until: member.away_until,
            })
            .collect(),
    }
}
#[async_trait::async_trait]
impl aphel_sdk::assignment::HostAssignment for HostAssignments {
    async fn rule_names(&self, pool: &Db, ids: &[Uuid]) -> Result<HashMap<Uuid, String>, AppError> {
        if ids.len() > 1000 {
            return Err(AppError::Validation(
                "at most 1000 assignment rule labels may be requested".into(),
            ));
        }
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let rows = crate::store::rule_names(pool, ids).await?;
        rows.into_iter()
            .map(|(id, name)| {
                Ok((
                    aphel_sdk::platform::infrastructure::repositories::mapper::parse_uuid(
                        &id,
                        "assignment_rules.id",
                    )?,
                    name,
                ))
            })
            .collect()
    }

    async fn find_rule(
        &self,
        ctx: &Ctx<'_>,
        id: Uuid,
    ) -> Result<Option<aphel_sdk::assignment::HostRule>, AppError> {
        Ok(find_rule(ctx, id).await?.map(snapshot))
    }
    async fn active_rules(
        &self,
        ctx: &Ctx<'_>,
    ) -> Result<Vec<aphel_sdk::assignment::HostRule>, AppError> {
        Ok(active_rules(ctx).await?.into_iter().map(snapshot).collect())
    }
    async fn rule_pool(
        &self,
        ctx: &Ctx<'_>,
        id: Uuid,
        at: DateTime<Utc>,
    ) -> Result<aphel_sdk::assignment::HostPool, AppError> {
        let Some(rule) = find_rule(ctx, id).await?.filter(|rule| rule.is_active) else {
            return Ok(aphel_sdk::assignment::HostPool {
                rule: None,
                members: Vec::new(),
            });
        };
        let members = rule_pool(ctx, &rule, at)
            .await?
            .members
            .into_iter()
            .map(|member| WeightedHost {
                user_id: member.user_id,
                weight: member.weight,
            })
            .collect();
        Ok(aphel_sdk::assignment::HostPool {
            rule: Some(snapshot(rule)),
            members,
        })
    }

    async fn pick_user(
        &self,
        ctx: &Ctx<'_>,
        tx: &mut Tx,
        id: Uuid,
        hosts: &[WeightedHost],
    ) -> Result<HostPick, AppError> {
        pick_user(ctx, tx, id, hosts).await
    }
}
