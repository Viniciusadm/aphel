use aphel_sdk::assignment::AssignmentPermissions;
use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use serde_json::{Map, Value, json};
use uuid::Uuid;

use crate::domain::{AssignmentEntry, AssignmentSource};
use crate::domain::{
    DistributionRule, DistributionStrategy, MAX_MEMBERS, MAX_ROUTE_USERS, MAX_ROUTE_VALUE_LENGTH,
    MAX_ROUTE_VALUES, MAX_ROUTES, MAX_WEIGHT, MIN_WEIGHT, Route, Routing, RoutingFallback,
    RuleMember, RuleStats, UserStatus,
};
use crate::store::{self, AssignmentFilter, DistributionStore, RuleListFilter, TargetInfo};
use aphel_kernel::Ctx;
use aphel_kernel::domain::{TeamId, UserId};
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_kernel::records::{RecordMutation, publish_mutation};
use aphel_sdk::configuration::{MAX_CONFIG_KEY_LENGTH, is_config_key, key_from_name};
use aphel_sdk::platform::application::access::Authorizer;
use aphel_sdk::platform::application::audit::{DiffOptions, diff_values};
use aphel_sdk::platform::application::organization_admin::{ListWindow, Listing};
use aphel_sdk::platform::application::shared::{RecordRef, RecordType, ensure_version};
use aphel_sdk::platform::database::transaction::Tx;
use aphel_sdk::platform::domain::access::{AccessScope, Permission};

mod distribution;

pub use distribution::*;

pub(crate) const RULE_KEY_TAKEN: &str = "assignment_rule_key_taken";
pub(crate) const RULE_CREATED: &str = "assignment_rule.created";
pub(crate) const RULE_UPDATED: &str = "assignment_rule.updated";
pub(crate) const RULE_DELETED: &str = "assignment_rule.deleted";
pub(crate) const RULE_RECORD_TYPE: RecordType =
    RecordType::from_static("assignment.assignment_rule");
pub(crate) const MAX_NAME_LENGTH: usize = 140;
pub(crate) const MAX_DESCRIPTION_LENGTH: usize = 2000;
pub(crate) const DEFAULT_SIMULATION: usize = 10;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct RuleMemberInput {
    pub(crate) user_id: String,
    pub(crate) weight: Option<u32>,
    pub(crate) away: bool,
    pub(crate) away_from: Option<String>,
    pub(crate) away_until: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct RouteInput {
    pub(crate) values: Vec<String>,
    pub(crate) team_id: Option<String>,
    pub(crate) user_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct RoutingInput {
    pub(crate) field: String,
    pub(crate) routes: Vec<RouteInput>,
    pub(crate) fallback: Option<RoutingFallback>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct SaveRuleCommand {
    pub(crate) key: Option<String>,
    pub(crate) name: Option<String>,
    pub(crate) description: Option<Option<String>>,
    pub(crate) record_type: Option<String>,
    pub(crate) strategy: Option<DistributionStrategy>,
    pub(crate) is_active: Option<bool>,
    pub(crate) members: Option<Vec<RuleMemberInput>>,
    pub(crate) routing: Option<Option<RoutingInput>>,
    pub(crate) ip: Option<String>,
    pub(crate) expected_updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct RuleListQuery {
    pub(crate) record_type: Option<String>,
    pub(crate) active: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct AssignmentListQuery {
    pub(crate) rule_id: Option<String>,
    pub(crate) user_id: Option<String>,
    pub(crate) record_type: Option<String>,
    pub(crate) source: Option<AssignmentSource>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct SimulateCommand {
    pub(crate) count: Option<usize>,
    pub(crate) values: Map<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RuleView {
    pub(crate) rule: DistributionRule,
    pub(crate) users: HashMap<UserId, UserStatus>,
    pub(crate) now: DateTime<Utc>,
    pub(crate) can_manage: bool,
}

fn allows(scope: &AccessScope, permission: &Permission) -> bool {
    scope.scope_for(permission).is_some()
}

fn snapshot(rule: &DistributionRule) -> Value {
    json!({
        "key": rule.key,
        "name": rule.name,
        "description": rule.description,
        "record_type": rule.record_type,
        "strategy": rule.strategy.as_str(),
        "is_active": rule.is_active,
        "members": rule.members.iter().map(|member| json!({
            "user_id": member.user_id.as_str(),
            "weight": member.weight,
            "away": member.away,
            "away_from": member.away_from.map(|at| at.to_rfc3339()),
            "away_until": member.away_until.map(|at| at.to_rfc3339()),
        })).collect::<Vec<_>>(),
        "routing": rule.routing.as_ref().map(|routing| json!({
            "field": routing.field,
            "fallback": routing.fallback.as_str(),
            "routes": routing.routes.iter().map(|route| json!({
                "values": route.values,
                "team_id": route.team_id.as_ref().map(TeamId::as_str),
                "user_ids": route.user_ids.iter().map(UserId::as_str).collect::<Vec<_>>(),
            })).collect::<Vec<_>>(),
        })),
    })
}

fn parse_user(value: &str, path: &str, violations: &mut Vec<FieldViolation>) -> Option<UserId> {
    let parsed = Uuid::parse_str(value.trim())
        .ok()
        .and_then(|id| UserId::new(id.to_string()).ok());
    if parsed.is_none() {
        violations.push(FieldViolation::new(path, "must be a valid user id"));
    }
    parsed
}

fn parse_instant(
    value: Option<&String>,
    path: &str,
    violations: &mut Vec<FieldViolation>,
) -> Option<DateTime<Utc>> {
    let value = value
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())?;
    match DateTime::parse_from_rfc3339(value) {
        Ok(parsed) => Some(parsed.with_timezone(&Utc)),
        Err(_) => {
            violations.push(FieldViolation::new(
                path,
                "must be an ISO-8601 date and time",
            ));
            None
        }
    }
}

fn parse_members(
    inputs: &[RuleMemberInput],
    violations: &mut Vec<FieldViolation>,
) -> Vec<RuleMember> {
    if inputs.len() > MAX_MEMBERS {
        violations.push(FieldViolation::new(
            "members",
            format!("must have at most {MAX_MEMBERS} members"),
        ));
        return Vec::new();
    }
    let mut seen = HashSet::new();
    let mut members = Vec::with_capacity(inputs.len());
    for (index, input) in inputs.iter().enumerate() {
        let path = format!("members.{index}");
        let Some(user_id) = parse_user(&input.user_id, &format!("{path}.user_id"), violations)
        else {
            continue;
        };
        if !seen.insert(user_id.clone()) {
            violations.push(FieldViolation::new(
                format!("{path}.user_id"),
                "is repeated",
            ));
            continue;
        }
        let weight = input.weight.unwrap_or(MIN_WEIGHT);
        if !(MIN_WEIGHT..=MAX_WEIGHT).contains(&weight) {
            violations.push(FieldViolation::new(
                format!("{path}.weight"),
                format!("must be between {MIN_WEIGHT} and {MAX_WEIGHT}"),
            ));
        }
        let away_from = parse_instant(
            input.away_from.as_ref(),
            &format!("{path}.away_from"),
            violations,
        );
        let away_until = parse_instant(
            input.away_until.as_ref(),
            &format!("{path}.away_until"),
            violations,
        );
        if let (Some(from), Some(until)) = (away_from, away_until)
            && until <= from
        {
            violations.push(FieldViolation::new(
                format!("{path}.away_until"),
                "must be after away_from",
            ));
        }
        members.push(RuleMember {
            user_id,
            weight,
            away: input.away,
            away_from,
            away_until,
        });
    }
    members
}

fn parse_routing(input: &RoutingInput, violations: &mut Vec<FieldViolation>) -> Option<Routing> {
    let field = input.field.trim().to_owned();
    if field.is_empty() {
        violations.push(FieldViolation::new("routing.field", "is required"));
    }
    let fallback = input.fallback.unwrap_or(RoutingFallback::Members);
    if input.routes.is_empty() || input.routes.len() > MAX_ROUTES {
        violations.push(FieldViolation::new(
            "routing.routes",
            format!("must have between 1 and {MAX_ROUTES} routes"),
        ));
        return None;
    }
    let mut routes = Vec::with_capacity(input.routes.len());
    for (index, route) in input.routes.iter().enumerate() {
        let path = format!("routing.routes.{index}");
        let values: Vec<String> = route
            .values
            .iter()
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
            .collect();
        if values.is_empty() || values.len() > MAX_ROUTE_VALUES {
            violations.push(FieldViolation::new(
                format!("{path}.values"),
                format!("must have between 1 and {MAX_ROUTE_VALUES} values"),
            ));
        }
        if values
            .iter()
            .any(|value| value.chars().count() > MAX_ROUTE_VALUE_LENGTH)
        {
            violations.push(FieldViolation::new(
                format!("{path}.values"),
                format!("each value must have at most {MAX_ROUTE_VALUE_LENGTH} characters"),
            ));
        }
        let team_id = route
            .team_id
            .as_deref()
            .map(str::trim)
            .filter(|id| !id.is_empty())
            .and_then(|id| {
                let parsed = Uuid::parse_str(id)
                    .ok()
                    .and_then(|id| TeamId::new(id.to_string()).ok());
                if parsed.is_none() {
                    violations.push(FieldViolation::new(
                        format!("{path}.team_id"),
                        "must be a valid team id",
                    ));
                }
                parsed
            });
        if route.user_ids.len() > MAX_ROUTE_USERS {
            violations.push(FieldViolation::new(
                format!("{path}.user_ids"),
                format!("must have at most {MAX_ROUTE_USERS} users"),
            ));
        }
        let mut user_ids = Vec::new();
        for (user_index, user) in route.user_ids.iter().enumerate() {
            if let Some(user_id) =
                parse_user(user, &format!("{path}.user_ids.{user_index}"), violations)
                && !user_ids.contains(&user_id)
            {
                user_ids.push(user_id);
            }
        }
        if team_id.is_none() && user_ids.is_empty() && route.team_id.is_none() {
            violations.push(FieldViolation::new(
                path.clone(),
                "must name a team or at least one user",
            ));
        }
        routes.push(Route {
            values,
            team_id,
            user_ids,
        });
    }
    Some(Routing {
        field,
        routes,
        fallback,
    })
}

fn optional_text(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn require_read(scope: &AccessScope) -> Result<(), AppError> {
    if allows(scope, &Permission::ASSIGNMENT_MANAGE)
        || allows(scope, &aphel_contract_automation::MANAGE_PERMISSION)
    {
        Ok(())
    } else {
        scope.require(&Permission::ASSIGNMENT_MANAGE).map(|_| ())
    }
}

pub(crate) async fn list(
    ctx: &Ctx<'_>,
    query: RuleListQuery,
    window: ListWindow,
) -> Result<Listing<RuleView>, AppError> {
    require_read(ctx.scope)?;
    let filter = RuleListFilter {
        record_type: optional_text(query.record_type),
        active: query.active,
        window,
    };
    let listing = DistributionStore::of(ctx).list(&filter).await?;
    let users = users_of(ctx, &listing.items).await?;
    let now = ctx.now();
    let can_manage = allows(ctx.scope, &Permission::ASSIGNMENT_MANAGE);
    Ok(Listing {
        items: listing
            .items
            .into_iter()
            .map(|rule| RuleView {
                users: users.clone(),
                rule,
                now,
                can_manage,
            })
            .collect(),
        total: listing.total,
    })
}

pub(crate) async fn list_assignments(
    ctx: &Ctx<'_>,
    query: AssignmentListQuery,
    window: ListWindow,
) -> Result<Listing<AssignmentEntry>, AppError> {
    ctx.scope.require(&Permission::ASSIGNMENT_MANAGE)?;
    let mut violations = Vec::new();
    let rule_id = optional_text(query.rule_id).and_then(|value| {
        let parsed = Uuid::parse_str(&value).ok();
        if parsed.is_none() {
            violations.push(FieldViolation::new("rule_id", "must be a valid rule id"));
        }
        parsed
    });
    let user_id = optional_text(query.user_id)
        .and_then(|value| parse_user(&value, "user_id", &mut violations));
    if !violations.is_empty() {
        return Err(AppError::invalid_fields(
            "invalid assignment filter",
            violations,
        ));
    }
    DistributionStore::of(ctx)
        .assignments(&AssignmentFilter {
            rule_id,
            user_id,
            record_type: optional_text(query.record_type),
            source: query.source,
            window,
        })
        .await
}

pub(crate) async fn get(ctx: &Ctx<'_>, rule_id: &str) -> Result<RuleView, AppError> {
    require_read(ctx.scope)?;
    let rule = find(ctx, rule_id).await?;
    view(ctx, rule).await
}

pub(crate) async fn create(ctx: &Ctx<'_>, command: SaveRuleCommand) -> Result<RuleView, AppError> {
    ctx.scope.require(&Permission::ASSIGNMENT_MANAGE)?;
    let mut violations = Vec::new();
    let name = command.name.clone().map(|name| name.trim().to_owned());
    if name.as_deref().is_none_or(str::is_empty) {
        violations.push(FieldViolation::new("name", "is required"));
    }
    if command
        .record_type
        .as_deref()
        .is_none_or(|value| value.trim().is_empty())
    {
        violations.push(FieldViolation::new("record_type", "is required"));
    }
    if command.strategy.is_none() {
        violations.push(FieldViolation::new("strategy", "is required"));
    }
    if !violations.is_empty() {
        return Err(AppError::invalid_fields(
            "invalid lead distribution rule",
            violations,
        ));
    }
    let now = ctx.now();
    let base = DistributionRule {
        id: Uuid::now_v7(),
        key: String::new(),
        name: String::new(),
        description: None,
        record_type: String::new(),
        strategy: DistributionStrategy::RoundRobin,
        is_active: true,
        members: Vec::new(),
        routing: None,
        created_by: Some(ctx.scope.user_id().clone()),
        created_at: now,
        updated_at: now,
        stats: RuleStats::default(),
    };
    let explicit_key = optional_text(command.key.clone());
    let mut rule = apply_command(ctx, base, &command).await?;
    rule.key = match explicit_key {
        Some(key) => {
            ensure_key(ctx, &key, None).await?;
            key
        }
        None => unique_key(ctx, &rule.name).await?,
    };
    let mut tx = ctx.begin().await?;
    DistributionStore::of(ctx).insert(&mut tx, &rule).await?;
    publish(
        ctx,
        &mut tx,
        &rule,
        RULE_CREATED,
        (&Value::Null, &snapshot(&rule)),
        command.ip,
    )
    .await?;
    tx.commit().await?;
    get(ctx, &rule.id.to_string()).await
}

pub(crate) async fn update(
    ctx: &Ctx<'_>,
    rule_id: &str,
    command: SaveRuleCommand,
) -> Result<RuleView, AppError> {
    ctx.scope.require(&Permission::ASSIGNMENT_MANAGE)?;
    let current = find(ctx, rule_id).await?;
    ensure_version(command.expected_updated_at, current.updated_at)?;
    let mut rule = apply_command(ctx, current.clone(), &command).await?;
    if let Some(key) = optional_text(command.key.clone())
        && key != current.key
    {
        ensure_key(ctx, &key, Some(current.id)).await?;
        rule.key = key;
    }
    let before = snapshot(&current);
    let after = snapshot(&rule);
    if before != after {
        rule.updated_at = ctx.now();
        let mut tx = ctx.begin().await?;
        DistributionStore::of(ctx).update(&mut tx, &rule).await?;
        publish(
            ctx,
            &mut tx,
            &rule,
            RULE_UPDATED,
            (&before, &after),
            command.ip,
        )
        .await?;
        tx.commit().await?;
    }
    get(ctx, &rule.id.to_string()).await
}

pub(crate) async fn delete(
    ctx: &Ctx<'_>,
    rule_id: &str,
    ip: Option<String>,
) -> Result<(), AppError> {
    ctx.scope.require(&Permission::ASSIGNMENT_MANAGE)?;
    let rule = find(ctx, rule_id).await?;
    let mut tx = ctx.begin().await?;
    DistributionStore::of(ctx).delete(&mut tx, rule.id).await?;
    publish(
        ctx,
        &mut tx,
        &rule,
        RULE_DELETED,
        (&snapshot(&rule), &Value::Null),
        ip,
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

pub(crate) async fn simulate(
    ctx: &Ctx<'_>,
    rule_id: &str,
    command: SimulateCommand,
) -> Result<Simulation, AppError> {
    require_read(ctx.scope)?;
    let rule = find(ctx, rule_id).await?;
    let count = command.count.unwrap_or(DEFAULT_SIMULATION);
    if !(1..=MAX_SIMULATION).contains(&count) {
        return Err(AppError::invalid_fields(
            "invalid simulation",
            vec![FieldViolation::new(
                "count",
                format!("must be between 1 and {}", MAX_SIMULATION),
            )],
        ));
    }
    simulate_rule(ctx, &rule, &command.values, count).await
}

async fn find(ctx: &Ctx<'_>, rule_id: &str) -> Result<DistributionRule, AppError> {
    DistributionStore::of(ctx)
        .find(parse_rule_id(rule_id)?)
        .await?
        .ok_or_else(rule_not_found)
}

async fn view(ctx: &Ctx<'_>, rule: DistributionRule) -> Result<RuleView, AppError> {
    let users = users_of(ctx, std::slice::from_ref(&rule)).await?;
    Ok(RuleView {
        rule,
        users,
        now: ctx.now(),
        can_manage: allows(ctx.scope, &Permission::ASSIGNMENT_MANAGE),
    })
}

async fn users_of(
    ctx: &Ctx<'_>,
    rules: &[DistributionRule],
) -> Result<HashMap<UserId, UserStatus>, AppError> {
    let mut ids: Vec<UserId> = Vec::new();
    for rule in rules {
        ids.extend(rule.members.iter().map(|member| member.user_id.clone()));
        if let Some(routing) = &rule.routing {
            for route in &routing.routes {
                ids.extend(route.user_ids.iter().cloned());
            }
        }
    }
    ids.sort();
    ids.dedup();
    if ids.is_empty() {
        return Ok(HashMap::new());
    }
    Ok(DistributionStore::of(ctx)
        .user_statuses(&ids)
        .await?
        .into_iter()
        .map(|status| (status.user_id.clone(), status))
        .collect())
}

async fn apply_command(
    ctx: &Ctx<'_>,
    mut rule: DistributionRule,
    command: &SaveRuleCommand,
) -> Result<DistributionRule, AppError> {
    let mut violations = Vec::new();
    if let Some(name) = &command.name {
        let name = name.trim();
        if name.is_empty() {
            violations.push(FieldViolation::new("name", "is required"));
        } else if name.chars().count() > MAX_NAME_LENGTH {
            violations.push(FieldViolation::new(
                "name",
                format!("must have at most {MAX_NAME_LENGTH} characters"),
            ));
        }
        rule.name = name.to_owned();
    }
    if let Some(description) = &command.description {
        let description = optional_text(description.clone());
        if description
            .as_ref()
            .is_some_and(|text| text.chars().count() > MAX_DESCRIPTION_LENGTH)
        {
            violations.push(FieldViolation::new(
                "description",
                format!("must have at most {MAX_DESCRIPTION_LENGTH} characters"),
            ));
        }
        rule.description = description;
    }
    if let Some(key) = optional_text(command.key.clone())
        && (!is_config_key(&key) || key.len() > MAX_CONFIG_KEY_LENGTH)
    {
        violations.push(FieldViolation::new(
            "key",
            "must start with a lowercase letter and use only lowercase letters, digits and underscores",
        ));
    }
    if let Some(record_type) = &command.record_type {
        rule.record_type = record_type.trim().to_owned();
    }
    if let Some(strategy) = command.strategy {
        rule.strategy = strategy;
    }
    if let Some(active) = command.is_active {
        rule.is_active = active;
    }
    if let Some(members) = &command.members {
        rule.members = parse_members(members, &mut violations);
    }
    if let Some(routing) = &command.routing {
        rule.routing = routing
            .as_ref()
            .and_then(|routing| parse_routing(routing, &mut violations));
    }
    let target = validate_target(ctx, &rule.record_type, &mut violations).await?;
    if let (Some(target), Some(routing)) = (&target, &rule.routing)
        && !routing.field.is_empty()
        && !target.fields.iter().any(|field| field == &routing.field)
    {
        violations.push(FieldViolation::new(
            "routing.field",
            "is not a field of this record type",
        ));
    }
    let needs_members = rule
        .routing
        .as_ref()
        .is_none_or(|routing| routing.fallback == RoutingFallback::Members);
    if needs_members && rule.members.is_empty() {
        violations.push(FieldViolation::new(
            "members",
            "must have at least one member (or routes with fallback none)",
        ));
    }
    check_references(ctx, &rule, &mut violations).await?;
    if violations.is_empty() {
        Ok(rule)
    } else {
        Err(AppError::invalid_fields(
            "invalid lead distribution rule",
            violations,
        ))
    }
}

async fn validate_target(
    ctx: &Ctx<'_>,
    record_type: &str,
    violations: &mut Vec<FieldViolation>,
) -> Result<Option<TargetInfo>, AppError> {
    if !targets(ctx)?
        .iter()
        .any(|target| target.record_type == record_type)
    {
        violations.push(FieldViolation::new(
            "record_type",
            "this record type is unavailable",
        ));
        return Ok(None);
    }
    match store::target(ctx, record_type).await? {
        Some(target) if target.has_owner => Ok(Some(target)),
        Some(_) => {
            violations.push(FieldViolation::new(
                "record_type",
                "this record type has no owner",
            ));
            Ok(None)
        }
        None => {
            violations.push(FieldViolation::new("record_type", "is not available"));
            Ok(None)
        }
    }
}

async fn check_references(
    ctx: &Ctx<'_>,
    rule: &DistributionRule,
    violations: &mut Vec<FieldViolation>,
) -> Result<(), AppError> {
    let mut users: Vec<UserId> = rule
        .members
        .iter()
        .map(|member| member.user_id.clone())
        .collect();
    let mut teams = Vec::new();
    if let Some(routing) = &rule.routing {
        for route in &routing.routes {
            users.extend(route.user_ids.iter().cloned());
            teams.extend(route.team_id.iter().cloned());
        }
    }
    users.sort();
    users.dedup();
    let known: HashSet<UserId> = if users.is_empty() {
        HashSet::new()
    } else {
        DistributionStore::of(ctx)
            .user_statuses(&users)
            .await?
            .into_iter()
            .map(|status| status.user_id)
            .collect()
    };
    for (index, member) in rule.members.iter().enumerate() {
        if !known.contains(&member.user_id) {
            violations.push(FieldViolation::new(
                format!("members.{index}.user_id"),
                "is not a member of the organization",
            ));
        }
    }
    let existing: HashSet<TeamId> = if teams.is_empty() {
        HashSet::new()
    } else {
        DistributionStore::of(ctx)
            .existing_teams(&teams)
            .await?
            .into_iter()
            .collect()
    };
    if let Some(routing) = &rule.routing {
        for (index, route) in routing.routes.iter().enumerate() {
            if let Some(team_id) = &route.team_id
                && !existing.contains(team_id)
            {
                violations.push(FieldViolation::new(
                    format!("routing.routes.{index}.team_id"),
                    "was not found",
                ));
            }
            for (user_index, user_id) in route.user_ids.iter().enumerate() {
                if !known.contains(user_id) {
                    violations.push(FieldViolation::new(
                        format!("routing.routes.{index}.user_ids.{user_index}"),
                        "is not a member of the organization",
                    ));
                }
            }
        }
    }
    Ok(())
}

async fn ensure_key(ctx: &Ctx<'_>, key: &str, except: Option<Uuid>) -> Result<(), AppError> {
    if DistributionStore::of(ctx).key_taken(key, except).await? {
        Err(AppError::conflict_code(
            RULE_KEY_TAKEN,
            "another lead distribution rule already uses this key",
        ))
    } else {
        Ok(())
    }
}

async fn unique_key(ctx: &Ctx<'_>, name: &str) -> Result<String, AppError> {
    let base: String = key_from_name(name)
        .chars()
        .take(MAX_CONFIG_KEY_LENGTH - 4)
        .collect();
    let mut candidate = base.clone();
    let mut suffix = 2;
    while DistributionStore::of(ctx)
        .key_taken(&candidate, None)
        .await?
    {
        candidate = format!("{base}_{suffix}");
        suffix += 1;
    }
    Ok(candidate)
}

async fn publish(
    ctx: &Ctx<'_>,
    tx: &mut Tx,
    rule: &DistributionRule,
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
            record: RecordRef::new(RULE_RECORD_TYPE, rule.id),
            event_type,
            payload: json!({
                "key": rule.key,
                "name": rule.name,
                "record_type": rule.record_type,
                "strategy": rule.strategy.as_str(),
                "is_active": rule.is_active,
            }),
            changes,
            ip,
        },
    )
    .await
}

#[derive(serde::Serialize, utoipa::ToSchema)]
pub(crate) struct AssignmentTarget {
    pub record_type: String,
    pub label: String,
    pub labels: std::collections::BTreeMap<String, String>,
}
pub(crate) fn targets(ctx: &Ctx<'_>) -> Result<Vec<AssignmentTarget>, AppError> {
    ctx.scope.require(&Permission::ASSIGNMENT_MANAGE)?;
    Ok(ctx
        .registry()
        .definitions()
        .filter(|definition| definition.owner_column.is_some())
        .filter(|definition| {
            let app = aphel_sdk::platform::modules::app();
            app.catalog()
                .permission_owner(&definition.read_permission)
                .is_none_or(|owner| app.is_enabled(owner))
        })
        .map(|definition| {
            let labels = definition
                .capabilities
                .plural_labels_value()
                .unwrap_or_default();
            let label = definition
                .custom
                .as_ref()
                .map(|custom| custom.plural.clone())
                .or_else(|| (!labels.0.is_empty()).then(|| labels.default_text().to_owned()))
                .unwrap_or_else(|| definition.record_type.as_str().to_owned());
            AssignmentTarget {
                record_type: definition.record_type.as_str().to_owned(),
                label,
                labels: labels.0,
            }
        })
        .collect())
}
