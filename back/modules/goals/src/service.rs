use aphel_contract_crm::CrmPermissions;
use std::collections::BTreeSet;
use std::str::FromStr;

use chrono::{Days, NaiveDate};
use rust_decimal::Decimal;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::domain::{
    CreateGoalCommand, DATE_FORMAT, GOAL_ALREADY_EXISTS, GOAL_CREATED, GOAL_DELETED,
    GOAL_NOT_FOUND, GOAL_RECORD_TYPE, GOAL_UPDATED, Goal, GoalCreditMode, GoalListFilter,
    GoalListQuery, GoalMetric, GoalOwner, GoalOwnerInput, GoalOwnerKind, GoalOwnerOptions,
    GoalPeriod, GoalProgress, GoalView, GoalVisibility, MAX_GOAL_TARGET, UpdateGoalCommand,
    progress_of,
};
use crate::store::GoalStore;
use ::dashboard::engine_domain::local_today;
use aphel_kernel::Ctx;
use aphel_kernel::application::access::{Authorizer, PERMISSION_DENIED};
use aphel_kernel::application::audit::{DiffOptions, diff_values};
use aphel_kernel::application::organization_admin::{ListWindow, Listing};
use aphel_kernel::application::query::{
    AggregateEngine, AggregateFunction, AggregatePeriod, AggregateQuery, COUNT_METRIC, Filter,
    FilterOperator, MetricSpec, OWNER_TEAM_FIELD, RecordQueryExecutor, RecordQueryPorts,
    TimeGranularity, bucket_start, next_bucket,
};
use aphel_kernel::application::shared::{RecordRef, ensure_version};
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::access::{AccessScope, Permission, RecordScope};
use aphel_kernel::domain::{TeamId, UserId};
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_kernel::records::{RecordMutation, publish_mutation};
use currencies::domain::BASE_AMOUNT_FIELD;
use deals::team::domain::CreditedUsers;
use deals::team::service::credited_won_total;

use aphel_kernel::application::shared::is_access_denied as is_unavailable;

const INVALID_GOAL: &str = "invalid goal";

pub(crate) fn goal_not_found() -> AppError {
    AppError::not_found_code(GOAL_NOT_FOUND, "goal was not found")
}

fn cannot_manage() -> AppError {
    AppError::forbidden_code(
        PERMISSION_DENIED,
        "goals.manage does not cover this goal owner",
    )
}

enum Manager {
    Everything,
    Teams {
        user_ids: BTreeSet<UserId>,
        team_ids: BTreeSet<TeamId>,
    },
    Own(UserId),
}

impl Manager {
    fn allows(&self, owner: &GoalOwner) -> bool {
        match (self, owner) {
            (Self::Everything, _) => true,
            (Self::Teams { user_ids, .. }, GoalOwner::User(user_id)) => user_ids.contains(user_id),
            (Self::Teams { team_ids, .. }, GoalOwner::Team(team_id)) => team_ids.contains(team_id),
            (Self::Own(own), GoalOwner::User(user_id)) => own == user_id,
            _ => false,
        }
    }
}

struct MetricSource {
    record_type: &'static str,
    status: Option<&'static str>,
    period_field: &'static str,
    metric: MetricSpec,
}

fn metric_source(metric: GoalMetric) -> MetricSource {
    match metric {
        GoalMetric::WonAmount => MetricSource {
            record_type: "deals.deal",
            status: Some("won"),
            period_field: "closed_at",
            metric: MetricSpec::of(AggregateFunction::Sum, BASE_AMOUNT_FIELD),
        },
        GoalMetric::WonCount => MetricSource {
            record_type: "deals.deal",
            status: Some("won"),
            period_field: "closed_at",
            metric: MetricSpec::count(),
        },
        GoalMetric::ActivitiesDone => MetricSource {
            record_type: "activities.activity",
            status: Some("done"),
            period_field: "completed_at",
            metric: MetricSpec::count(),
        },
        GoalMetric::NewContacts => MetricSource {
            record_type: "people.person",
            status: None,
            period_field: "created_at",
            metric: MetricSpec::count(),
        },
    }
}

fn goal_query(goal: &Goal) -> (AggregateQuery, String) {
    let source = metric_source(goal.metric);
    let mut conditions = Vec::new();
    if let Some(status) = source.status {
        conditions.push(Filter::condition(
            "status",
            FilterOperator::Eq,
            Value::String(status.to_owned()),
        ));
    }
    match &goal.owner {
        GoalOwner::Organization => {}
        GoalOwner::User(user_id) => conditions.push(Filter::condition(
            "owner_id",
            FilterOperator::In,
            json!([user_id.as_str()]),
        )),
        GoalOwner::Team(team_id) => conditions.push(Filter::condition(
            OWNER_TEAM_FIELD,
            FilterOperator::In,
            json!([team_id.as_str()]),
        )),
    }
    let key = source.metric.key();
    let mut query = AggregateQuery::new(source.record_type)
        .metrics(vec![source.metric])
        .period(AggregatePeriod::between(
            source.period_field,
            &goal.starts_on.format(DATE_FORMAT).to_string(),
            &goal.ends_on.format(DATE_FORMAT).to_string(),
        ));
    if !conditions.is_empty() {
        query = query.filter(Filter::and(conditions));
    }
    (query, key)
}

fn split_credit_owner(goal: &Goal) -> Option<CreditedUsers> {
    if goal.credit_mode != GoalCreditMode::Split || goal.metric != GoalMetric::WonAmount {
        return None;
    }
    match &goal.owner {
        GoalOwner::Organization => None,
        GoalOwner::User(user_id) => Some(CreditedUsers::User(user_id.clone())),
        GoalOwner::Team(team_id) => Some(CreditedUsers::Team(team_id.clone())),
    }
}

pub(crate) async fn goal_progress(
    ctx: &Ctx<'_>,
    goal: &Goal,
    today: NaiveDate,
) -> Result<Option<GoalProgress>, AppError> {
    let scope = ctx.scope;
    let query_ports = ctx.query();
    if let Some(credited) = split_credit_owner(goal) {
        let amount_key = metric_source(goal.metric)
            .metric
            .field
            .clone()
            .unwrap_or_default();
        return match credited_won_total(ctx, &credited, &amount_key, goal.starts_on, goal.ends_on)
            .await
        {
            Ok(totals) => Ok(Some(progress_of(goal, totals.amount, today))),
            Err(error) if is_unavailable(&error) => Ok(None),
            Err(error) => Err(error),
        };
    }
    let (query, key) = goal_query(goal);
    match AggregateEngine::new(&query_ports).run(scope, query).await {
        Ok(result) => {
            let actual = result
                .total(&key)
                .or_else(|| (key == COUNT_METRIC).then_some(Decimal::ZERO))
                .unwrap_or_default();
            Ok(Some(progress_of(goal, actual, today)))
        }
        Err(error) if is_unavailable(&error) => Ok(None),
        Err(error) => Err(error),
    }
}

fn snapshot(goal: &Goal) -> Value {
    json!({
        "owner_type": goal.owner.kind().as_str(),
        "user_id": goal.owner.user_id().map(UserId::as_str),
        "team_id": goal.owner.team_id().map(TeamId::as_str),
        "metric": goal.metric.as_str(),
        "period": goal.period.as_str(),
        "starts_on": goal.starts_on.format(DATE_FORMAT).to_string(),
        "ends_on": goal.ends_on.format(DATE_FORMAT).to_string(),
        "target": format!("{:.2}", goal.target),
        "credit_mode": goal.credit_mode.as_str(),
    })
}

fn granularity(period: GoalPeriod) -> TimeGranularity {
    match period {
        GoalPeriod::Month => TimeGranularity::Month,
        GoalPeriod::Quarter => TimeGranularity::Quarter,
        GoalPeriod::Year => TimeGranularity::Year,
    }
}

pub(crate) fn period_bounds(period: GoalPeriod, day: NaiveDate) -> (NaiveDate, NaiveDate) {
    let granularity = granularity(period);
    let start = bucket_start(day, granularity);
    let end = next_bucket(start, granularity)
        .and_then(|next| next.checked_sub_days(Days::new(1)))
        .unwrap_or(start);
    (start, end)
}

fn parse_date(value: &str, path: &str, violations: &mut Vec<FieldViolation>) -> Option<NaiveDate> {
    let parsed = NaiveDate::parse_from_str(value.trim(), DATE_FORMAT).ok();
    if parsed.is_none() {
        violations.push(FieldViolation::new(
            path,
            "must be a date in the format YYYY-MM-DD",
        ));
    }
    parsed
}

fn parse_target(
    value: &str,
    metric: Option<GoalMetric>,
    violations: &mut Vec<FieldViolation>,
) -> Option<Decimal> {
    let maximum = Decimal::from_str(MAX_GOAL_TARGET).unwrap_or(Decimal::MAX);
    match Decimal::from_str(value.trim()) {
        Ok(target) if target > Decimal::ZERO && target <= maximum => {
            if metric.is_some_and(|metric| metric.is_count()) && !target.fract().is_zero() {
                violations.push(FieldViolation::new(
                    "target",
                    "must be a whole number for count metrics",
                ));
                return None;
            }
            if target.round_dp(2) != target {
                violations.push(FieldViolation::new(
                    "target",
                    "must have at most 2 decimal places",
                ));
                return None;
            }
            Some(target)
        }
        _ => {
            violations.push(FieldViolation::new(
                "target",
                format!("must be a number greater than 0 and at most {MAX_GOAL_TARGET}"),
            ));
            None
        }
    }
}

fn parse_owner(input: &GoalOwnerInput, violations: &mut Vec<FieldViolation>) -> Option<GoalOwner> {
    let id = |value: &Option<String>, path: &str, violations: &mut Vec<FieldViolation>| {
        let parsed = value
            .as_deref()
            .map(str::trim)
            .and_then(|value| Uuid::parse_str(value).ok());
        if parsed.is_none() {
            violations.push(FieldViolation::new(path, "must be a valid id"));
        }
        parsed
    };
    match input.owner_type {
        GoalOwnerKind::Organization => Some(GoalOwner::Organization),
        GoalOwnerKind::User => id(&input.user_id, "user_id", violations)
            .and_then(|id| UserId::new(id.to_string()).ok())
            .map(GoalOwner::User),
        GoalOwnerKind::Team => id(&input.team_id, "team_id", violations)
            .and_then(|id| TeamId::new(id.to_string()).ok())
            .map(GoalOwner::Team),
    }
}

pub(crate) async fn list(
    ctx: &Ctx<'_>,
    query: GoalListQuery,
    window: ListWindow,
) -> Result<Listing<GoalView>, AppError> {
    let mut violations = Vec::new();
    let owner_type = query.owner_type.map(|kind| kind.as_str());
    let optional_id = |value: &Option<String>, path: &str, violations: &mut Vec<FieldViolation>| {
        let value = value
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())?;
        let parsed = Uuid::parse_str(value).ok();
        if parsed.is_none() {
            violations.push(FieldViolation::new(path, "must be a valid id"));
        }
        parsed.map(|id| id.to_string())
    };
    let user_id =
        optional_id(&query.user_id, "user_id", &mut violations).and_then(|id| UserId::new(id).ok());
    let team_id =
        optional_id(&query.team_id, "team_id", &mut violations).and_then(|id| TeamId::new(id).ok());
    let metric = query.metric;
    let period = query.period;
    let mut active_on = query
        .active_on
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .and_then(|value| parse_date(value, "active_on", &mut violations));
    if !violations.is_empty() {
        return Err(AppError::invalid_fields("invalid goal filter", violations));
    }
    let today = today(ctx).await?;
    if query.current && active_on.is_none() {
        active_on = Some(today);
    }
    let manager = manager(ctx).await?;
    let filter = GoalListFilter {
        visibility: visibility(ctx).await?,
        owner_type,
        user_id,
        team_id,
        metric,
        period,
        active_on,
        window,
    };
    let store = GoalStore::of(ctx);
    let listing = store.list(&filter).await?;
    let currency = store.currency().await?;
    let mut items = Vec::with_capacity(listing.items.len());
    for goal in listing.items {
        items.push(
            view(
                ctx,
                goal,
                &currency,
                manager.as_ref(),
                today,
                query.with_progress,
            )
            .await?,
        );
    }
    Ok(Listing {
        items,
        total: listing.total,
    })
}

pub(crate) async fn get(ctx: &Ctx<'_>, goal_id: &str) -> Result<GoalView, AppError> {
    let goal = visible(ctx, goal_id).await?;
    let manager = manager(ctx).await?;
    let currency = GoalStore::of(ctx).currency().await?;
    let today = today(ctx).await?;
    view(ctx, goal, &currency, manager.as_ref(), today, true).await
}

pub(crate) async fn owner_options(ctx: &Ctx<'_>) -> Result<GoalOwnerOptions, AppError> {
    let store = GoalStore::of(ctx);
    Ok(match manager(ctx).await? {
        None => GoalOwnerOptions {
            organization: false,
            users: Vec::new(),
            teams: Vec::new(),
        },
        Some(Manager::Everything) => GoalOwnerOptions {
            organization: true,
            users: store.users(None).await?,
            teams: store.teams(None).await?,
        },
        Some(Manager::Teams { user_ids, team_ids }) => {
            let user_ids: Vec<UserId> = user_ids.into_iter().collect();
            let team_ids: Vec<TeamId> = team_ids.into_iter().collect();
            GoalOwnerOptions {
                organization: false,
                users: store.users(Some(&user_ids)).await?,
                teams: if team_ids.is_empty() {
                    Vec::new()
                } else {
                    store.teams(Some(&team_ids)).await?
                },
            }
        }
        Some(Manager::Own(user_id)) => GoalOwnerOptions {
            organization: false,
            users: store.users(Some(&[user_id])).await?,
            teams: Vec::new(),
        },
    })
}

pub(crate) async fn create(
    ctx: &Ctx<'_>,
    command: CreateGoalCommand,
) -> Result<GoalView, AppError> {
    let scope = ctx.scope;
    let manager = require_manager(ctx).await?;
    let mut violations = Vec::new();
    let owner = parse_owner(&command.owner, &mut violations);
    let metric = command.metric;
    let period = command.period;
    let starts_on = parse_date(&command.starts_on, "starts_on", &mut violations);
    let target = parse_target(&command.target, Some(metric), &mut violations);
    let credit_mode = command.credit_mode.unwrap_or(GoalCreditMode::Owner);
    let (Some(owner), Some(starts_on), Some(target)) = (owner, starts_on, target) else {
        return Err(AppError::invalid_fields(INVALID_GOAL, violations));
    };
    ensure_owner(ctx, &manager, &owner).await?;
    let (starts_on, ends_on) = period_bounds(period, starts_on);
    let now = ctx.now();
    let goal = Goal {
        id: Uuid::now_v7(),
        owner,
        owner_name: None,
        metric,
        period,
        starts_on,
        ends_on,
        target,
        credit_mode,
        created_by: Some(scope.user_id().clone()),
        created_at: now,
        updated_at: now,
    };
    ensure_unique(ctx, &goal).await?;
    let mut tx = ctx.begin().await?;
    GoalStore::of(ctx).insert(&mut tx, &goal).await?;
    publish(
        ctx,
        &mut tx,
        &goal,
        GOAL_CREATED,
        (&Value::Null, &snapshot(&goal)),
        command.ip,
    )
    .await?;
    tx.commit().await?;
    get(ctx, &goal.id.to_string()).await
}

pub(crate) async fn update(
    ctx: &Ctx<'_>,
    goal_id: &str,
    command: UpdateGoalCommand,
) -> Result<GoalView, AppError> {
    let manager = require_manager(ctx).await?;
    let current = visible(ctx, goal_id).await?;
    if !manager.allows(&current.owner) {
        return Err(cannot_manage());
    }
    ensure_version(command.expected_updated_at, current.updated_at)?;
    let mut goal = current.clone();
    let mut violations = Vec::new();
    if let Some(owner) = &command.owner
        && let Some(owner) = parse_owner(owner, &mut violations)
    {
        goal.owner = owner;
    }
    if let Some(metric) = command.metric {
        goal.metric = metric;
    }
    if let Some(period) = command.period {
        goal.period = period;
    }
    if let Some(credit_mode) = command.credit_mode {
        goal.credit_mode = credit_mode;
    }
    let mut anchor = goal.starts_on;
    if let Some(starts_on) = &command.starts_on
        && let Some(starts_on) = parse_date(starts_on, "starts_on", &mut violations)
    {
        anchor = starts_on;
    }
    (goal.starts_on, goal.ends_on) = period_bounds(goal.period, anchor);
    match &command.target {
        Some(target) => {
            if let Some(target) = parse_target(target, Some(goal.metric), &mut violations) {
                goal.target = target;
            }
        }
        None if goal.metric.is_count() && !goal.target.fract().is_zero() => {
            violations.push(FieldViolation::new(
                "target",
                "must be a whole number for count metrics",
            ));
        }
        None => {}
    }
    if !violations.is_empty() {
        return Err(AppError::invalid_fields(INVALID_GOAL, violations));
    }
    let before = snapshot(&current);
    let after = snapshot(&goal);
    if before != after {
        if goal.owner != current.owner {
            ensure_owner(ctx, &manager, &goal.owner).await?;
        }
        ensure_unique(ctx, &goal).await?;
        goal.updated_at = ctx.now();
        let mut tx = ctx.begin().await?;
        GoalStore::of(ctx).update(&mut tx, &goal).await?;
        publish(
            ctx,
            &mut tx,
            &goal,
            GOAL_UPDATED,
            (&before, &after),
            command.ip,
        )
        .await?;
        tx.commit().await?;
    }
    get(ctx, &goal.id.to_string()).await
}

pub(crate) async fn delete(
    ctx: &Ctx<'_>,
    goal_id: &str,
    ip: Option<String>,
) -> Result<(), AppError> {
    let manager = require_manager(ctx).await?;
    let goal = visible(ctx, goal_id).await?;
    if !manager.allows(&goal.owner) {
        return Err(cannot_manage());
    }
    let mut tx = ctx.begin().await?;
    GoalStore::of(ctx).delete(&mut tx, goal.id).await?;
    publish(
        ctx,
        &mut tx,
        &goal,
        GOAL_DELETED,
        (&snapshot(&goal), &Value::Null),
        ip,
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

pub(crate) async fn purge_orphans(ctx: &Ctx<'_>) -> Result<usize, AppError> {
    let store = GoalStore::of(ctx);
    let mut tx = ctx.begin().await?;
    let orphans = store.lock_orphans(&mut tx).await?;
    for orphan in &orphans {
        store.delete(&mut tx, orphan.id).await?;
        let before = json!({
            "owner_type": orphan.owner_type,
            "metric": orphan.metric,
            "period": orphan.period,
            "starts_on": orphan.starts_on.format(DATE_FORMAT).to_string(),
            "target": format!("{:.2}", orphan.target),
        });
        publish_mutation(
            ctx.events(),
            ctx.audit(),
            &mut tx,
            ctx.scope,
            ctx.now(),
            RecordMutation {
                record: RecordRef::new(GOAL_RECORD_TYPE, orphan.id),
                event_type: GOAL_DELETED,
                payload: json!({
                    "owner_type": orphan.owner_type,
                    "user_id": Value::Null,
                    "team_id": Value::Null,
                    "metric": orphan.metric,
                    "period": orphan.period,
                    "starts_on": orphan.starts_on.format(DATE_FORMAT).to_string(),
                    "target": format!("{:.2}", orphan.target),
                    "reason": "owner_removed",
                }),
                changes: diff_values(&before, &Value::Null, &DiffOptions::default())?,
                ip: None,
            },
        )
        .await?;
    }
    tx.commit().await?;
    Ok(orphans.len())
}

async fn view(
    ctx: &Ctx<'_>,
    goal: Goal,
    currency: &str,
    manager: Option<&Manager>,
    today: NaiveDate,
    with_progress: bool,
) -> Result<GoalView, AppError> {
    let progress = if with_progress {
        goal_progress(ctx, &goal, today).await?
    } else {
        None
    };
    Ok(GoalView {
        can_manage: manager.is_some_and(|manager| manager.allows(&goal.owner)),
        currency: (goal.metric == GoalMetric::WonAmount).then(|| currency.to_owned()),
        progress,
        goal,
    })
}

async fn today(ctx: &Ctx<'_>) -> Result<NaiveDate, AppError> {
    let timezone = ctx.query().executor().timezone().await?;
    Ok(local_today(ctx.now(), timezone))
}

async fn manager(ctx: &Ctx<'_>) -> Result<Option<Manager>, AppError> {
    let scope: &AccessScope = ctx.scope;
    Ok(match scope.scope_for(&Permission::GOALS_MANAGE) {
        None => None,
        Some(RecordScope::Organization) => Some(Manager::Everything),
        Some(RecordScope::Team) => {
            let team_ids: BTreeSet<TeamId> = scope.team_ids().iter().cloned().collect();
            let mut user_ids: BTreeSet<UserId> = GoalStore::of(ctx)
                .team_members(scope.team_ids())
                .await?
                .into_iter()
                .collect();
            user_ids.insert(scope.user_id().clone());
            Some(Manager::Teams { user_ids, team_ids })
        }
        Some(RecordScope::Own) => Some(Manager::Own(scope.user_id().clone())),
    })
}

async fn require_manager(ctx: &Ctx<'_>) -> Result<Manager, AppError> {
    ctx.scope.require(&Permission::GOALS_MANAGE)?;
    manager(ctx).await?.ok_or_else(cannot_manage)
}

async fn visibility(ctx: &Ctx<'_>) -> Result<GoalVisibility, AppError> {
    let scope = ctx.scope;
    Ok(match manager(ctx).await? {
        Some(Manager::Everything) => GoalVisibility::All,
        Some(Manager::Teams { user_ids, .. }) => GoalVisibility::Limited {
            user_ids: user_ids.into_iter().collect(),
            team_ids: scope.team_ids().to_vec(),
        },
        _ => GoalVisibility::Limited {
            user_ids: vec![scope.user_id().clone()],
            team_ids: scope.team_ids().to_vec(),
        },
    })
}

async fn visible(ctx: &Ctx<'_>, goal_id: &str) -> Result<Goal, AppError> {
    let id = Uuid::parse_str(goal_id.trim()).map_err(|_| goal_not_found())?;
    let goal = GoalStore::of(ctx)
        .find(id)
        .await?
        .ok_or_else(goal_not_found)?;
    let visible = match visibility(ctx).await? {
        GoalVisibility::All => true,
        GoalVisibility::Limited { user_ids, team_ids } => match &goal.owner {
            GoalOwner::Organization => true,
            GoalOwner::User(user_id) => user_ids.contains(user_id),
            GoalOwner::Team(team_id) => team_ids.contains(team_id),
        },
    };
    if visible {
        Ok(goal)
    } else {
        Err(goal_not_found())
    }
}

async fn ensure_owner(ctx: &Ctx<'_>, manager: &Manager, owner: &GoalOwner) -> Result<(), AppError> {
    if !manager.allows(owner) {
        return Err(cannot_manage());
    }
    let store = GoalStore::of(ctx);
    let exists = match owner {
        GoalOwner::Organization => true,
        GoalOwner::User(user_id) => !store
            .users(Some(std::slice::from_ref(user_id)))
            .await?
            .is_empty(),
        GoalOwner::Team(team_id) => !store
            .teams(Some(std::slice::from_ref(team_id)))
            .await?
            .is_empty(),
    };
    if exists {
        Ok(())
    } else {
        let field = match owner {
            GoalOwner::Team(_) => "team_id",
            _ => "user_id",
        };
        Err(AppError::invalid_fields(
            INVALID_GOAL,
            vec![FieldViolation::new(
                field,
                "is not an active member or team of the organization",
            )],
        ))
    }
}

async fn ensure_unique(ctx: &Ctx<'_>, goal: &Goal) -> Result<(), AppError> {
    match GoalStore::of(ctx).find_duplicate(goal).await? {
        Some(existing) if existing != goal.id => Err(AppError::conflict_code(
            GOAL_ALREADY_EXISTS,
            "a goal with this owner, metric and period already exists",
        )),
        _ => Ok(()),
    }
}

async fn publish(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    goal: &Goal,
    event_type: &'static str,
    (before, after): (&Value, &Value),
    ip: Option<String>,
) -> Result<(), AppError> {
    let changes = diff_values(before, after, &DiffOptions::default())?;
    publish_mutation(
        ctx.events(),
        ctx.audit(),
        tx,
        ctx.scope,
        ctx.now(),
        RecordMutation {
            record: RecordRef::new(GOAL_RECORD_TYPE, goal.id),
            event_type,
            payload: json!({
                "owner_type": goal.owner.kind().as_str(),
                "user_id": goal.owner.user_id().map(UserId::as_str),
                "team_id": goal.owner.team_id().map(TeamId::as_str),
                "metric": goal.metric.as_str(),
                "period": goal.period.as_str(),
                "starts_on": goal.starts_on.format(DATE_FORMAT).to_string(),
                "target": format!("{:.2}", goal.target),
            }),
            changes,
            ip,
        },
    )
    .await
}
